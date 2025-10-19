use crate::core::engine::EngineInner;
use crate::parser::ast::{Class, ClassMember, DataType, Enum, Func, FuncDef, Interface, Namespace, Script, ScriptItem, Type, TypeMod, Typedef, Var, Visibility};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct SemanticAnalyzer {
    pub symbol_table: SymbolTable,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    current_function: Option<String>,
    current_class: Option<String>,
    /// Reference to the script engine for accessing registered types
    engine: Arc<RwLock<EngineInner>>,
    /// Local type IDs for script-defined types
    script_types: HashMap<String, ScriptTypeInfo>,
    next_script_type_id: u32,
}

#[derive(Debug, Clone)]
struct ScriptTypeInfo {
    pub type_id: u32,
    pub kind: ScriptTypeKind,
    pub members: Vec<MemberInfo>,
    pub methods: Vec<MethodInfo>,
    pub base_classes: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
enum ScriptTypeKind {
    Class,
    Interface,
    Enum,
}

#[derive(Debug, Clone)]
pub struct MemberInfo {
    pub name: String,
    pub type_id: u32,
    pub visibility: Visibility,
}

#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub return_type: u32,
    pub params: Vec<ParamInfo>,
    pub visibility: Option<Visibility>,
    pub is_const: bool,
}

#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub name: Option<String>,
    pub type_id: u32,
    pub is_ref: bool,
    pub is_const: bool,
}

#[derive(Debug, Clone)]
pub struct SymbolTable {
    scopes: Vec<Scope>,
    global_scope: HashMap<String, Symbol>,
}

#[derive(Debug, Clone)]
pub struct Scope {
    symbols: HashMap<String, Symbol>,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub symbol_type: SymbolType,
    pub type_id: u32,
    pub is_const: bool,
    pub is_initialized: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolType {
    Variable,
    Function,
    Parameter,
    Type,
    EnumVariant,
    Member,
}

pub type TypeId = u32;

impl SemanticAnalyzer {
    /// Create a semantic analyzer with access to the script engine
    pub fn new(engine: Arc<RwLock<EngineInner>>) -> Self {
        let mut analyzer = Self {
            symbol_table: SymbolTable::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            current_function: None,
            current_class: None,
            engine,
            script_types: HashMap::new(),
            next_script_type_id: 10000,
        };

        analyzer.import_engine_symbols();
        analyzer
    }

    /// Import symbols from the script engine
    fn import_engine_symbols(&mut self) {
        let engine_guard = self.engine.read().unwrap();

        // Import enum values as global symbols
        for (name, enum_type) in &engine_guard.enum_types {
            self.symbol_table.insert_global(Symbol {
                name: name.clone(),
                symbol_type: SymbolType::Type,
                type_id: enum_type.type_id,
                is_const: false,
                is_initialized: true,
            });

            for (value_name, _value) in &enum_type.values {
                self.symbol_table.insert_global(Symbol {
                    name: value_name.clone(),
                    symbol_type: SymbolType::EnumVariant,
                    type_id: enum_type.type_id,
                    is_const: true,
                    is_initialized: true,
                });
            }
        }

        // Import global functions
        for (name, func) in &engine_guard.global_functions {
            self.symbol_table.insert_global(Symbol {
                name: name.clone(),
                symbol_type: SymbolType::Function,
                type_id: func.return_type_id,
                is_const: false,
                is_initialized: true,
            });
        }

        // Import global properties
        for (name, prop) in &engine_guard.global_properties {
            self.symbol_table.insert_global(Symbol {
                name: name.clone(),
                symbol_type: SymbolType::Variable,
                type_id: prop.type_id,
                is_const: false,
                is_initialized: true,
            });
        }

        // Import registered types as type symbols
        for (name, obj_type) in &engine_guard.object_types {
            self.symbol_table.insert_global(Symbol {
                name: name.clone(),
                symbol_type: SymbolType::Type,
                type_id: obj_type.type_id,
                is_const: false,
                is_initialized: true,
            });
        }

        for (name, iface_type) in &engine_guard.interface_types {
            self.symbol_table.insert_global(Symbol {
                name: name.clone(),
                symbol_type: SymbolType::Type,
                type_id: iface_type.type_id,
                is_const: false,
                is_initialized: true,
            });
        }
    }

