//! Global registry for types, functions, and variables.
//!
//! The Registry is the central storage for all global declarations in an AngelScript
//! program. It stores types (primitives, classes, interfaces, enums), functions
//! (with overloading support), and global variables.
//!
//! # Architecture
//!
//! - **Types**: Stored in a Vec with TypeId as index, plus a name→TypeId map
//! - **Functions**: Stored in a Vec with FunctionId as index, plus a name→[FunctionId] map for overloading
//! - **Template Cache**: Memoizes template instantiations to avoid duplicates
//!
//! # Example
//!
//! ```
//! use angelscript::semantic::{Registry, TypeDef, PrimitiveType, INT32_TYPE};
//!
//! let registry = Registry::new();
//!
//! // Built-in types are pre-registered
//! let int_type = registry.get_type(INT32_TYPE);
//! assert!(int_type.is_primitive());
//! ```

use super::data_type::DataType;
use super::type_def::{
    TypeId, TypeDef, FunctionId, PrimitiveType, FunctionTraits,
    VOID_TYPE, BOOL_TYPE, INT8_TYPE, INT16_TYPE, INT32_TYPE, INT64_TYPE,
    UINT8_TYPE, UINT16_TYPE, UINT32_TYPE, UINT64_TYPE, FLOAT_TYPE, DOUBLE_TYPE,
    STRING_TYPE, ARRAY_TEMPLATE, DICT_TEMPLATE, FIRST_USER_TYPE_ID,
};
use super::error::{SemanticError, SemanticErrorKind};
use crate::lexer::Span;
use rustc_hash::FxHashMap;

/// Function definition with complete signature
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    /// Function identifier
    pub id: FunctionId,
    /// Function name (unqualified)
    pub name: String,
    /// Namespace path (e.g., ["Game", "Player"])
    pub namespace: Vec<String>,
    /// Parameter types
    pub params: Vec<DataType>,
    /// Return type
    pub return_type: DataType,
    /// Object type if this is a method
    pub object_type: Option<TypeId>,
    /// Function traits (virtual, const, etc.)
    pub traits: FunctionTraits,
}

impl FunctionDef {
    /// Get the qualified name of this function
    pub fn qualified_name(&self) -> String {
        if self.namespace.is_empty() {
            self.name.clone()
        } else {
            format!("{}::{}", self.namespace.join("::"), self.name)
        }
    }
}

/// A global variable definition
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalVarDef {
    /// Variable name (unqualified)
    pub name: String,
    /// Namespace path
    pub namespace: Vec<String>,
    /// Variable type
    pub data_type: DataType,
}

impl GlobalVarDef {
    /// Get the qualified name of this variable
    pub fn qualified_name(&self) -> String {
        if self.namespace.is_empty() {
            self.name.clone()
        } else {
            format!("{}::{}", self.namespace.join("::"), self.name)
        }
    }
}

/// Global registry for all types, functions, and variables
#[derive(Debug, Clone)]
pub struct Registry {
    // Type storage
    types: Vec<TypeDef>,
    type_by_name: FxHashMap<String, TypeId>,

    // Function storage
    functions: Vec<FunctionDef>,
    func_by_name: FxHashMap<String, Vec<FunctionId>>,

    // Global variable storage
    global_vars: FxHashMap<String, GlobalVarDef>,

    // Template instantiation cache (Template TypeId + args → Instance TypeId)
    template_cache: FxHashMap<(TypeId, Vec<DataType>), TypeId>,

    // Fixed TypeIds for quick access
    pub void_type: TypeId,
    pub bool_type: TypeId,
    pub int8_type: TypeId,
    pub int16_type: TypeId,
    pub int32_type: TypeId,
    pub int64_type: TypeId,
    pub uint8_type: TypeId,
    pub uint16_type: TypeId,
    pub uint32_type: TypeId,
    pub uint64_type: TypeId,
    pub float_type: TypeId,
    pub double_type: TypeId,
    pub string_type: TypeId,
    pub array_template: TypeId,
    pub dict_template: TypeId,
}

