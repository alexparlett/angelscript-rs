use angelscript_core::core::engine::{Engine, EngineInstallable};
use angelscript_core::core::error::ScriptResult;
use angelscript_core::types::callbacks::GenericFn;
use angelscript_core::types::enums::{Behaviour, ObjectTypeFlags};
use angelscript_core::types::script_data::ScriptData;
use std::marker::PhantomData;

/// Internal representation of different registration types
pub(crate) enum Registration {
    GlobalFunction {
        declaration: String,
        function: GenericFn,
        auxiliary: Option<Box<dyn ScriptData>>,
    },
    GlobalProperty {
        declaration: String,
        property: Box<dyn ScriptData>,
    },
    Enum {
        name: String,
        values: Vec<EnumRegistration>,
    },
    ObjectType {
        name: String,
        size: i32,
        flags: ObjectTypeFlags,
        type_builder: TypeBuilder,
    },
    Interface {
        name: String,
        methods: Vec<String>,
    },
    Funcdef {
        declaration: String,
    },
}

/// Builder for object type registrations
pub struct TypeBuilder {
    pub(crate) methods: Vec<MethodRegistration>,
    pub(crate) properties: Vec<PropertyRegistration>,
    pub(crate) behaviors: Vec<BehaviorRegistration>,
}

pub(crate) struct MethodRegistration {
    pub(crate) declaration: String,
    pub(crate) function: GenericFn,
    pub(crate) auxiliary: Option<Box<dyn ScriptData>>,
    pub(crate) composite_offset: Option<i32>,
    pub(crate) is_composite_indirect: Option<bool>,
}

pub(crate) struct PropertyRegistration {
    pub(crate) declaration: String,
    pub(crate) byte_offset: i32,
    pub(crate) composite_offset: Option<i32>,
    pub(crate) is_composite_indirect: Option<bool>,
}

pub(crate) struct BehaviorRegistration {
    pub(crate) behavior: Behaviour,
    pub(crate) declaration: String,
    pub(crate) function: GenericFn,
    pub(crate) auxiliary: Option<Box<dyn ScriptData>>,
    pub(crate) composite_offset: Option<i32>,
    pub(crate) is_composite_indirect: Option<bool>,
}

pub(crate) struct EnumRegistration {
    pub(crate) name: String,
    pub(crate) value: i32,
}

/// A plugin that groups related AngelScript registrations
pub struct Addon {
    pub(crate) namespace: Option<String>,
    pub(crate) registrations: Vec<Registration>,
    pub(crate) engine_configuration: Option<Box<dyn FnOnce(&Engine) -> ScriptResult<()>>>,
}

impl Addon {
    /// Create a new plugin with no namespace (global namespace)
    pub fn new() -> Self {
        Self {
            namespace: None,
            registrations: Vec::new(),
            engine_configuration: None,
        }
    }

    /// Set the namespace for all registrations in this plugin
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    pub fn with_engine_configuration(
        mut self,
        configure: impl FnOnce(&Engine) -> ScriptResult<()> + 'static,
    ) -> Self {
        let _ = &self.engine_configuration.insert(Box::new(configure));
        self
    }

    /// Register an enum with its values
    pub fn with_enum(
        mut self,
        name: impl Into<String>,
        values: Vec<(impl Into<String>, i32)>,
    ) -> Self {
        let name = name.into();
        let enum_values = values
            .into_iter()
            .map(|(name, value)| EnumRegistration {
                name: name.into(),
                value,
            })
            .collect();

        self.registrations.push(Registration::Enum {
            name,
            values: enum_values,
        });

        self
    }

    /// Register a global function
    pub fn with_global_function(
        mut self,
        declaration: impl Into<String>,
        function: GenericFn,
        auxiliary: Option<Box<dyn ScriptData>>,
    ) -> Self {
        let declaration = declaration.into();

        self.registrations.push(Registration::GlobalFunction {
            declaration,
            function,
            auxiliary,
        });

        self
    }