    pub fn analyze(&mut self, script: &Script) -> Result<(), Vec<String>> {
        // First pass: collect all type definitions and global symbols
        self.collect_declarations(script);

        // Second pass: analyze function bodies and validate types
        self.analyze_definitions(script);

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    /// Resolve a type from an AST type definition (public helper)
    pub fn resolve_type_from_ast(&self, type_def: &Type) -> u32 {
        self.resolve_type(type_def)
    }

    /// Look up a type ID by name (public helper)
    pub fn lookup_type_id(&self, name: &str) -> Option<u32> {
        // Check script types first
        if let Some(script_type) = self.script_types.get(name) {
            return Some(script_type.type_id);
        }

        // Check engine types
        let engine_guard = self.engine.read().unwrap();
        engine_guard.get_type_id(name)
    }

    fn collect_declarations(&mut self, script: &Script) {
        for item in &script.items {
            match item {
                ScriptItem::Class(class) => self.collect_class(class),
                ScriptItem::Enum(enum_def) => self.collect_enum(enum_def),
                ScriptItem::Interface(interface) => self.collect_interface(interface),
                ScriptItem::Typedef(typedef) => self.collect_typedef(typedef),
                ScriptItem::Func(func) => self.collect_function(func),
                ScriptItem::FuncDef(funcdef) => self.collect_funcdef(funcdef),
                ScriptItem::Var(var) => self.collect_global_var(var),
                ScriptItem::Namespace(ns) => self.collect_namespace(ns),
                _ => {}
            }
        }
    }

    fn analyze_definitions(&mut self, script: &Script) {
        for item in &script.items {
            match item {
                ScriptItem::Func(func) => self.analyze_function(func),
                ScriptItem::Class(class) => self.analyze_class(class),
                ScriptItem::Var(var) => self.analyze_global_var(var),
                _ => {}
            }
        }
    }

    fn collect_class(&mut self, class: &Class) {
        let type_id = self.next_script_type_id;
        self.next_script_type_id += 1;

        self.symbol_table.insert_global(Symbol {
            name: class.name.clone(),
            symbol_type: SymbolType::Type,
            type_id,
            is_const: false,
            is_initialized: true,
        });

        let mut base_class_ids = Vec::new();
        for base_name in &class.extends {
            if let Some(base_id) = self.lookup_type_id(base_name) {
                base_class_ids.push(base_id);
            } else {
                self.errors.push(format!(
                    "Unknown base class '{}' for class '{}'",
                    base_name, class.name
                ));
            }
        }

        let mut members = Vec::new();
        let mut methods = Vec::new();

        for member in &class.members {
            match member {
                ClassMember::Var(var) => {
                    let member_type_id = self.resolve_type(&var.var_type);
                    for decl in &var.declarations {
                        members.push(MemberInfo {
                            name: decl.name.clone(),
                            type_id: member_type_id,
                            visibility: var.visibility.clone().unwrap_or(Visibility::Private),
                        });
                    }
                }
                ClassMember::Func(func) => {
                    let return_type = func
                        .return_type
                        .as_ref()
                        .map(|t| self.resolve_type(t))
                        .unwrap_or_else(|| self.lookup_type_id("void").unwrap_or(0));

                    let params = func
                        .params
                        .iter()
                        .map(|p| ParamInfo {
                            name: p.name.clone(),
                            type_id: self.resolve_type(&p.param_type),
                            is_ref: matches!(
                                p.type_mod,
                                Some(TypeMod::In) | Some(TypeMod::Out) | Some(TypeMod::InOut)
                            ),
                            is_const: p.param_type.is_const,
                        })
                        .collect();

                    methods.push(MethodInfo {
                        name: func.name.clone(),
                        return_type,
                        params,
                        visibility: func.visibility.clone(),
                        is_const: func.is_const,
                    });
                }
                _ => {}
            }
        }

        self.script_types.insert(
            class.name.clone(),
            ScriptTypeInfo {
                type_id,
                kind: ScriptTypeKind::Class,
                members,
                methods,
                base_classes: base_class_ids,
            },
        );
    }

    fn collect_enum(&mut self, enum_def: &Enum) {
        let type_id = self.next_script_type_id;
        self.next_script_type_id += 1;

        self.symbol_table.insert_global(Symbol {
            name: enum_def.name.clone(),
            symbol_type: SymbolType::Type,
            type_id,
            is_const: false,
            is_initialized: true,
        });

        for variant in &enum_def.variants {
            self.symbol_table.insert_global(Symbol {
                name: variant.name.clone(),
                symbol_type: SymbolType::EnumVariant,
                type_id,
                is_const: true,
                is_initialized: true,
            });
        }

        self.script_types.insert(
            enum_def.name.clone(),
            ScriptTypeInfo {
                type_id,
                kind: ScriptTypeKind::Enum,
                members: Vec::new(),
                methods: Vec::new(),
                base_classes: Vec::new(),
            },
        );
    }

    fn collect_interface(&mut self, interface: &Interface) {
        let type_id = self.next_script_type_id;
        self.next_script_type_id += 1;

        self.symbol_table.insert_global(Symbol {
            name: interface.name.clone(),
            symbol_type: SymbolType::Type,
            type_id,
            is_const: false,
            is_initialized: true,
        });

        self.script_types.insert(
            interface.name.clone(),
            ScriptTypeInfo {
                type_id,
                kind: ScriptTypeKind::Interface,
                members: Vec::new(),
                methods: Vec::new(),
                base_classes: Vec::new(),
            },
        );
    }

    fn collect_typedef(&mut self, typedef: &Typedef) {
        if let Some(base_id) = self.lookup_type_id(&typedef.prim_type) {
            self.symbol_table.insert_global(Symbol {
                name: typedef.name.clone(),
                symbol_type: SymbolType::Type,
                type_id: base_id,
                is_const: false,
                is_initialized: true,
            });
        } else {
            self.errors
                .push(format!("Unknown type '{}' in typedef", typedef.prim_type));
        }
    }

    fn collect_function(&mut self, func: &Func) {
        let return_type = func
            .return_type
            .as_ref()
            .map(|t| self.resolve_type(t))
            .unwrap_or_else(|| self.lookup_type_id("void").unwrap_or(0));

        self.symbol_table.insert_global(Symbol {
            name: func.name.clone(),
            symbol_type: SymbolType::Function,
            type_id: return_type,
            is_const: false,
            is_initialized: true,
        });
    }

    fn collect_funcdef(&mut self, funcdef: &FuncDef) {
        let return_type = self.resolve_type(&funcdef.return_type);

        self.symbol_table.insert_global(Symbol {
            name: funcdef.name.clone(),
            symbol_type: SymbolType::Type,
            type_id: return_type,
            is_const: false,
            is_initialized: true,
        });
    }

    fn collect_global_var(&mut self, var: &Var) {
        let type_id = self.resolve_type(&var.var_type);

        for decl in &var.declarations {
            self.symbol_table.insert_global(Symbol {
                name: decl.name.clone(),
                symbol_type: SymbolType::Variable,
                type_id,
                is_const: var.var_type.is_const,
                is_initialized: decl.initializer.is_some(),
            });
        }
    }

    fn collect_namespace(&mut self, namespace: &Namespace) {
        self.collect_declarations(&Script {
            items: namespace.items.clone(),
        });
    }

    fn resolve_type(&self, type_def: &Type) -> TypeId {
        let base_name = match &type_def.datatype {
            DataType::PrimType(name) => name.clone(),
            DataType::Identifier(name) => name.clone(),
            DataType::Auto => return 0,
            DataType::Question => return 0,
        };

        self.lookup_type_id(&base_name).unwrap_or(0)
    }

    // Stub implementations for analysis methods
    fn analyze_function(&mut self, _func: &Func) {}
    fn analyze_class(&mut self, _class: &Class) {}
    fn analyze_global_var(&mut self, _var: &Var) {}
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope {
                symbols: HashMap::new(),
            }],
            global_scope: HashMap::new(),
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(Scope {
            symbols: HashMap::new(),
        });
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn insert(&mut self, symbol: Symbol) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.symbols.insert(symbol.name.clone(), symbol);
        }
    }

    pub fn insert_global(&mut self, symbol: Symbol) {
        self.global_scope.insert(symbol.name.clone(), symbol);
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.symbols.get(name) {
                return Some(symbol);
            }
        }

        self.global_scope.get(name)
    }

    pub fn lookup_global(&self, name: &str) -> Option<&Symbol> {
        self.global_scope.get(name)
    }

    pub fn exists_in_current_scope(&self, name: &str) -> bool {
        if let Some(scope) = self.scopes.last() {
            scope.symbols.contains_key(name)
        } else {
            false
        }
    }
}