impl Registry {
    /// Create a new registry with all built-in types pre-registered
    pub fn new() -> Self {
        let mut registry = Self {
            types: Vec::with_capacity(32),
            type_by_name: FxHashMap::default(),
            functions: Vec::new(),
            func_by_name: FxHashMap::default(),
            global_vars: FxHashMap::default(),
            template_cache: FxHashMap::default(),
            void_type: VOID_TYPE,
            bool_type: BOOL_TYPE,
            int8_type: INT8_TYPE,
            int16_type: INT16_TYPE,
            int32_type: INT32_TYPE,
            int64_type: INT64_TYPE,
            uint8_type: UINT8_TYPE,
            uint16_type: UINT16_TYPE,
            uint32_type: UINT32_TYPE,
            uint64_type: UINT64_TYPE,
            float_type: FLOAT_TYPE,
            double_type: DOUBLE_TYPE,
            string_type: STRING_TYPE,
            array_template: ARRAY_TEMPLATE,
            dict_template: DICT_TEMPLATE,
        };

        // Pre-register primitives at fixed indices (0-11)
        registry.register_primitive(PrimitiveType::Void, VOID_TYPE);
        registry.register_primitive(PrimitiveType::Bool, BOOL_TYPE);
        registry.register_primitive(PrimitiveType::Int8, INT8_TYPE);
        registry.register_primitive(PrimitiveType::Int16, INT16_TYPE);
        registry.register_primitive(PrimitiveType::Int32, INT32_TYPE);
        registry.register_primitive(PrimitiveType::Int64, INT64_TYPE);
        registry.register_primitive(PrimitiveType::Uint8, UINT8_TYPE);
        registry.register_primitive(PrimitiveType::Uint16, UINT16_TYPE);
        registry.register_primitive(PrimitiveType::Uint32, UINT32_TYPE);
        registry.register_primitive(PrimitiveType::Uint64, UINT64_TYPE);
        registry.register_primitive(PrimitiveType::Float, FLOAT_TYPE);
        registry.register_primitive(PrimitiveType::Double, DOUBLE_TYPE);

        // Fill gap 12-15 with placeholders
        while registry.types.len() < 16 {
            registry.types.push(TypeDef::Primitive { kind: PrimitiveType::Void });
        }

        // Pre-register built-in types (16-18)
        registry.register_builtin_string(STRING_TYPE);
        registry.register_builtin_template("array", 1, ARRAY_TEMPLATE);
        registry.register_builtin_template("dictionary", 2, DICT_TEMPLATE);

        // Fill gap 19-31 with placeholders
        while registry.types.len() < FIRST_USER_TYPE_ID as usize {
            registry.types.push(TypeDef::Primitive { kind: PrimitiveType::Void });
        }

        registry
    }

    /// Register a primitive type at a fixed index
    fn register_primitive(&mut self, kind: PrimitiveType, type_id: TypeId) {
        let index = type_id.as_u32() as usize;

        // Ensure vector is large enough
        while self.types.len() <= index {
            self.types.push(TypeDef::Primitive { kind: PrimitiveType::Void });
        }

        self.types[index] = TypeDef::Primitive { kind };
        self.type_by_name.insert(kind.name().to_string(), type_id);
    }

    /// Register built-in string type
    fn register_builtin_string(&mut self, type_id: TypeId) {
        let index = type_id.as_u32() as usize;

        while self.types.len() <= index {
            self.types.push(TypeDef::Primitive { kind: PrimitiveType::Void });
        }

        self.types[index] = TypeDef::Class {
            name: "string".to_string(),
            qualified_name: "string".to_string(),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
        };
        self.type_by_name.insert("string".to_string(), type_id);
    }

