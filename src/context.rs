//! Execution context for the scripting engine.
//!
//! The Context owns a [`SymbolRegistry`] that stores all registered types and functions.
//! Users install modules into the context, then create compilation units from it.

use std::sync::Arc;
use thiserror::Error;

use angelscript_core::{
    ClassEntry, ClassMeta, DataType, FuncdefEntry, FuncdefMeta, FunctionDef, FunctionEntry,
    FunctionMeta, FunctionTraits, InterfaceEntry, InterfaceMeta, MethodSignature, Param,
    PropertyEntry, StringFactory, TemplateParamEntry, TypeHash, TypeSource, Visibility,
};
use angelscript_registry::{Module, SymbolRegistry};

use crate::unit::Unit;

/// Execution context that owns the type registry.
///
/// The context is the central owner of all type and function registrations.
/// Install modules to register FFI types, then create compilation units to
/// compile and run scripts.
///
/// # Example
///
/// ```ignore
/// use angelscript::{Context, Module};
/// use std::sync::Arc;
///
/// let mut ctx = Context::new();
/// ctx.install(Module::new().class::<MyClass>())?;
///
/// let ctx = Arc::new(ctx);
/// let unit = ctx.create_unit()?;
/// ```
pub struct Context {
    registry: SymbolRegistry,
    /// The string factory for creating string literal values.
    /// If None, string literals will produce a compile error.
    string_factory: Option<Box<dyn StringFactory>>,
}

impl Context {
    /// Create a new context with primitives pre-registered.
    pub fn new() -> Self {
        Self {
            registry: SymbolRegistry::with_primitives(),
            string_factory: None,
        }
    }

    /// Create a context with default modules pre-installed.
    ///
    /// This registers the standard library types (string, array, dictionary, etc.)
    /// in addition to primitives. Also sets the default string factory for
    /// string literals.
    pub fn with_default_modules() -> Result<Self, ContextError> {
        let mut ctx = Self::new();

        // Install stdlib modules
        ctx.install(angelscript_modules::string::module())?;
        ctx.install(angelscript_modules::array::module())?;
        ctx.install(angelscript_modules::dictionary::module())?;
        ctx.install(angelscript_modules::math::module())?;
        ctx.install(angelscript_modules::std::module())?;

        // Set default string factory
        ctx.set_string_factory(Box::new(angelscript_modules::string::ScriptStringFactory));

        Ok(ctx)
    }

    /// Install a module into the context.
    ///
    /// This registers all types and functions from the module into the
    /// context's type registry.
    ///
    /// # Errors
    ///
    /// Returns an error if registration fails (e.g., duplicate type names).
    pub fn install(&mut self, module: Module) -> Result<(), ContextError> {
        // Compute qualified namespace string once (only for registry operations that need it)
        let qualified_ns = if module.namespace.is_empty() {
            String::new()
        } else {
            module.namespace.join("::")
        };

        // Register namespace if non-empty
        if !qualified_ns.is_empty() {
            self.registry.register_namespace(&qualified_ns);
        }

        // Install classes
        for class_meta in module.classes {
            self.install_class(&module.namespace, &qualified_ns, class_meta)?;
        }

        // Install functions - pass associated_type for methods, None for globals
        for func_meta in module.functions {
            self.install_function(&module.namespace, func_meta.associated_type, func_meta)?;
        }

        // Install interfaces
        for interface_meta in module.interfaces {
            self.install_interface(&module.namespace, &qualified_ns, interface_meta)?;
        }

        // Install funcdefs
        for funcdef_meta in module.funcdefs {
            self.install_funcdef(&module.namespace, &qualified_ns, funcdef_meta)?;
        }

        Ok(())
    }

    /// Get a reference to the type registry.
    pub fn registry(&self) -> &SymbolRegistry {
        &self.registry
    }

    /// Set a custom string factory.
    ///
    /// The string factory creates string values from raw byte data when
    /// the VM loads string constants. This allows custom string implementations
    /// (interned strings, OsString, etc.).
    ///
    /// # Example
    ///
    /// ```ignore
    /// use angelscript_modules::string::ScriptStringFactory;
    ///
    /// ctx.set_string_factory(Box::new(ScriptStringFactory));
    /// ```
    pub fn set_string_factory(&mut self, factory: Box<dyn StringFactory>) {
        self.string_factory = Some(factory);
    }

    /// Get the string factory (for compiler/VM use).
    ///
    /// Returns `None` if no string factory has been configured.
    /// Use `with_default_modules()` to get a context with the default
    /// `ScriptStringFactory` already set.
    pub fn string_factory(&self) -> Option<&dyn StringFactory> {
        self.string_factory.as_deref()
    }

