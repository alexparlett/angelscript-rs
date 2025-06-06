use crate::internal::callback_manager::GenericFn;
use crate::prelude::{Behaviour, ObjectTypeFlags};
use std::marker::PhantomData;
use crate::types::script_data::ScriptData;

/// A plugin that groups related AngelScript registrations
pub struct Plugin {
    name: String,
    namespace: Option<String>,
    pub(crate) registrations: Vec<Registration>,
}

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
    ObjectType {
        name: String,
        size: i32,
        flags: ObjectTypeFlags,
        type_builder: TypeBuilder,
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
    pub(crate) byte_offset: Option<i32>,
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

impl Plugin {
    /// Create a new plugin with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: None,
            registrations: Vec::new(),
        }
    }

    /// Get the plugin name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the namespace for all registrations in this plugin
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
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
            plugin: self,
            type_name: type_name.clone(),
            size: std::mem::size_of::<T>() as i32,
            flags: ObjectTypeFlags::VALUE, // asOBJ_VALUE as default
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
}

/// Builder for configuring object type registrations
pub struct TypeRegistration<T> {
    plugin: Plugin,
    type_name: String,
    size: i32,
    flags: ObjectTypeFlags,
    type_builder: TypeBuilder,
    _phantom: PhantomData<T>,
}

impl<T: 'static> TypeRegistration<T> {
    /// Mark this type as a value type
    pub fn as_value_type(&mut self) -> &mut Self {
        self.flags = ObjectTypeFlags::VALUE; // asOBJ_VALUE
        self
    }

    /// Mark this type as a reference type
    pub fn as_reference_type(&mut self) -> &mut Self {
        self.flags = ObjectTypeFlags::REF; // asOBJ_REF
        self
    }

    /// Add POD flag (Plain Old Data)
    pub fn with_pod_flag(&mut self) -> &mut Self {
        self.flags |= ObjectTypeFlags::POD; // asOBJ_POD
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
        byte_offset: Option<i32>,
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
    fn register(mut self) -> Plugin {
        self.plugin.registrations.push(Registration::ObjectType {
            name: self.type_name,
            size: self.size,
            flags: self.flags,
            type_builder: self.type_builder,
        });

        self.plugin
    }
}