    /// Register a built-in template
    fn register_builtin_template(&mut self, name: &str, param_count: usize, type_id: TypeId) {
        let index = type_id.as_u32() as usize;

        while self.types.len() <= index {
            self.types.push(TypeDef::Primitive { kind: PrimitiveType::Void });
        }

        self.types[index] = TypeDef::Template {
            name: name.to_string(),
            param_count,
        };
        self.type_by_name.insert(name.to_string(), type_id);
    }

    /// Register a new type and return its TypeId
    pub fn register_type(&mut self, typedef: TypeDef, name: Option<&str>) -> TypeId {
        let type_id = TypeId::new(self.types.len() as u32);
        self.types.push(typedef);

        if let Some(name) = name {
            self.type_by_name.insert(name.to_string(), type_id);
        }

        type_id
    }

    /// Look up a type by name (returns None if not found)
    pub fn lookup_type(&self, name: &str) -> Option<TypeId> {
        self.type_by_name.get(name).copied()
    }

    /// Get a type definition by TypeId
    pub fn get_type(&self, type_id: TypeId) -> &TypeDef {
        &self.types[type_id.as_u32() as usize]
    }

    /// Get a mutable type definition by TypeId
    pub fn get_type_mut(&mut self, type_id: TypeId) -> &mut TypeDef {
        &mut self.types[type_id.as_u32() as usize]
    }

    /// Instantiate a template with the given arguments
    pub fn instantiate_template(
        &mut self,
        template_id: TypeId,
        args: Vec<DataType>,
    ) -> Result<TypeId, SemanticError> {
        // Check if this is actually a template
        let template_def = self.get_type(template_id);
        let param_count = match template_def {
            TypeDef::Template { param_count, .. } => *param_count,
            _ => {
                return Err(SemanticError::new(
                    SemanticErrorKind::NotATemplate,
                    Span::default(),
                    format!("Type {:?} is not a template", template_id),
                ));
            }
        };

        // Check argument count
        if args.len() != param_count {
            return Err(SemanticError::new(
                SemanticErrorKind::WrongTemplateArgCount,
                Span::default(),
                format!(
                    "Template expects {} arguments, got {}",
                    param_count,
                    args.len()
                ),
            ));
        }

        // Check cache
        let cache_key = (template_id, args.clone());
        if let Some(&cached_id) = self.template_cache.get(&cache_key) {
            return Ok(cached_id);
        }

        // Create new instance
        let instance = TypeDef::TemplateInstance {
            template: template_id,
            sub_types: args.clone(),
        };

        let instance_id = self.register_type(instance, None);
        self.template_cache.insert(cache_key, instance_id);

        Ok(instance_id)
    }

    /// Register a function and return its FunctionId
    pub fn register_function(&mut self, def: FunctionDef) -> FunctionId {
        let func_id = def.id;
        let qualified_name = def.qualified_name();

        self.functions.push(def);

        // Add to overload map
        self.func_by_name
            .entry(qualified_name)
            .or_default()
            .push(func_id);

        func_id
    }