    /// Create a new compilation unit from this context.
    pub fn create_unit(self: &Arc<Self>) -> Result<Unit, ContextError> {
        Ok(Unit::with_context(Arc::clone(self)))
    }

    // =========================================================================
    // Private installation helpers
    // =========================================================================

    fn install_class(
        &mut self,
        namespace: &[String],
        qualified_ns: &str,
        meta: ClassMeta,
    ) -> Result<(), ContextError> {
        let qualified_name = if qualified_ns.is_empty() {
            meta.name.to_string()
        } else {
            format!("{}::{}", qualified_ns, meta.name)
        };

        // Create the class entry with rust_type_id for safe downcasting
        let mut class_entry = ClassEntry::new(
            meta.name,
            namespace.to_vec(),
            &qualified_name,
            meta.type_hash,
            meta.type_kind,
            TypeSource::ffi_with_type_id(meta.rust_type_id),
        );

        // Register template parameters as TemplateParamEntry and collect their hashes
        if !meta.template_params.is_empty() {
            let mut template_param_hashes = Vec::with_capacity(meta.template_params.len());
            for (index, param_name) in meta.template_params.iter().enumerate() {
                let param_entry = TemplateParamEntry::for_template(
                    *param_name,
                    index,
                    meta.type_hash,
                    &qualified_name,
                );
                let param_hash = param_entry.type_hash;
                template_param_hashes.push(param_hash);

                // Register the TemplateParamEntry in the registry
                self.registry
                    .register_type(param_entry.into())
                    .map_err(|e| ContextError::RegistrationFailed(e.to_string()))?;
            }
            class_entry = class_entry.with_template_params(template_param_hashes);
        }

        // Add template specialization info
        if let Some(base_name) = meta.specialization_of {
            let template_hash = TypeHash::from_name(base_name);
            let type_args: Vec<DataType> = meta
                .specialization_args
                .iter()
                .map(|&h| DataType::simple(h))
                .collect();
            class_entry = class_entry.with_template_instance(template_hash, type_args);
        }

        // Convert properties
        for prop in meta.properties {
            let data_type = DataType::simple(prop.type_hash);
            // Generate getter/setter function hashes based on property name
            let getter_hash = if prop.get {
                Some(TypeHash::from_name(&format!(
                    "{}::get_{}",
                    meta.name, prop.name
                )))
            } else {
                None
            };
            let setter_hash = if prop.set {
                Some(TypeHash::from_name(&format!(
                    "{}::set_{}",
                    meta.name, prop.name
                )))
            } else {
                None
            };
            let prop_entry = PropertyEntry::new(
                prop.name,
                data_type,
                Visibility::Public,
                getter_hash,
                setter_hash,
            );
            class_entry = class_entry.with_property(prop_entry);
        }

        // Register the class
        self.registry
            .register_type(class_entry.into())
            .map_err(|e| ContextError::RegistrationFailed(e.to_string()))?;

        Ok(())
    }