    /// Register a global property
    pub fn with_global_property(
        mut self,
        declaration: impl Into<String>,
        property: Box<dyn ScriptData>,
    ) -> Self {
        let declaration = declaration.into();

        self.registrations.push(Registration::GlobalProperty {
            declaration,
            property,
        });

        self
    }

    /// Register an object type and return a TypeRegistration for further configuration
    pub fn with_type<T>(
        self,
        name: impl Into<String>,
        configure: impl FnOnce(&mut TypeRegistration<T>),
    ) -> Self
    where
        T: 'static,
    {
        let type_name = name.into();

        let mut type_registration = TypeRegistration {
            addon: self,
            type_name: type_name.clone(),
            size: size_of::<T>() as i32,
            flags: ObjectTypeFlags::REF, // Default
            type_builder: TypeBuilder {
                methods: Vec::new(),
                properties: Vec::new(),
                behaviors: Vec::new(),
            },
            _phantom: PhantomData,
        };

        // Use the closure to configure the TypeRegistration
        configure(&mut type_registration);

        // Finish the type registration and return the updated plugin
        type_registration.register()
    }

    /// Register an interface and return an InterfaceRegistration for further configuration
    pub fn with_interface(
        self,
        name: impl Into<String>,
        configure: impl FnOnce(&mut InterfaceRegistration),
    ) -> Self {
        let interface_name = name.into();

        let mut interface_registration = InterfaceRegistration {
            addon: self,
            interface_name: interface_name.clone(),
            methods: Vec::new(),
        };

        // Use the closure to configure the InterfaceRegistration
        configure(&mut interface_registration);

        // Finish the interface registration and return the updated addon
        interface_registration.register()
    }

    /// Register a function definition (funcdef)
    pub fn with_funcdef(mut self, declaration: impl Into<String>) -> Self {
        self.registrations.push(Registration::Funcdef {
            declaration: declaration.into(),
        });

        self
    }


    pub(crate) fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }
}

impl Default for Addon {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for configuring interface registrations
#[doc(hidden)]
pub struct InterfaceRegistration {
    addon: Addon,
    interface_name: String,
    methods: Vec<String>,
}

impl InterfaceRegistration {
    /// Add a method to the interface
    pub fn with_method(&mut self, declaration: impl Into<String>) -> &mut Self {
        self.methods.push(declaration.into());
        self
    }

    /// Finish interface registration and return to addon
    fn register(mut self) -> Addon {
        self.addon.registrations.push(Registration::Interface {
            name: self.interface_name,
            methods: self.methods,
        });

        self.addon
    }
}

/// Builder for configuring object type registrations
#[doc(hidden)]
pub struct TypeRegistration<T> {
    addon: Addon,
    type_name: String,
    size: i32,
    flags: ObjectTypeFlags,
    type_builder: TypeBuilder,
    _phantom: PhantomData<T>,
}

impl<T: 'static> TypeRegistration<T> {
    /// Mark this type as a value type
    pub fn as_value_type(&mut self) -> &mut Self {
        self.flags = ObjectTypeFlags::VALUE;
        self
    }

    /// Mark this type as a reference type
    pub fn as_reference_type(&mut self) -> &mut Self {
        self.flags = ObjectTypeFlags::REF;
        self
    }

    /// Add POD flag (Plain Old Data)
    pub fn with_pod_flag(&mut self) -> &mut Self {
        self.flags |= ObjectTypeFlags::POD;
        self
    }

    /// Set custom flags
    pub fn with_flags(&mut self, flags: ObjectTypeFlags) -> &mut Self {
        self.flags = flags;
        self
    }

    /// Register a method
    pub fn with_method(
        &mut self,
        declaration: impl Into<String>,
        function: GenericFn,
        auxiliary: Option<Box<dyn ScriptData>>,
        composite_offset: Option<i32>,
        is_composite_indirect: Option<bool>,
    ) -> &mut Self {
        let declaration = declaration.into();

        self.type_builder.methods.push(MethodRegistration {
            declaration,
            function,
            auxiliary,
            composite_offset,
            is_composite_indirect,
        });

        self
    }