    /// Look up all functions with the given name (for overload resolution)
    pub fn lookup_functions(&self, name: &str) -> &[FunctionId] {
        self.func_by_name
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get a function definition by FunctionId
    pub fn get_function(&self, func_id: FunctionId) -> &FunctionDef {
        &self.functions[func_id.as_u32() as usize]
    }

    /// Get all methods for a given type
    pub fn get_methods(&self, type_id: TypeId) -> Vec<FunctionId> {
        self.functions
            .iter()
            .filter(|f| f.object_type == Some(type_id))
            .map(|f| f.id)
            .collect()
    }

    /// Get the total number of functions registered
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Register a global variable
    pub fn register_global_var(&mut self, name: String, namespace: Vec<String>, data_type: DataType) {
        let qualified_name = if namespace.is_empty() {
            name.clone()
        } else {
            format!("{}::{}", namespace.join("::"), name)
        };

        self.global_vars.insert(
            qualified_name,
            GlobalVarDef {
                name,
                namespace,
                data_type,
            },
        );
    }

    /// Look up a global variable by qualified name
    pub fn lookup_global_var(&self, name: &str) -> Option<&GlobalVarDef> {
        self.global_vars.get(name)
    }

    /// Update a class with complete details (fields, methods, inheritance)
    pub fn update_class_details(
        &mut self,
        type_id: TypeId,
        fields: Vec<super::type_def::FieldDef>,
        methods: Vec<FunctionId>,
        base_class: Option<TypeId>,
        interfaces: Vec<TypeId>,
    ) {
        let typedef = self.get_type_mut(type_id);
        if let TypeDef::Class {
            fields: class_fields,
            methods: class_methods,
            base_class: class_base,
            interfaces: class_interfaces,
            ..
        } = typedef
        {
            *class_fields = fields;
            *class_methods = methods;
            *class_base = base_class;
            *class_interfaces = interfaces;
        }
    }

    /// Update an interface with complete method signatures
    pub fn update_interface_details(
        &mut self,
        type_id: TypeId,
        methods: Vec<super::type_def::MethodSignature>,
    ) {
        let typedef = self.get_type_mut(type_id);
        if let TypeDef::Interface {
            methods: interface_methods,
            ..
        } = typedef
        {
            *interface_methods = methods;
        }
    }

    /// Update a funcdef with complete signature
    pub fn update_funcdef_signature(
        &mut self,
        type_id: TypeId,
        params: Vec<DataType>,
        return_type: DataType,
    ) {
        let typedef = self.get_type_mut(type_id);
        if let TypeDef::Funcdef {
            params: funcdef_params,
            return_type: funcdef_return,
            ..
        } = typedef
        {
            *funcdef_params = params;
            *funcdef_return = return_type;
        }
    }

    /// Update a function's signature
    pub fn update_function_signature(
        &mut self,
        qualified_name: &str,
        params: Vec<DataType>,
        return_type: DataType,
        object_type: Option<TypeId>,
    ) {
        // Find the function(s) with this name
        if let Some(func_ids) = self.func_by_name.get(qualified_name) {
            // For now, update all functions with this name
            // TODO: In a real implementation, we'd match by signature for overload resolution
            for &func_id in func_ids {
                let index = func_id.as_u32() as usize;
                if index < self.functions.len() {
                    self.functions[index].params = params.clone();
                    self.functions[index].return_type = return_type.clone();
                    if object_type.is_some() {
                        self.functions[index].object_type = object_type;
                    }
                }
            }
        }
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::type_def::Visibility;

    #[test]
    fn registry_new_has_primitives() {
        let registry = Registry::new();
        assert_eq!(registry.types.len(), 32);
    }

    #[test]
    fn registry_void_type() {
        let registry = Registry::new();
        let typedef = registry.get_type(VOID_TYPE);
        assert!(typedef.is_primitive());
        assert_eq!(typedef.name(), "void");
    }

    #[test]
    fn registry_bool_type() {
        let registry = Registry::new();
        let typedef = registry.get_type(BOOL_TYPE);
        assert!(typedef.is_primitive());
        assert_eq!(typedef.name(), "bool");
    }

    #[test]
    fn registry_int_types() {
        let registry = Registry::new();

        assert_eq!(registry.get_type(INT8_TYPE).name(), "int8");
        assert_eq!(registry.get_type(INT16_TYPE).name(), "int16");
        assert_eq!(registry.get_type(INT32_TYPE).name(), "int");
        assert_eq!(registry.get_type(INT64_TYPE).name(), "int64");
    }

    #[test]
    fn registry_uint_types() {
        let registry = Registry::new();

        assert_eq!(registry.get_type(UINT8_TYPE).name(), "uint8");
        assert_eq!(registry.get_type(UINT16_TYPE).name(), "uint16");
        assert_eq!(registry.get_type(UINT32_TYPE).name(), "uint");
        assert_eq!(registry.get_type(UINT64_TYPE).name(), "uint64");
    }

    #[test]
    fn registry_float_types() {
        let registry = Registry::new();

        assert_eq!(registry.get_type(FLOAT_TYPE).name(), "float");
        assert_eq!(registry.get_type(DOUBLE_TYPE).name(), "double");
    }

    #[test]
    fn registry_string_type() {
        let registry = Registry::new();
        let typedef = registry.get_type(STRING_TYPE);
        assert!(typedef.is_class());
        assert_eq!(typedef.name(), "string");
    }

    #[test]
    fn registry_array_template() {
        let registry = Registry::new();
        let typedef = registry.get_type(ARRAY_TEMPLATE);
        assert!(typedef.is_template());
        assert_eq!(typedef.name(), "array");
    }

    #[test]
    fn registry_dict_template() {
        let registry = Registry::new();
        let typedef = registry.get_type(DICT_TEMPLATE);
        assert!(typedef.is_template());
        assert_eq!(typedef.name(), "dictionary");
    }

    #[test]
    fn lookup_primitive_by_name() {
        let registry = Registry::new();

        assert_eq!(registry.lookup_type("void"), Some(VOID_TYPE));
        assert_eq!(registry.lookup_type("bool"), Some(BOOL_TYPE));
        assert_eq!(registry.lookup_type("int"), Some(INT32_TYPE));
        assert_eq!(registry.lookup_type("uint"), Some(UINT32_TYPE));
        assert_eq!(registry.lookup_type("float"), Some(FLOAT_TYPE));
        assert_eq!(registry.lookup_type("double"), Some(DOUBLE_TYPE));
    }

    #[test]
    fn lookup_string_by_name() {
        let registry = Registry::new();
        assert_eq!(registry.lookup_type("string"), Some(STRING_TYPE));
    }

    #[test]
    fn lookup_template_by_name() {
        let registry = Registry::new();
        assert_eq!(registry.lookup_type("array"), Some(ARRAY_TEMPLATE));
        assert_eq!(registry.lookup_type("dictionary"), Some(DICT_TEMPLATE));
    }

    #[test]
    fn lookup_nonexistent_type() {
        let registry = Registry::new();
        assert_eq!(registry.lookup_type("NonExistent"), None);
    }

    #[test]
    fn register_simple_class() {
        let mut registry = Registry::new();

        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
        };

        let type_id = registry.register_type(typedef, Some("Player"));
        assert_eq!(registry.lookup_type("Player"), Some(type_id));
        assert!(registry.get_type(type_id).is_class());
    }

    #[test]
    fn register_qualified_class() {
        let mut registry = Registry::new();

        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Game::Player".to_string(),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
        };

        let type_id = registry.register_type(typedef, Some("Game::Player"));
        assert_eq!(registry.lookup_type("Game::Player"), Some(type_id));
    }