    fn install_function(
        &mut self,
        namespace: &[String],
        object_type: Option<TypeHash>,
        meta: FunctionMeta,
    ) -> Result<(), ContextError> {
        let name = meta.as_name.unwrap_or(meta.name);

        // Get owner class qualified name for resolving template param names
        let owner_qualified_name = object_type.and_then(|owner| {
            self.registry
                .get(owner)
                .and_then(|e| e.as_class())
                .map(|c| c.qualified_name.clone())
        });

        // Helper to resolve template param name to type hash
        let resolve_template_param = |template_param: Option<&str>, default_hash: TypeHash| {
            if let Some(param_name) = template_param {
                if let Some(ref qualified_name) = owner_qualified_name {
                    // Compute hash as "qualified_name::param_name" (e.g., "dictionary::K")
                    TypeHash::from_name(&format!("{}::{}", qualified_name, param_name))
                } else {
                    default_hash
                }
            } else {
                default_hash
            }
        };

        // For generic calling convention, use generic_params; otherwise use params
        // Variadic parameters are excluded from the function hash but included in params
        let (param_hashes, params, is_variadic) = if meta.is_generic {
            // Hash excludes variadic params (they don't affect signature)
            let hashes: Vec<TypeHash> = meta
                .generic_params
                .iter()
                .filter(|p| !p.is_variadic)
                .map(|p| p.type_hash)
                .collect();

            // Params includes all params (variadic last, for type checking extra args)
            let params: Vec<Param> = meta
                .generic_params
                .iter()
                .map(|p| {
                    // Create DataType with appropriate ref_modifier from metadata
                    let mut data_type = match p.ref_mode {
                        angelscript_core::RefModifier::None => DataType::simple(p.type_hash),
                        angelscript_core::RefModifier::In => DataType::with_ref_in(p.type_hash),
                        angelscript_core::RefModifier::Out => DataType::with_ref_out(p.type_hash),
                        angelscript_core::RefModifier::InOut => {
                            DataType::with_ref_inout(p.type_hash)
                        }
                    };
                    // Apply const if specified
                    if p.is_const {
                        data_type = data_type.as_const();
                    }
                    // Variadic param should have default (0 extra args is valid)
                    let param = if p.default_value.is_some() || p.is_variadic {
                        Param::with_default("", data_type)
                    } else {
                        Param::new("", data_type)
                    };
                    if p.if_handle_then_const {
                        param.with_if_handle_then_const(true)
                    } else {
                        param
                    }
                })
                .collect();

            let is_variadic = meta.generic_params.iter().any(|p| p.is_variadic);
            (hashes, params, is_variadic)
        } else {
            let hashes: Vec<TypeHash> = meta
                .params
                .iter()
                .map(|p| resolve_template_param(p.template_param, p.type_hash))
                .collect();

            let params: Vec<Param> = meta
                .params
                .iter()
                .map(|p| {
                    let type_hash = resolve_template_param(p.template_param, p.type_hash);
                    // Create DataType with appropriate ref_modifier from metadata
                    let mut data_type = match p.ref_mode {
                        angelscript_core::RefModifier::None => DataType::simple(type_hash),
                        angelscript_core::RefModifier::In => DataType::with_ref_in(type_hash),
                        angelscript_core::RefModifier::Out => DataType::with_ref_out(type_hash),
                        angelscript_core::RefModifier::InOut => DataType::with_ref_inout(type_hash),
                    };
                    // Apply const if specified (e.g., Rust &T becomes const T &in)
                    if p.is_const {
                        data_type = data_type.as_const();
                    }
                    let param = if p.default_value.is_some() {
                        Param::with_default(p.name, data_type)
                    } else {
                        Param::new(p.name, data_type)
                    };
                    if p.if_handle_then_const {
                        param.with_if_handle_then_const(true)
                    } else {
                        param
                    }
                })
                .collect();

            (hashes, params, false)
        };

        // Compute function hash - use from_method for methods, from_function for globals
        let func_hash = if let Some(owner) = object_type {
            TypeHash::from_method(owner, name, &param_hashes)
        } else {
            TypeHash::from_function(name, &param_hashes)
        };

        // Determine return type (resolve template param if specified)
        let return_type = if let Some(type_hash) = meta.return_meta.type_hash {
            let resolved_hash = resolve_template_param(meta.return_meta.template_param, type_hash);
            DataType::simple(resolved_hash)
        } else {
            DataType::void()
        };

        // Build function traits
        let (is_constructor, is_destructor) = match &meta.behavior {
            Some(angelscript_core::Behavior::Constructor) => (true, false),
            Some(angelscript_core::Behavior::Destructor) => (false, true),
            _ => (false, false),
        };
        let traits = FunctionTraits {
            is_const: meta.is_const,
            is_constructor,
            is_destructor,
            ..Default::default()
        };

        let def = if meta.template_params.is_empty() {
            FunctionDef::new(
                func_hash,
                name.to_string(),
                namespace.to_vec(),
                params,
                return_type,
                object_type,
                traits,
                true, // is_native
                Visibility::Public,
            )
        } else {
            // Compute qualified function name for template param naming
            let qualified_func_name = if namespace.is_empty() {
                name.to_string()
            } else {
                format!("{}::{}", namespace.join("::"), name)
            };

            // Register template parameters as TemplateParamEntry and collect their hashes
            let mut template_param_hashes = Vec::with_capacity(meta.template_params.len());
            for (index, param_name) in meta.template_params.iter().enumerate() {
                let param_entry = TemplateParamEntry::for_template(
                    *param_name,
                    index,
                    func_hash,
                    &qualified_func_name,
                );
                let param_hash = param_entry.type_hash;
                template_param_hashes.push(param_hash);

                // Register the TemplateParamEntry in the registry
                self.registry
                    .register_type(param_entry.into())
                    .map_err(|e| ContextError::RegistrationFailed(e.to_string()))?;
            }

            FunctionDef::new_template(
                func_hash,
                name.to_string(),
                namespace.to_vec(),
                params,
                return_type,
                object_type,
                traits,
                true, // is_native
                Visibility::Public,
                template_param_hashes,
            )
        };

        // Set variadic flag if this function accepts variadic arguments
        let mut def = def;
        def.is_variadic = is_variadic;

        // Create function entry - use native_fn if available
        let entry = match meta.native_fn {
            Some(native_fn) => FunctionEntry::ffi_with_native(def, native_fn),
            None => FunctionEntry::ffi(def),
        };

        self.registry
            .register_function(entry)
            .map_err(|e| ContextError::RegistrationFailed(e.to_string()))?;

        // Add method to the class's methods map (for method lookup during compilation)
        if let Some(type_hash) = object_type
            && let Some(class) = self
                .registry
                .get_mut(type_hash)
                .and_then(|e| e.as_class_mut())
        {
            class.add_method(name, func_hash);
        }

        // Wire behavior to the type's behaviors if this function has an associated behavior
        if let (Some(type_hash), Some(behavior)) = (object_type, &meta.behavior) {
            self.wire_behavior(type_hash, func_hash, behavior, meta.list_pattern.as_ref())?;
        }

        Ok(())
    }