    /// Register a property
    pub fn with_property(
        &mut self,
        declaration: impl Into<String>,
        byte_offset: i32,
        composite_offset: Option<i32>,
        is_composite_indirect: Option<bool>,
    ) -> &mut Self {
        let declaration = declaration.into();

        self.type_builder.properties.push(PropertyRegistration {
            declaration,
            byte_offset,
            composite_offset,
            is_composite_indirect,
        });

        self
    }

    /// Register a behavior
    pub fn with_behavior(
        &mut self,
        behavior: Behaviour,
        declaration: impl Into<String>,
        function: GenericFn,
        auxiliary: Option<Box<dyn ScriptData>>,
        composite_offset: Option<i32>,
        is_composite_indirect: Option<bool>,
    ) -> &mut Self {
        let declaration = declaration.into();

        self.type_builder.behaviors.push(BehaviorRegistration {
            behavior,
            declaration,
            function,
            auxiliary,
            composite_offset,
            is_composite_indirect,
        });

        self
    }

    /// Finish type registration and return to plugin
    fn register(mut self) -> Addon {
        self.addon.registrations.push(Registration::ObjectType {
            name: self.type_name,
            size: self.size,
            flags: self.flags,
            type_builder: self.type_builder,
        });

        self.addon
    }
}

impl EngineInstallable for Addon {
    fn install(self, engine: &Engine) -> ScriptResult<()> {
        let was_namespaced = if let Some(namespace) = self.namespace() {
            engine.set_default_namespace(namespace)?;
            true
        } else {
            false
        };
        for registration in self.registrations {
            match registration {
                Registration::GlobalFunction {
                    declaration,
                    function,
                    auxiliary,
                } => {
                    engine.register_global_function(&declaration, function, auxiliary)?;
                }
                Registration::GlobalProperty {
                    declaration,
                    property,
                } => {
                    engine.register_global_property(&declaration, property)?;
                }
                Registration::ObjectType {
                    name,
                    size,
                    flags,
                    type_builder,
                } => {
                    engine.register_object_type(&name, size, flags)?;

                    // Apply methods
                    for mut method in type_builder.methods {
                        engine.register_object_method(
                            &name,
                            &method.declaration,
                            method.function,
                            method.auxiliary.as_mut(),
                            method.composite_offset,
                            method.is_composite_indirect,
                        )?;
                    }

                    // Apply properties
                    for property in type_builder.properties {
                        engine.register_object_property(
                            &name,
                            &property.declaration,
                            property.byte_offset,
                            property.composite_offset.unwrap_or(0),
                            property.is_composite_indirect.unwrap_or(false),
                        )?;
                    }

                    // Apply custom behaviors
                    for mut behavior in type_builder.behaviors {
                        engine.register_object_behaviour(
                            &name,
                            behavior.behavior,
                            &behavior.declaration,
                            behavior.function,
                            behavior.auxiliary.as_mut(),
                            behavior.composite_offset,
                            behavior.is_composite_indirect,
                        )?;
                    }
                }
                Registration::Enum { name, values } => {
                    engine.register_enum(name.as_str())?;
                    for value in values {
                        engine.register_enum_value(
                            name.as_str(),
                            value.name.as_str(),
                            value.value,
                        )?;
                    }
                }
                Registration::Interface { name, methods } => {
                    engine.register_interface(&name)?;

                    for method in methods {
                        engine.register_interface_method(&name, &method)?;
                    }
                }
                Registration::Funcdef { declaration } => {
                    engine.register_funcdef(&declaration)?;
                }
            }
        }
        if was_namespaced {
            engine.set_default_namespace("")?;
        }
        if let Some(engine_configuration) = self.engine_configuration {
            engine_configuration(engine)?;
        }
        Ok(())
    }
}