    #[test]
    fn register_interface() {
        let mut registry = Registry::new();

        let typedef = TypeDef::Interface {
            name: "IDrawable".to_string(),
            qualified_name: "IDrawable".to_string(),
            methods: Vec::new(),
        };

        let type_id = registry.register_type(typedef, Some("IDrawable"));
        assert_eq!(registry.lookup_type("IDrawable"), Some(type_id));
        assert!(registry.get_type(type_id).is_interface());
    }

    #[test]
    fn register_enum() {
        let mut registry = Registry::new();

        let typedef = TypeDef::Enum {
            name: "Color".to_string(),
            qualified_name: "Color".to_string(),
            values: vec![
                ("Red".to_string(), 0),
                ("Green".to_string(), 1),
                ("Blue".to_string(), 2),
            ],
        };

        let type_id = registry.register_type(typedef, Some("Color"));
        assert_eq!(registry.lookup_type("Color"), Some(type_id));
        assert!(registry.get_type(type_id).is_enum());
    }

    #[test]
    fn register_funcdef() {
        let mut registry = Registry::new();

        let typedef = TypeDef::Funcdef {
            name: "Callback".to_string(),
            qualified_name: "Callback".to_string(),
            params: vec![DataType::simple(INT32_TYPE)],
            return_type: DataType::simple(VOID_TYPE),
        };

        let type_id = registry.register_type(typedef, Some("Callback"));
        assert_eq!(registry.lookup_type("Callback"), Some(type_id));
        assert!(registry.get_type(type_id).is_funcdef());
    }