    /// Wire a function's behavior to the type's TypeBehaviors.
    fn wire_behavior(
        &mut self,
        type_hash: TypeHash,
        func_hash: TypeHash,
        behavior: &angelscript_core::Behavior,
        list_pattern: Option<&angelscript_core::meta::ListPatternMeta>,
    ) -> Result<(), ContextError> {
        use angelscript_core::{Behavior, ListBehavior, ListPattern};

        // Get the type entry and modify its behaviors
        let type_entry = self.registry.get_mut(type_hash).ok_or_else(|| {
            ContextError::RegistrationFailed(format!(
                "type {:?} not found when wiring behavior",
                type_hash
            ))
        })?;

        // Only ClassEntry has behaviors
        let class_entry = type_entry.as_class_mut().ok_or_else(|| {
            ContextError::RegistrationFailed(format!(
                "type {:?} is not a class, cannot wire behavior",
                type_hash
            ))
        })?;

        // Wire the behavior based on its type
        match behavior {
            Behavior::Constructor | Behavior::CopyConstructor => {
                class_entry.behaviors.add_constructor(func_hash);
            }
            Behavior::Factory => {
                class_entry.behaviors.add_factory(func_hash);
            }
            Behavior::Destructor => {
                class_entry.behaviors.set_destructor(func_hash);
            }
            Behavior::AddRef => {
                // AddRef needs Release to be complete - store for later if needed
                if let Some(release) = class_entry.behaviors.release {
                    class_entry.behaviors.set_ref_counting(func_hash, release);
                }
                // Store addref for when release comes
                class_entry.behaviors.addref = Some(func_hash);
            }
            Behavior::Release => {
                // Release needs AddRef to be complete
                if let Some(addref) = class_entry.behaviors.addref {
                    class_entry.behaviors.set_ref_counting(addref, func_hash);
                }
                // Store release for when addref comes
                class_entry.behaviors.release = Some(func_hash);
            }
            Behavior::ListConstruct => {
                let pattern = list_pattern
                    .map(|p| ListPattern::from(p.clone()))
                    .ok_or_else(|| {
                        ContextError::RegistrationFailed(format!(
                            "list_construct behavior requires a list_pattern for function {:?}",
                            func_hash
                        ))
                    })?;
                class_entry
                    .behaviors
                    .add_list_construct(ListBehavior::new(func_hash, pattern));
            }
            Behavior::ListFactory => {
                let pattern = list_pattern
                    .map(|p| ListPattern::from(p.clone()))
                    .ok_or_else(|| {
                        ContextError::RegistrationFailed(format!(
                            "list_factory behavior requires a list_pattern for function {:?}",
                            func_hash
                        ))
                    })?;
                class_entry
                    .behaviors
                    .add_list_factory(ListBehavior::new(func_hash, pattern));
            }
            Behavior::TemplateCallback => {
                class_entry.behaviors.template_callback = Some(func_hash);
            }
            Behavior::Operator(op) => {
                class_entry.behaviors.add_operator(*op, func_hash);
            }
            Behavior::GetWeakRefFlag => {
                class_entry.behaviors.get_weakref_flag = Some(func_hash);
            }
            // GC behaviors - not yet supported in TypeBehaviors
            Behavior::GcGetRefCount
            | Behavior::GcSetFlag
            | Behavior::GcGetFlag
            | Behavior::GcEnumRefs
            | Behavior::GcReleaseRefs => {
                // TODO: Add GC behavior fields to TypeBehaviors when needed
            }
        }

        Ok(())
    }

