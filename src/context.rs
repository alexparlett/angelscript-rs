//! Execution context for the scripting engine.
//!
//! The Context owns a [`TypeRegistry`] that stores all registered types and functions.
//! Users install modules into the context, then create compilation units from it.

use std::sync::Arc;
use thiserror::Error;

use angelscript_core::{
    ClassEntry, ClassMeta, DataType, FuncdefEntry, FuncdefMeta, FunctionDef, FunctionEntry,
    FunctionMeta, FunctionTraits, InterfaceEntry, InterfaceMeta, MethodSignature, Param,
    PropertyEntry, TypeHash, TypeSource, Visibility,
};
use angelscript_registry::{Module, TypeRegistry};

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
    registry: TypeRegistry,
}

impl Context {
    /// Create a new context with primitives pre-registered.
    pub fn new() -> Self {
        Self {
            registry: TypeRegistry::with_primitives(),
        }
    }

    /// Create a context with default modules pre-installed.
    ///
    /// This registers the standard library types (string, array, dictionary, etc.)
    /// in addition to primitives.
    pub fn with_default_modules() -> Result<Self, ContextError> {
        // TODO: Install stdlib modules when they're implemented
        Ok(Self::new())
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
            self.install_class(&qualified_ns, class_meta)?;
        }

        // Install global functions - pass namespace Vec directly
        for func_meta in module.functions {
            self.install_function(&module.namespace, None, func_meta)?;
        }

        // Install interfaces
        for interface_meta in module.interfaces {
            self.install_interface(&qualified_ns, interface_meta)?;
        }

        // Install funcdefs
        for funcdef_meta in module.funcdefs {
            self.install_funcdef(&qualified_ns, funcdef_meta)?;
        }

        Ok(())
    }

    /// Get a reference to the type registry.
    pub fn registry(&self) -> &TypeRegistry {
        &self.registry
    }

    /// Create a new compilation unit from this context.
    pub fn create_unit(self: &Arc<Self>) -> Result<Unit, ContextError> {
        Ok(Unit::with_context(Arc::clone(self)))
    }

    // =========================================================================
    // Private installation helpers
    // =========================================================================

    fn install_class(&mut self, namespace: &str, meta: ClassMeta) -> Result<(), ContextError> {
        let qualified_name = if namespace.is_empty() {
            meta.name.to_string()
        } else {
            format!("{}::{}", namespace, meta.name)
        };

        // Create the class entry
        let mut class_entry = ClassEntry::new(
            meta.name,
            &qualified_name,
            meta.type_hash,
            meta.type_kind,
            TypeSource::ffi_untyped(),
        );

        // Add template parameters
        if !meta.template_params.is_empty() {
            class_entry = class_entry.with_template_params(meta.template_params);
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

        // Compute function hash directly from iterator (no Vec allocation)
        let func_hash =
            TypeHash::from_function_iter(name, meta.params.iter().map(|p| p.type_hash));

        // Convert parameters
        let params: Vec<Param> = meta
            .params
            .iter()
            .map(|p| {
                if p.default_value.is_some() {
                    Param::with_default(p.name, DataType::simple(p.type_hash))
                } else {
                    Param::new(p.name, DataType::simple(p.type_hash))
                }
            })
            .collect();

        // Determine return type
        let return_type = meta
            .return_meta
            .type_hash
            .map(DataType::simple)
            .unwrap_or_else(DataType::void);

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

        let def = FunctionDef::new(
            func_hash,
            name.to_string(),
            namespace.to_vec(),
            params,
            return_type,
            object_type,
            traits,
            true, // is_native
            Visibility::Public,
        );

        let entry = FunctionEntry::ffi(def);

        self.registry
            .register_function(entry)
            .map_err(|e| ContextError::RegistrationFailed(e.to_string()))?;

        Ok(())
    }

    fn install_interface(
        &mut self,
        namespace: &str,
        meta: InterfaceMeta,
    ) -> Result<(), ContextError> {
        let qualified_name = if namespace.is_empty() {
            meta.name.to_string()
        } else {
            format!("{}::{}", namespace, meta.name)
        };

        let mut entry = InterfaceEntry::new(
            meta.name,
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

    fn install_funcdef(&mut self, namespace: &str, meta: FuncdefMeta) -> Result<(), ContextError> {
        let qualified_name = if namespace.is_empty() {
            meta.name.to_string()
        } else {
            format!("{}::{}", namespace, meta.name)
        };

        let params: Vec<DataType> = meta
            .param_types
            .iter()
            .map(|&h| DataType::simple(h))
            .collect();
        let return_type = DataType::simple(meta.return_type);

        let entry = FuncdefEntry::new(
            meta.name,
            &qualified_name,
            meta.type_hash,
            TypeSource::ffi_untyped(),
            params,
            return_type,
        );

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
    use angelscript_core::{primitives, TypeKind};

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
            properties: vec![],
            template_params: vec![],
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
            properties: vec![],
            template_params: vec![],
        });
        ctx.install(module).unwrap();

        assert!(ctx.registry().has_namespace("Game"));
        assert!(ctx
            .registry()
            .get(TypeHash::from_name("Game::Entity"))
            .is_some());
    }

    #[test]
    fn context_install_duplicate_type_fails() {
        let mut ctx = Context::new();

        let class_meta = ClassMeta {
            name: "Player",
            type_hash: TypeHash::from_name("Player"),
            type_kind: TypeKind::reference(),
            properties: vec![],
            template_params: vec![],
        };

        let mut module1 = Module::new();
        module1.classes.push(class_meta.clone());
        let mut module2 = Module::new();
        module2.classes.push(class_meta);

        ctx.install(module1).unwrap();
        let result = ctx.install(module2);
        assert!(result.is_err());
    }
}