    #[test]
    fn get_type_mut() {
        let mut registry = Registry::new();

        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
        };

        let type_id = registry.register_type(typedef, Some("Player"));

        // Modify the type
        if let TypeDef::Class { fields, .. } = registry.get_type_mut(type_id) {
            fields.push(super::super::type_def::FieldDef::new(
                "health".to_string(),
                DataType::simple(INT32_TYPE),
                Visibility::Public,
            ));
        }

        // Verify modification
        if let TypeDef::Class { fields, .. } = registry.get_type(type_id) {
            assert_eq!(fields.len(), 1);
        } else {
            panic!("Expected Class");
        }
    }

    #[test]
    fn instantiate_array_template() {
        let mut registry = Registry::new();

        let instance_id = registry
            .instantiate_template(ARRAY_TEMPLATE, vec![DataType::simple(INT32_TYPE)])
            .unwrap();

        let typedef = registry.get_type(instance_id);
        assert!(typedef.is_template_instance());
    }

    #[test]
    fn instantiate_template_caching() {
        let mut registry = Registry::new();

        let instance1 = registry
            .instantiate_template(ARRAY_TEMPLATE, vec![DataType::simple(INT32_TYPE)])
            .unwrap();

        let instance2 = registry
            .instantiate_template(ARRAY_TEMPLATE, vec![DataType::simple(INT32_TYPE)])
            .unwrap();

        assert_eq!(instance1, instance2);
    }

    #[test]
    fn instantiate_template_different_args() {
        let mut registry = Registry::new();

        let instance1 = registry
            .instantiate_template(ARRAY_TEMPLATE, vec![DataType::simple(INT32_TYPE)])
            .unwrap();

        let instance2 = registry
            .instantiate_template(ARRAY_TEMPLATE, vec![DataType::simple(FLOAT_TYPE)])
            .unwrap();

        assert_ne!(instance1, instance2);
    }

    #[test]
    fn instantiate_dict_template() {
        let mut registry = Registry::new();

        let instance_id = registry
            .instantiate_template(
                DICT_TEMPLATE,
                vec![DataType::simple(STRING_TYPE), DataType::simple(INT32_TYPE)],
            )
            .unwrap();

        let typedef = registry.get_type(instance_id);
        assert!(typedef.is_template_instance());
    }

    #[test]
    fn instantiate_nested_template() {
        let mut registry = Registry::new();

        // array<int>
        let inner = registry
            .instantiate_template(ARRAY_TEMPLATE, vec![DataType::simple(INT32_TYPE)])
            .unwrap();

        // array<array<int>>
        let outer = registry
            .instantiate_template(ARRAY_TEMPLATE, vec![DataType::simple(inner)])
            .unwrap();

        let typedef = registry.get_type(outer);
        assert!(typedef.is_template_instance());
    }

    #[test]
    fn instantiate_non_template_fails() {
        let mut registry = Registry::new();

        let result = registry.instantiate_template(INT32_TYPE, vec![DataType::simple(INT32_TYPE)]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, SemanticErrorKind::NotATemplate);
    }

    #[test]
    fn instantiate_wrong_arg_count_fails() {
        let mut registry = Registry::new();

        // array expects 1 arg, give it 2
        let result = registry.instantiate_template(
            ARRAY_TEMPLATE,
            vec![DataType::simple(INT32_TYPE), DataType::simple(FLOAT_TYPE)],
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, SemanticErrorKind::WrongTemplateArgCount);
    }

    #[test]
    fn register_function() {
        let mut registry = Registry::new();

        let func = FunctionDef {
            id: FunctionId::new(0),
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: vec![DataType::simple(INT32_TYPE)],
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
        };

        let func_id = registry.register_function(func);
        assert_eq!(func_id, FunctionId::new(0));
    }

    #[test]
    fn lookup_function_by_name() {
        let mut registry = Registry::new();

        let func = FunctionDef {
            id: FunctionId::new(0),
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: vec![DataType::simple(INT32_TYPE)],
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
        };

        registry.register_function(func);

        let functions = registry.lookup_functions("foo");
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0], FunctionId::new(0));
    }

    #[test]
    fn lookup_nonexistent_function() {
        let registry = Registry::new();
        let functions = registry.lookup_functions("nonexistent");
        assert_eq!(functions.len(), 0);
    }

    #[test]
    fn function_overloading() {
        let mut registry = Registry::new();

        // foo(int)
        let func1 = FunctionDef {
            id: FunctionId::new(0),
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: vec![DataType::simple(INT32_TYPE)],
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
        };

        // foo(float)
        let func2 = FunctionDef {
            id: FunctionId::new(1),
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: vec![DataType::simple(FLOAT_TYPE)],
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
        };

        registry.register_function(func1);
        registry.register_function(func2);

        let functions = registry.lookup_functions("foo");
        assert_eq!(functions.len(), 2);
    }

    #[test]
    fn qualified_function_name() {
        let func = FunctionDef {
            id: FunctionId::new(0),
            name: "update".to_string(),
            namespace: vec!["Game".to_string(), "Player".to_string()],
            params: Vec::new(),
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
        };

        assert_eq!(func.qualified_name(), "Game::Player::update");
    }

    #[test]
    fn unqualified_function_name() {
        let func = FunctionDef {
            id: FunctionId::new(0),
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
        };

        assert_eq!(func.qualified_name(), "foo");
    }

    #[test]
    fn get_function() {
        let mut registry = Registry::new();

        let func = FunctionDef {
            id: FunctionId::new(0),
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: vec![DataType::simple(INT32_TYPE)],
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
        };

        let func_id = registry.register_function(func.clone());
        let retrieved = registry.get_function(func_id);
        assert_eq!(retrieved.name, "foo");
    }

    #[test]
    fn get_methods_for_type() {
        let mut registry = Registry::new();

        let player_type = TypeId::new(100);

        // Method for Player
        let method1 = FunctionDef {
            id: FunctionId::new(0),
            name: "update".to_string(),
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: DataType::simple(VOID_TYPE),
            object_type: Some(player_type),
            traits: FunctionTraits::new(),
        };

        // Another method for Player
        let method2 = FunctionDef {
            id: FunctionId::new(1),
            name: "draw".to_string(),
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: DataType::simple(VOID_TYPE),
            object_type: Some(player_type),
            traits: FunctionTraits::new(),
        };

        // Global function (not a method)
        let global_func = FunctionDef {
            id: FunctionId::new(2),
            name: "main".to_string(),
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
        };

        registry.register_function(method1);
        registry.register_function(method2);
        registry.register_function(global_func);

        let methods = registry.get_methods(player_type);
        assert_eq!(methods.len(), 2);
        assert!(methods.contains(&FunctionId::new(0)));
        assert!(methods.contains(&FunctionId::new(1)));
        assert!(!methods.contains(&FunctionId::new(2)));
    }

    #[test]
    fn registry_default() {
        let registry = Registry::default();
        assert_eq!(registry.types.len(), 32);
    }

    #[test]
    fn registry_clone() {
        let mut registry = Registry::new();

        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
        };

        let type_id = registry.register_type(typedef, Some("Player"));

        let cloned = registry.clone();
        assert_eq!(cloned.lookup_type("Player"), Some(type_id));
    }
}