    fn install_interface(
        &mut self,
        namespace: &[String],
        qualified_ns: &str,
        meta: InterfaceMeta,
    ) -> Result<(), ContextError> {
        let qualified_name = if qualified_ns.is_empty() {
            meta.name.to_string()
        } else {
            format!("{}::{}", qualified_ns, meta.name)
        };

        let mut entry = InterfaceEntry::new(
            meta.name,
            namespace.to_vec(),
            &qualified_name,
            meta.type_hash,
            TypeSource::ffi_untyped(),
        );

        // Convert methods
        for method in meta.methods {
            let params: Vec<DataType> = method
                .param_types
                .iter()
                .map(|&h| DataType::simple(h))
                .collect();
            let return_type = DataType::simple(method.return_type);

            let sig = if method.is_const {
                MethodSignature::new_const(method.name, params, return_type)
            } else {
                MethodSignature::new(method.name, params, return_type)
            };

            entry = entry.with_method(sig);
        }

        self.registry
            .register_type(entry.into())
            .map_err(|e| ContextError::RegistrationFailed(e.to_string()))?;

        Ok(())
    }

    fn install_funcdef(
        &mut self,
        namespace: &[String],
        qualified_ns: &str,
        meta: FuncdefMeta,
    ) -> Result<(), ContextError> {
        let qualified_name = if qualified_ns.is_empty() {
            meta.name.to_string()
        } else {
            format!("{}::{}", qualified_ns, meta.name)
        };

        let params: Vec<DataType> = meta
            .param_types
            .iter()
            .map(|&h| DataType::simple(h))
            .collect();
        let return_type = DataType::simple(meta.return_type);

        let entry = if let Some(parent_hash) = meta.parent_type {
            FuncdefEntry::new_child(
                meta.name,
                namespace.to_vec(),
                &qualified_name,
                meta.type_hash,
                TypeSource::ffi_untyped(),
                params,
                return_type,
                parent_hash,
            )
        } else {
            FuncdefEntry::new(
                meta.name,
                namespace.to_vec(),
                &qualified_name,
                meta.type_hash,
                TypeSource::ffi_untyped(),
                params,
                return_type,
            )
        };

        self.registry
            .register_type(entry.into())
            .map_err(|e| ContextError::RegistrationFailed(e.to_string()))?;

        Ok(())
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

/// Errors that can occur during context operations.
#[derive(Debug, Error)]
pub enum ContextError {
    /// Registration failed
    #[error("registration failed: {0}")]
    RegistrationFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{TypeKind, primitives};

    #[test]
    fn context_new() {
        let ctx = Context::new();
        // Should have primitives registered
        assert!(ctx.registry().get(primitives::INT32).is_some());
    }

    #[test]
    fn context_default() {
        let ctx = Context::default();
        assert!(ctx.registry().get(primitives::FLOAT).is_some());
    }

    #[test]
    fn context_with_default_modules() {
        let ctx = Context::with_default_modules().unwrap();
        assert!(ctx.registry().get(primitives::BOOL).is_some());
    }

    #[test]
    fn context_create_unit() {
        let ctx = Arc::new(Context::new());
        let _unit = ctx.create_unit().unwrap();
    }

    #[test]
    fn context_install_empty_module() {
        let mut ctx = Context::new();
        let module = Module::new();
        ctx.install(module).unwrap();
    }

    #[test]
    fn context_install_module_with_class() {
        let mut ctx = Context::new();

        let mut module = Module::new();
        module.classes.push(ClassMeta {
            name: "Player",
            type_hash: TypeHash::from_name("Player"),
            type_kind: TypeKind::reference(),
            rust_type_id: None,
            properties: vec![],
            template_params: vec![],
            specialization_of: None,
            specialization_args: vec![],
        });
        ctx.install(module).unwrap();

        assert!(ctx.registry().get(TypeHash::from_name("Player")).is_some());
    }

    #[test]
    fn context_install_module_with_namespace() {
        let mut ctx = Context::new();

        let mut module = Module::in_namespace(&["Game"]);
        module.classes.push(ClassMeta {
            name: "Entity",
            type_hash: TypeHash::from_name("Game::Entity"),
            type_kind: TypeKind::reference(),
            rust_type_id: None,
            properties: vec![],
            template_params: vec![],
            specialization_of: None,
            specialization_args: vec![],
        });
        ctx.install(module).unwrap();

        assert!(ctx.registry().has_namespace("Game"));
        assert!(
            ctx.registry()
                .get(TypeHash::from_name("Game::Entity"))
                .is_some()
        );
    }

    #[test]
    fn context_install_class_sets_namespace_field() {
        let mut ctx = Context::new();

        let mut module = Module::in_namespace(&["Game", "Entities"]);
        module.classes.push(ClassMeta {
            name: "Player",
            type_hash: TypeHash::from_name("Game::Entities::Player"),
            type_kind: TypeKind::reference(),
            rust_type_id: None,
            properties: vec![],
            template_params: vec![],
            specialization_of: None,
            specialization_args: vec![],
        });
        ctx.install(module).unwrap();

        let entry = ctx
            .registry()
            .get(TypeHash::from_name("Game::Entities::Player"))
            .unwrap();
        let class = entry.as_class().unwrap();
        assert_eq!(
            class.namespace,
            vec!["Game".to_string(), "Entities".to_string()]
        );
        assert_eq!(class.qualified_name, "Game::Entities::Player");
    }

    #[test]
    fn context_install_class_global_namespace() {
        let mut ctx = Context::new();

        let mut module = Module::new();
        module.classes.push(ClassMeta {
            name: "Singleton",
            type_hash: TypeHash::from_name("Singleton"),
            type_kind: TypeKind::reference(),
            rust_type_id: None,
            properties: vec![],
            template_params: vec![],
            specialization_of: None,
            specialization_args: vec![],
        });
        ctx.install(module).unwrap();

        let entry = ctx
            .registry()
            .get(TypeHash::from_name("Singleton"))
            .unwrap();
        let class = entry.as_class().unwrap();
        assert!(class.namespace.is_empty());
        assert_eq!(class.qualified_name, "Singleton");
    }

    #[test]
    fn context_install_interface_sets_namespace_field() {
        let mut ctx = Context::new();

        let mut module = Module::in_namespace(&["Game"]);
        module.interfaces.push(InterfaceMeta {
            name: "IDrawable",
            type_hash: TypeHash::from_name("Game::IDrawable"),
            methods: vec![],
        });
        ctx.install(module).unwrap();

        let entry = ctx
            .registry()
            .get(TypeHash::from_name("Game::IDrawable"))
            .unwrap();
        let interface = entry.as_interface().unwrap();
        assert_eq!(interface.namespace, vec!["Game".to_string()]);
        assert_eq!(interface.qualified_name, "Game::IDrawable");
    }

    #[test]
    fn context_install_funcdef_sets_namespace_field() {
        let mut ctx = Context::new();

        let mut module = Module::in_namespace(&["Events"]);
        module.funcdefs.push(FuncdefMeta {
            name: "EventCallback",
            type_hash: TypeHash::from_name("Events::EventCallback"),
            param_types: vec![primitives::INT32],
            return_type: primitives::VOID,
            parent_type: None,
        });
        ctx.install(module).unwrap();

        let entry = ctx
            .registry()
            .get(TypeHash::from_name("Events::EventCallback"))
            .unwrap();
        let funcdef = entry.as_funcdef().unwrap();
        assert_eq!(funcdef.namespace, vec!["Events".to_string()]);
        assert_eq!(funcdef.qualified_name, "Events::EventCallback");
    }

    #[test]
    fn context_namespace_index_populated() {
        let mut ctx = Context::new();

        let mut module = Module::in_namespace(&["Game"]);
        module.classes.push(ClassMeta {
            name: "Player",
            type_hash: TypeHash::from_name("Game::Player"),
            type_kind: TypeKind::reference(),
            rust_type_id: None,
            properties: vec![],
            template_params: vec![],
            specialization_of: None,
            specialization_args: vec![],
        });
        ctx.install(module).unwrap();

        // Verify namespace index is populated
        let types = ctx.registry().get_namespace_types("Game");
        assert!(types.is_some());
        assert!(types.unwrap().get("Player").is_some());
    }

    #[test]
    fn context_install_duplicate_type_fails() {
        let mut ctx = Context::new();

        let class_meta = ClassMeta {
            name: "Player",
            type_hash: TypeHash::from_name("Player"),
            type_kind: TypeKind::reference(),
            rust_type_id: None,
            properties: vec![],
            template_params: vec![],
            specialization_of: None,
            specialization_args: vec![],
        };

        let mut module1 = Module::new();
        module1.classes.push(class_meta.clone());
        let mut module2 = Module::new();
        module2.classes.push(class_meta);

        ctx.install(module1).unwrap();
        let result = ctx.install(module2);
        assert!(result.is_err());
    }

    #[test]
    fn context_install_template_class_registers_params() {
        let mut ctx = Context::new();

        let mut module = Module::new();
        module.classes.push(ClassMeta {
            name: "array",
            type_hash: TypeHash::from_name("array"),
            type_kind: TypeKind::reference(),
            rust_type_id: None,
            properties: vec![],
            template_params: vec!["T"],
            specialization_of: None,
            specialization_args: vec![],
        });
        ctx.install(module).unwrap();

        // Verify the class was registered
        let class_entry = ctx.registry().get(TypeHash::from_name("array"));
        assert!(class_entry.is_some());

        // Verify the template param entry was registered
        let t_param = ctx.registry().get(TypeHash::from_name("array::T"));
        assert!(
            t_param.is_some(),
            "TemplateParamEntry for 'T' should be registered"
        );
        assert!(t_param.unwrap().as_template_param().is_some());
    }

    #[test]
    fn context_install_template_class_multi_params() {
        let mut ctx = Context::new();

        let mut module = Module::new();
        module.classes.push(ClassMeta {
            name: "dict",
            type_hash: TypeHash::from_name("dict"),
            type_kind: TypeKind::reference(),
            rust_type_id: None,
            properties: vec![],
            template_params: vec!["K", "V"],
            specialization_of: None,
            specialization_args: vec![],
        });
        ctx.install(module).unwrap();

        // Verify both template params were registered
        let k_param = ctx.registry().get(TypeHash::from_name("dict::K"));
        let v_param = ctx.registry().get(TypeHash::from_name("dict::V"));
        assert!(
            k_param.is_some(),
            "TemplateParamEntry for 'K' should be registered"
        );
        assert!(
            v_param.is_some(),
            "TemplateParamEntry for 'V' should be registered"
        );

        // Verify the class entry has the correct template_params hashes
        let class_entry = ctx.registry().get(TypeHash::from_name("dict")).unwrap();
        let class = class_entry.as_class().unwrap();
        assert_eq!(class.template_params.len(), 2);
        assert_eq!(class.template_params[0], TypeHash::from_name("dict::K"));
        assert_eq!(class.template_params[1], TypeHash::from_name("dict::V"));
    }

    #[test]
    fn context_install_template_function_registers_params() {
        use angelscript_core::ReturnMeta;

        let mut ctx = Context::new();

        let mut module = Module::new();
        module.functions.push(FunctionMeta {
            name: "identity",
            as_name: None,
            native_fn: None,
            params: vec![],
            generic_params: vec![],
            return_meta: ReturnMeta::default(),
            is_method: false,
            associated_type: None,
            behavior: None,
            is_const: false,
            is_property: false,
            property_name: None,
            is_generic: true,
            list_pattern: None,
            template_params: vec!["T"],
        });
        ctx.install(module).unwrap();

        // Verify the template param entry was registered
        let t_param = ctx.registry().get(TypeHash::from_name("identity::T"));
        assert!(
            t_param.is_some(),
            "TemplateParamEntry for function 'T' should be registered"
        );
        assert!(t_param.unwrap().as_template_param().is_some());
    }

    #[test]
    fn context_install_namespaced_template_class() {
        let mut ctx = Context::new();

        let mut module = Module::in_namespace(&["std"]);
        module.classes.push(ClassMeta {
            name: "vector",
            type_hash: TypeHash::from_name("std::vector"),
            type_kind: TypeKind::reference(),
            rust_type_id: None,
            properties: vec![],
            template_params: vec!["T"],
            specialization_of: None,
            specialization_args: vec![],
        });
        ctx.install(module).unwrap();

        // Verify the template param uses the qualified name
        let t_param = ctx.registry().get(TypeHash::from_name("std::vector::T"));
        assert!(
            t_param.is_some(),
            "TemplateParamEntry should use qualified name 'std::vector::T'"
        );
    }

    #[test]
    fn context_string_factory_not_set() {
        let ctx = Context::new();
        assert!(ctx.string_factory().is_none());
    }

    #[test]
    fn context_string_factory_set_by_default_modules() {
        use angelscript_modules::string::ScriptString;

        let ctx = Context::with_default_modules().unwrap();
        let factory = ctx.string_factory().expect("should have factory");
        assert_eq!(
            factory.type_hash(),
            <ScriptString as angelscript_core::Any>::type_hash()
        );
    }

    #[test]
    fn context_custom_string_factory() {
        use angelscript_modules::string::ScriptStringFactory;

        let mut ctx = Context::new();
        ctx.set_string_factory(Box::new(ScriptStringFactory));

        let factory = ctx.string_factory().unwrap();
        assert_eq!(factory.type_hash(), TypeHash::from_name("string"));
    }

    #[test]
    fn context_install_generic_function_with_params() {
        use angelscript_core::{GenericParamMeta, RefModifier, ReturnMeta};

        let mut ctx = Context::new();

        // Generic function with one required param (like print with format string)
        let mut module = Module::new();
        module.functions.push(FunctionMeta {
            name: "print",
            as_name: None,
            native_fn: None,
            params: vec![], // Normal params are empty for generic functions
            generic_params: vec![GenericParamMeta {
                type_hash: primitives::STRING,
                ref_mode: RefModifier::In,
                is_variadic: false,
                default_value: None,
                if_handle_then_const: false,
                is_const: false,
            }],
            return_meta: ReturnMeta::default(),
            is_method: false,
            associated_type: None,
            behavior: None,
            is_const: false,
            is_property: false,
            property_name: None,
            is_generic: true,
            list_pattern: None,
            template_params: vec![],
        });
        ctx.install(module).unwrap();

        // Verify the function was registered with the correct params
        let func_hash = TypeHash::from_function("print", &[primitives::STRING]);
        let func_entry = ctx.registry().get_function(func_hash);
        assert!(func_entry.is_some(), "print function should be registered");

        let func = func_entry.unwrap();
        assert_eq!(
            func.def.params.len(),
            1,
            "should have 1 param from generic_params"
        );
        assert_eq!(func.def.params[0].data_type.type_hash, primitives::STRING);
    }

    #[test]
    fn context_install_generic_function_variadic() {
        use angelscript_core::{GenericParamMeta, RefModifier, ReturnMeta};

        let mut ctx = Context::new();

        // Generic variadic function: print(string format, ...)
        let mut module = Module::new();
        module.functions.push(FunctionMeta {
            name: "print",
            as_name: None,
            native_fn: None,
            params: vec![],
            generic_params: vec![
                GenericParamMeta {
                    type_hash: primitives::STRING,
                    ref_mode: RefModifier::In,
                    is_variadic: false,
                    default_value: None,
                    if_handle_then_const: false,
                    is_const: false,
                },
                GenericParamMeta {
                    type_hash: primitives::VARIABLE_PARAM,
                    ref_mode: RefModifier::In,
                    is_variadic: true,
                    default_value: None,
                    if_handle_then_const: false,
                    is_const: false,
                },
            ],
            return_meta: ReturnMeta::default(),
            is_method: false,
            associated_type: None,
            behavior: None,
            is_const: false,
            is_property: false,
            property_name: None,
            is_generic: true,
            list_pattern: None,
            template_params: vec![],
        });
        ctx.install(module).unwrap();

        // Function hash is computed WITHOUT variadic params
        let func_hash = TypeHash::from_function("print", &[primitives::STRING]);
        let func_entry = ctx.registry().get_function(func_hash);
        assert!(func_entry.is_some(), "variadic print should be registered");

        let func = func_entry.unwrap();
        // Variadic param is included in def.params (for type checking extra args)
        assert_eq!(
            func.def.params.len(),
            2,
            "variadic param included in def.params"
        );
        assert!(
            func.def.is_variadic,
            "function should be marked as variadic"
        );
    }

    #[test]
    fn context_install_generic_function_variadic_excluded_from_hash() {
        use angelscript_core::{GenericParamMeta, RefModifier, ReturnMeta};

        let mut ctx = Context::new();

        // Function with two required params and variadic
        let mut module = Module::new();
        module.functions.push(FunctionMeta {
            name: "format",
            as_name: None,
            native_fn: None,
            params: vec![],
            generic_params: vec![
                GenericParamMeta {
                    type_hash: primitives::STRING,
                    ref_mode: RefModifier::In,
                    is_variadic: false,
                    default_value: None,
                    if_handle_then_const: false,
                    is_const: false,
                },
                GenericParamMeta {
                    type_hash: primitives::INT32,
                    ref_mode: RefModifier::In,
                    is_variadic: false,
                    default_value: None,
                    if_handle_then_const: false,
                    is_const: false,
                },
                GenericParamMeta {
                    type_hash: primitives::VARIABLE_PARAM,
                    ref_mode: RefModifier::In,
                    is_variadic: true,
                    default_value: None,
                    if_handle_then_const: false,
                    is_const: false,
                },
            ],
            return_meta: ReturnMeta::default(),
            is_method: false,
            associated_type: None,
            behavior: None,
            is_const: false,
            is_property: false,
            property_name: None,
            is_generic: true,
            list_pattern: None,
            template_params: vec![],
        });
        ctx.install(module).unwrap();

        // Hash computed with only required params (not variadic)
        let func_hash = TypeHash::from_function("format", &[primitives::STRING, primitives::INT32]);
        let func_entry = ctx.registry().get_function(func_hash);
        assert!(func_entry.is_some(), "format function should be found");

        let func = func_entry.unwrap();
        // Variadic param is included in def.params (for type checking extra args)
        assert_eq!(func.def.params.len(), 3);
        assert!(func.def.is_variadic);
    }
}
