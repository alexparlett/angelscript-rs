// src/compiler/compiler.rs - Complete AngelScript Bytecode Compiler with HashMap-based memory model

use crate::compiler::bytecode::*;
use crate::compiler::semantic::{ExprContext, SemanticAnalyzer, TypeId};
use crate::core::engine::EngineInner;
use crate::parser::ast::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// Type ID constants (must match semantic.rs)
const TYPE_VOID: TypeId = 0;
const TYPE_BOOL: TypeId = 1;
const TYPE_INT8: TypeId = 2;
const TYPE_INT16: TypeId = 3;
const TYPE_INT32: TypeId = 4;
const TYPE_INT64: TypeId = 5;
const TYPE_UINT8: TypeId = 6;
const TYPE_UINT16: TypeId = 7;
const TYPE_UINT32: TypeId = 8;
const TYPE_UINT64: TypeId = 9;
const TYPE_FLOAT: TypeId = 10;
const TYPE_DOUBLE: TypeId = 11;
const TYPE_STRING: TypeId = 12;
const TYPE_AUTO: TypeId = 13;

/// Code generator that converts AST to AngelScript bytecode
pub struct Compiler {
    /// The bytecode module being built
    module: BytecodeModule,

    /// Current instruction address
    current_address: u32,

    /// Local variables in current function
    local_vars: HashMap<String, LocalVarInfo>,

    /// Next available local variable index
    local_count: u32,

    /// Pool of reusable temporary variables
    temp_var_pool: Vec<u32>,

    /// Semantic analyzer (owned)
    analyzer: SemanticAnalyzer,

    /// Current function being compiled
    current_function: Option<FunctionContext>,

    /// Current class being compiled
    current_class: Option<ClassContext>,

    /// Break jump targets (for loop breaks)
    break_targets: Vec<Vec<u32>>,

    /// Continue jump targets (for loop continues)
    continue_targets: Vec<Vec<u32>>,

    /// Stack of deferred cleanup operations
    cleanup_stack: Vec<CleanupInfo>,

    /// Global variable name to ID mapping
    global_map: HashMap<String, u32>,

    /// Function name to ID mapping
    function_map: HashMap<String, u32>,

    /// Type name to ID mapping
    type_map: HashMap<String, u32>,

    /// Lambda counter for generating unique names
    lambda_count: u32,

    /// Current namespace path
    current_namespace: Vec<String>,

    /// Property accessor map (property_name -> (getter_id, setter_id))
    property_map: HashMap<String, (Option<u32>, Option<u32>)>,

    /// Operator overload map (type_id::op_name -> func_id)
    operator_map: HashMap<String, u32>,

    /// Virtual method table
    vtable: HashMap<TypeId, Vec<u32>>,

    /// Member initializer expressions
    member_initializers: HashMap<String, HashMap<String, Expr>>,
}

#[derive(Debug, Clone)]
struct LocalVarInfo {
    index: u32,
    type_id: TypeId,
    is_ref: bool,
    is_temp: bool,
    needs_cleanup: bool,
}

#[derive(Debug, Clone)]
struct FunctionContext {
    name: String,
    return_type: TypeId,
    param_count: u32,
    start_address: u32,
    max_stack_size: u32,
    has_return: bool,
}

#[derive(Debug, Clone)]
struct ClassContext {
    name: String,
    type_id: TypeId,
    member_types: HashMap<String, TypeId>,
    has_base_class: bool,
    base_class_type: Option<TypeId>,
}

#[derive(Debug, Clone)]
struct CleanupInfo {
    var: u32,
    type_id: TypeId,
    needs_destructor: bool,
}

impl Compiler {
    /// Create a new code generator with an engine
    pub fn new(engine: Arc<RwLock<EngineInner>>) -> Self {
        Self {
            module: BytecodeModule::new(),
            current_address: 0,
            local_vars: HashMap::new(),
            local_count: 0,
            temp_var_pool: Vec::new(),
            analyzer: SemanticAnalyzer::new(engine),
            current_function: None,
            current_class: None,
            break_targets: Vec::new(),
            continue_targets: Vec::new(),
            cleanup_stack: Vec::new(),
            global_map: HashMap::new(),
            function_map: HashMap::new(),
            type_map: HashMap::new(),
            lambda_count: 0,
            current_namespace: Vec::new(),
            property_map: HashMap::new(),
            operator_map: HashMap::new(),
            vtable: HashMap::new(),
            member_initializers: HashMap::new(),
        }
    }

    /// Create from an existing analyzer (for testing)
    pub fn with_analyzer(analyzer: SemanticAnalyzer) -> Self {
        Self {
            module: BytecodeModule::new(),
            current_address: 0,
            local_vars: HashMap::new(),
            local_count: 0,
            temp_var_pool: Vec::new(),
            analyzer,
            current_function: None,
            current_class: None,
            break_targets: Vec::new(),
            continue_targets: Vec::new(),
            cleanup_stack: Vec::new(),
            global_map: HashMap::new(),
            function_map: HashMap::new(),
            type_map: HashMap::new(),
            lambda_count: 0,
            current_namespace: Vec::new(),
            property_map: HashMap::new(),
            operator_map: HashMap::new(),
            vtable: HashMap::new(),
            member_initializers: HashMap::new(),
        }
    }

    /// Generate bytecode from AST (runs semantic analysis first)
    pub fn compile(mut self, script: Script) -> Result<BytecodeModule, CompileError> {
        // Run semantic analysis
        self.analyzer
            .analyze(&script)
            .map_err(|errors| CompileError::SemanticErrors(errors))?;

        // Generate bytecode
        self.generate(script)
            .map_err(|e| CompileError::CodegenError(e))
    }

    /// Generate bytecode from already-analyzed AST
    fn generate(mut self, script: Script) -> Result<BytecodeModule, CodegenError> {
        // First pass: Register all global symbols
        self.register_global_symbols(&script)?;

        // Second pass: Collect member initializers
        self.collect_member_initializers(&script)?;

        // Third pass: Build virtual method tables
        self.build_vtables(&script)?;

        // Fourth pass: Generate code for all items
        for item in &script.items {
            match item {
                ScriptNode::Func(func) => self.generate_function(func)?,
                ScriptNode::Class(class) => self.generate_class(class)?,
                ScriptNode::Var(var) => self.generate_global_var(var)?,
                ScriptNode::Namespace(ns) => self.generate_namespace(ns)?,
                ScriptNode::Enum(enum_def) => self.generate_enum(enum_def)?,
                ScriptNode::Interface(interface) => self.generate_interface(interface)?,
                _ => {}
            }
        }

        Ok(self.module)
    }

    // Helper to access analyzer methods
    fn resolve_type(&self, type_def: &Type) -> TypeId {
        self.analyzer.resolve_type_from_ast(type_def)
    }

    fn lookup_type_id(&self, name: &str) -> Option<TypeId> {
        self.analyzer.lookup_type_id(name)
    }

    fn get_expr_type(&self, expr: &Expr) -> TypeId {
        self.analyzer.get_expr_type(expr)
    }

    // ==================== FIRST PASS: SYMBOL REGISTRATION ====================

    fn register_global_symbols(&mut self, script: &Script) -> Result<(), CodegenError> {
        for item in &script.items {
            match item {
                ScriptNode::Func(func) => {
                    let func_id = self.module.functions.len() as u32;
                    let full_name = self.make_full_name(&func.name);
                    self.function_map.insert(full_name, func_id);
                }
                ScriptNode::Var(var) => {
                    for decl in &var.declarations {
                        let global_id = self.module.globals.len() as u32;
                        let full_name = self.make_full_name(&decl.name);
                        self.global_map.insert(full_name, global_id);
                    }
                }
                ScriptNode::Class(class) => {
                    let type_id = self
                        .analyzer
                        .lookup_type_id(&class.name)
                        .ok_or_else(|| CodegenError::UnknownType(class.name.clone()))?;
                    self.type_map.insert(class.name.clone(), type_id);

                    // Register class methods
                    for member in &class.members {
                        if let ClassMember::Func(func) = member {
                            let func_id = self.module.functions.len() as u32 + 1000;
                            let method_name = format!("{}::{}", class.name, func.name);
                            self.function_map.insert(method_name, func_id);
                        }
                    }
                }
                ScriptNode::Enum(enum_def) => {
                    let type_id = self
                        .analyzer
                        .lookup_type_id(&enum_def.name)
                        .ok_or_else(|| CodegenError::UnknownType(enum_def.name.clone()))?;
                    self.type_map.insert(enum_def.name.clone(), type_id);
                }
                ScriptNode::Namespace(ns) => {
                    let saved_namespace = self.current_namespace.clone();
                    self.current_namespace.extend(ns.name.clone());
                    self.register_global_symbols(&Script {
                        items: ns.items.clone(),
                    })?;
                    self.current_namespace = saved_namespace;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn make_full_name(&self, name: &str) -> String {
        if self.current_namespace.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.current_namespace.join("::"), name)
        }
    }

    // ==================== SECOND PASS: COLLECT MEMBER INITIALIZERS ====================

    fn collect_member_initializers(&mut self, script: &Script) -> Result<(), CodegenError> {
        for item in &script.items {
            if let ScriptNode::Class(class) = item {
                let mut initializers = HashMap::new();

                for member in &class.members {
                    if let ClassMember::Var(var) = member {
                        for decl in &var.declarations {
                            if let Some(VarInit::Expr(expr)) = &decl.initializer {
                                initializers.insert(decl.name.clone(), expr.clone());
                            }
                        }
                    }
                }

                self.member_initializers
                    .insert(class.name.clone(), initializers);
            }
        }
        Ok(())
    }

    // ==================== THIRD PASS: BUILD VIRTUAL METHOD TABLES ====================

    fn build_vtables(&mut self, script: &Script) -> Result<(), CodegenError> {
        for item in &script.items {
            if let ScriptNode::Class(class) = item {
                let type_id = self
                    .type_map
                    .get(&class.name)
                    .copied()
                    .ok_or_else(|| CodegenError::UnknownType(class.name.clone()))?;

                let mut vtable = Vec::new();

                // Collect virtual methods
                for member in &class.members {
                    if let ClassMember::Func(func) = member {
                        if func.modifiers.contains(&"virtual".to_string())
                            || func.modifiers.contains(&"override".to_string())
                        {
                            let method_name = format!("{}::{}", class.name, func.name);
                            if let Some(&func_id) = self.function_map.get(&method_name) {
                                vtable.push(func_id);
                            }
                        }
                    }
                }

                self.vtable.insert(type_id, vtable);
            }
        }
        Ok(())
    }

    // ==================== FUNCTION GENERATION ====================

    fn generate_function(&mut self, func: &Func) -> Result<(), CodegenError> {
        let func_address = self.current_address;

        // Determine return type
        let return_type = func
            .return_type
            .as_ref()
            .map(|t| self.resolve_type(t))
            .unwrap_or(TYPE_VOID);

        // Reset local state
        self.local_vars.clear();
        self.local_count = 0;
        self.temp_var_pool.clear();
        self.cleanup_stack.clear();

        // Setup function context
        self.current_function = Some(FunctionContext {
            name: func.name.clone(),
            return_type,
            param_count: func.params.len() as u32,
            start_address: func_address,
            max_stack_size: 0,
            has_return: false,
        });

        // Allocate space for parameters
        for param in &func.params {
            if let Some(name) = &param.name {
                let param_type = self.resolve_type(&param.param_type);
                let is_ref = matches!(param.type_mod, Some(TypeMod::Out) | Some(TypeMod::InOut));

                let var_info = LocalVarInfo {
                    index: self.local_count,
                    type_id: param_type,
                    is_ref,
                    is_temp: false,
                    needs_cleanup: self.type_needs_cleanup(param_type),
                };

                self.local_vars.insert(name.clone(), var_info);
                self.local_count += 1;
            }
        }

        // Generate function prologue (for constructors)
        if self.is_constructor(func) {
            self.generate_constructor_prologue(&func.name)?;
        }

        // Generate function body
        if let Some(body) = &func.body {
            self.generate_statement_block(body)?;
        }

        // Generate function epilogue (for destructors)
        if self.is_destructor(func) {
            self.generate_destructor_epilogue(&func.name)?;
        }

        // Ensure function returns
        if !self.last_instruction_is_return() {
            if return_type == TYPE_VOID {
                self.emit(Instruction::RET { stack_size: 0 });
            } else {
                self.emit_default_return(return_type);
            }
        }

        // Register function info
        let func_info = FunctionInfo {
            name: self.make_full_name(&func.name),
            address: func_address,
            param_count: func.params.len() as u8,
            local_count: self.local_count,
            stack_size: self.current_function.as_ref().unwrap().max_stack_size,
            return_type,
            is_script_func: true,
        };

        self.module.functions.push(func_info);
        self.current_function = None;

        Ok(())
    }

    fn is_constructor(&self, func: &Func) -> bool {
        if let Some(class_ctx) = &self.current_class {
            func.name == class_ctx.name && func.return_type.is_none()
        } else {
            false
        }
    }

    fn is_destructor(&self, func: &Func) -> bool {
        func.name.starts_with('~')
    }

    fn generate_constructor_prologue(&mut self, class_name: &str) -> Result<(), CodegenError> {
        // Call base class constructor if exists
        if let Some(class_ctx) = &self.current_class {
            if class_ctx.has_base_class {
                if let Some(base_type) = class_ctx.base_class_type {
                    // Push 'this' pointer
                    self.emit(Instruction::PshR);

                    // Call base constructor
                    if let Some(base_ctor_id) = self.find_default_constructor(base_type) {
                        self.emit(Instruction::CALL {
                            func_id: base_ctor_id,
                        });
                    }
                }
            }
        }

        // FIX: Clone the initializers to avoid borrow checker issues
        let initializers = self.member_initializers.get(class_name).cloned();

        if let Some(initializers) = initializers {
            for (member_name, init_expr) in initializers {
                let init_var = self.generate_expr(&init_expr)?;

                // Register property name
                let prop_name_id = self.module.add_property_name(member_name.clone());

                // Set property on 'this' using HashMap-based instruction
                self.emit(Instruction::SetThisProperty {
                    prop_name_id,
                    src_var: init_var,
                });

                if self.is_temp_var(init_var) {
                    self.free_temp(init_var);
                }
            }
        }

        Ok(())
    }

    fn generate_destructor_epilogue(&mut self, _class_name: &str) -> Result<(), CodegenError> {
        // Cleanup is handled by the VM automatically for reference types
        // For value types with destructors, we need to call member destructors

        let member_info: Vec<_> = if let Some(ctx) = &self.current_class {
            ctx.member_types
                .iter()
                .filter_map(|(name, &type_id)| {
                    if self.type_has_destructor(type_id) {
                        Some((name.clone(), type_id))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        for (member_name, member_type) in member_info {
            // Get property
            let prop_name_id = self.module.add_property_name(member_name);
            let temp = self.allocate_temp(member_type);

            self.emit(Instruction::GetThisProperty {
                prop_name_id,
                dst_var: temp,
            });

            // Call destructor
            if let Some(dtor_id) = self.get_destructor_id(member_type) {
                self.emit(Instruction::CALL { func_id: dtor_id });
            }

            self.free_temp(temp);
        }

        Ok(())
    }

    fn find_default_constructor(&self, type_id: TypeId) -> Option<u32> {
        // Look for constructor with no parameters
        if let Some(type_info) = self
            .analyzer
            .script_types
            .values()
            .find(|t| t.type_id == type_id)
        {
            let ctor_name = format!("{}::{}", type_info.name, type_info.name);
            return self.function_map.get(&ctor_name).copied();
        }
        None
    }

    fn type_has_destructor(&self, type_id: TypeId) -> bool {
        self.analyzer
            .script_types
            .values()
            .find(|t| t.type_id == type_id)
            .map(|t| t.has_destructor)
            .unwrap_or(false)
    }

    fn get_destructor_id(&self, type_id: TypeId) -> Option<u32> {
        if let Some(type_info) = self
            .analyzer
            .script_types
            .values()
            .find(|t| t.type_id == type_id)
        {
            let dtor_name = format!("{}::~{}", type_info.name, type_info.name);
            return self.function_map.get(&dtor_name).copied();
        }
        None
    }

    fn emit_default_return(&mut self, return_type: TypeId) {
        let default_value = match return_type {
            TYPE_BOOL => ScriptValue::Bool(false),
            TYPE_INT8 => ScriptValue::Int8(0),
            TYPE_INT16 => ScriptValue::Int16(0),
            TYPE_INT32 => ScriptValue::Int32(0),
            TYPE_INT64 => ScriptValue::Int64(0),
            TYPE_UINT8 => ScriptValue::UInt8(0),
            TYPE_UINT16 => ScriptValue::UInt16(0),
            TYPE_UINT32 => ScriptValue::UInt32(0),
            TYPE_UINT64 => ScriptValue::UInt64(0),
            TYPE_FLOAT => ScriptValue::Float(0.0),
            TYPE_DOUBLE => ScriptValue::Double(0.0),
            _ => ScriptValue::Null,
        };

        self.emit(Instruction::PshC {
            value: default_value,
        });
        self.emit(Instruction::RET { stack_size: 0 });
    }

    fn last_instruction_is_return(&self) -> bool {
        self.module
            .instructions
            .last()
            .map(|instr| matches!(instr, Instruction::RET { .. }))
            .unwrap_or(false)
    }

    // ==================== CLASS GENERATION ====================

    fn generate_class(&mut self, class: &Class) -> Result<(), CodegenError> {
        let type_id = self
            .type_map
            .get(&class.name)
            .copied()
            .ok_or_else(|| CodegenError::UnknownType(class.name.clone()))?;

        let mut member_types = HashMap::new();

        // Collect member types
        for member in &class.members {
            if let ClassMember::Var(var) = member {
                let member_type = self.resolve_type(&var.var_type);

                for decl in &var.declarations {
                    member_types.insert(decl.name.clone(), member_type);
                }
            }
        }

        // Check for base class
        let has_base_class = !class.extends.is_empty();
        let base_class_type = if has_base_class {
            self.type_map.get(&class.extends[0]).copied()
        } else {
            None
        };

        self.current_class = Some(ClassContext {
            name: class.name.clone(),
            type_id,
            member_types,
            has_base_class,
            base_class_type,
        });

        // Generate class members
        for member in &class.members {
            match member {
                ClassMember::Func(func) => {
                    self.generate_function(func)?;
                }
                ClassMember::VirtProp(prop) => {
                    self.generate_virtual_property(prop)?;
                }
                _ => {}
            }
        }

        self.current_class = None;
        Ok(())
    }

    fn generate_virtual_property(&mut self, prop: &VirtProp) -> Result<(), CodegenError> {
        let mut getter_id = None;
        let mut setter_id = None;

        for accessor in &prop.accessors {
            if let Some(body) = &accessor.body {
                let func_name = format!(
                    "{}_{}",
                    prop.name,
                    match accessor.kind {
                        AccessorKind::Get => "get",
                        AccessorKind::Set => "set",
                    }
                );

                let func = Func {
                    modifiers: Vec::new(),
                    visibility: prop.visibility.clone(),
                    return_type: match accessor.kind {
                        AccessorKind::Get => Some(prop.prop_type.clone()),
                        AccessorKind::Set => None,
                    },
                    is_ref: prop.is_ref,
                    name: func_name.clone(),
                    params: match accessor.kind {
                        AccessorKind::Get => Vec::new(),
                        AccessorKind::Set => vec![Param {
                            param_type: prop.prop_type.clone(),
                            type_mod: None,
                            name: Some("value".to_string()),
                            default_value: None,
                            is_variadic: false,
                        }],
                    },
                    is_const: accessor.is_const,
                    attributes: accessor.attributes.clone(),
                    body: Some(body.clone()),
                };

                let func_id = self.module.functions.len() as u32;
                self.generate_function(&func)?;

                match accessor.kind {
                    AccessorKind::Get => getter_id = Some(func_id),
                    AccessorKind::Set => setter_id = Some(func_id),
                }
            }
        }

        self.property_map
            .insert(prop.name.clone(), (getter_id, setter_id));

        Ok(())
    }

    // ==================== GLOBAL VARIABLE GENERATION ====================

    fn generate_global_var(&mut self, var: &Var) -> Result<(), CodegenError> {
        let var_type = self.resolve_type(&var.var_type);

        for decl in &var.declarations {
            let global_id = self.module.globals.len() as u32;

            self.module.globals.push(GlobalVar {
                name: self.make_full_name(&decl.name),
                type_id: var_type,
                address: global_id,
                is_const: var.var_type.is_const,
            });

            if let Some(init) = &decl.initializer {
                match init {
                    VarInit::Expr(expr) => {
                        if let Expr::Literal(lit) = expr {
                            self.generate_global_literal_init(global_id, lit, var_type);
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn generate_global_literal_init(&mut self, global_id: u32, lit: &Literal, type_id: TypeId) {
        let value = match lit {
            Literal::Number(n) => {
                if n.contains('.') {
                    if type_id == TYPE_FLOAT {
                        let val: f32 = n.parse().unwrap_or(0.0);
                        ScriptValue::Float(val)
                    } else {
                        let val: f64 = n.parse().unwrap_or(0.0);
                        ScriptValue::Double(val)
                    }
                } else {
                    if n.ends_with("ll") || n.ends_with("LL") {
                        let val: i64 = n
                            .trim_end_matches(|c| c == 'l' || c == 'L')
                            .parse()
                            .unwrap_or(0);
                        ScriptValue::Int64(val)
                    } else {
                        let val: i32 = n.parse().unwrap_or(0);
                        ScriptValue::Int32(val)
                    }
                }
            }
            Literal::Bool(b) => ScriptValue::Bool(*b),
            Literal::String(s) => ScriptValue::String(s.clone()),
            Literal::Null => ScriptValue::Null,
            Literal::Bits(b) => {
                let val: u32 = u32::from_str_radix(b.trim_start_matches("0x"), 16)
                    .or_else(|_| u32::from_str_radix(b.trim_start_matches("0b"), 2))
                    .unwrap_or(0);
                ScriptValue::UInt32(val)
            }
        };

        self.emit(Instruction::SetG { global_id, value });
    }

    // ==================== NAMESPACE GENERATION ====================

    fn generate_namespace(&mut self, namespace: &Namespace) -> Result<(), CodegenError> {
        let saved_namespace = self.current_namespace.clone();
        self.current_namespace.extend(namespace.name.clone());

        for item in &namespace.items {
            match item {
                ScriptNode::Func(func) => self.generate_function(func)?,
                ScriptNode::Class(class) => self.generate_class(class)?,
                ScriptNode::Var(var) => self.generate_global_var(var)?,
                ScriptNode::Namespace(ns) => self.generate_namespace(ns)?,
                _ => {}
            }
        }

        self.current_namespace = saved_namespace;
        Ok(())
    }

    fn generate_enum(&mut self, _enum_def: &Enum) -> Result<(), CodegenError> {
        // Enums are compile-time only, no runtime code needed
        Ok(())
    }

    fn generate_interface(&mut self, _interface: &Interface) -> Result<(), CodegenError> {
        // Interfaces are compile-time only, no runtime code needed
        Ok(())
    }

    // ==================== STATEMENT GENERATION ====================

    fn generate_statement_block(&mut self, block: &StatBlock) -> Result<(), CodegenError> {
        for stmt in &block.statements {
            self.generate_statement(stmt)?;
        }
        Ok(())
    }

    fn generate_statement(&mut self, stmt: &Statement) -> Result<(), CodegenError> {
        match stmt {
            Statement::Var(var) => self.generate_var_decl(var),
            Statement::Expr(expr) => {
                if let Some(e) = expr {
                    let result_var = self.generate_expr(e)?;
                    if self.is_temp_var(result_var) {
                        self.free_temp(result_var);
                    }
                }
                Ok(())
            }
            Statement::If(if_stmt) => self.generate_if(if_stmt),
            Statement::While(while_stmt) => self.generate_while(while_stmt),
            Statement::DoWhile(do_while) => self.generate_do_while(do_while),
            Statement::For(for_stmt) => self.generate_for(for_stmt),
            Statement::ForEach(foreach) => self.generate_foreach(foreach),
            Statement::Return(ret) => self.generate_return(ret),
            Statement::Break => self.generate_break(),
            Statement::Continue => self.generate_continue(),
            Statement::Switch(switch) => self.generate_switch(switch),
            Statement::Block(block) => self.generate_statement_block(block),
            Statement::Try(try_stmt) => self.generate_try(try_stmt),
            Statement::Using(_) => Ok(()),
        }
    }

    fn generate_var_decl(&mut self, var: &Var) -> Result<(), CodegenError> {
        let var_type = self.resolve_type(&var.var_type);

        for decl in &var.declarations {
            let local_idx = self.local_count;

            let cleanup = self.type_needs_cleanup(var_type);

            let var_info = LocalVarInfo {
                index: local_idx,
                type_id: var_type,
                is_ref: false,
                is_temp: false,
                needs_cleanup: cleanup,
            };

            self.local_vars.insert(decl.name.clone(), var_info);
            self.local_count += 1;

            if let Some(init) = &decl.initializer {
                match init {
                    VarInit::Expr(expr) => {
                        let result_var = self.generate_expr(expr)?;
                        self.emit(Instruction::CpyV {
                            dst: local_idx,
                            src: result_var,
                        });
                        if self.is_temp_var(result_var) {
                            self.free_temp(result_var);
                        }
                    }
                    VarInit::InitList(init_list) => {
                        // FIX 1: Use generate_init_list_expr and handle the result
                        self.generate_init_list_expr(init_list)?;
                        // Pop the init list from value stack to local variable
                        self.emit(Instruction::PopR);
                        self.emit(Instruction::CpyRtoV { var: local_idx });
                    }
                    VarInit::ArgList(args) => {
                        self.generate_constructor_call(local_idx, var_type, args)?;
                    }
                }
            } else {
                self.emit_default_init(local_idx, var_type);
            }

            // Track for cleanup if needed
            if cleanup {
                self.cleanup_stack.push(CleanupInfo {
                    var: local_idx,
                    type_id: var_type,
                    needs_destructor: self.type_has_destructor(var_type),
                });
            }
        }
        Ok(())
    }

    fn type_needs_cleanup(&self, type_id: TypeId) -> bool {
        if self.is_primitive_type(type_id) {
            return false;
        }

        // Check if it's a reference type that needs cleanup
        self.analyzer
            .script_types
            .values()
            .find(|t| t.type_id == type_id)
            .map(|t| t.is_ref_type || t.has_destructor)
            .unwrap_or(false)
    }

    fn is_primitive_type(&self, type_id: TypeId) -> bool {
        type_id <= TYPE_AUTO
    }

    fn emit_default_init(&mut self, var: u32, type_id: TypeId) {
        let value = match type_id {
            TYPE_BOOL => ScriptValue::Bool(false),
            TYPE_INT8 => ScriptValue::Int8(0),
            TYPE_INT16 => ScriptValue::Int16(0),
            TYPE_INT32 => ScriptValue::Int32(0),
            TYPE_INT64 => ScriptValue::Int64(0),
            TYPE_UINT8 => ScriptValue::UInt8(0),
            TYPE_UINT16 => ScriptValue::UInt16(0),
            TYPE_UINT32 => ScriptValue::UInt32(0),
            TYPE_UINT64 => ScriptValue::UInt64(0),
            TYPE_FLOAT => ScriptValue::Float(0.0),
            TYPE_DOUBLE => ScriptValue::Double(0.0),
            _ => ScriptValue::Null,
        };

        self.emit(Instruction::SetV { var, value });
    }

    // ==================== CONTROL FLOW GENERATION ====================

    fn generate_if(&mut self, if_stmt: &IfStmt) -> Result<(), CodegenError> {
        let cond_var = self.generate_expr(&if_stmt.condition)?;

        if self.is_comparison_expr(&if_stmt.condition) {
            self.emit(Instruction::TNZ);
        } else {
            let cond_type = self.get_expr_type(&if_stmt.condition);
            self.emit_compare_zero(cond_var, cond_type);
        }

        let jump_to_else = self.emit_jump_placeholder(Instruction::JZ { offset: 0 });

        self.generate_statement(&if_stmt.then_branch)?;

        if let Some(else_branch) = &if_stmt.else_branch {
            let jump_to_end = self.emit_jump_placeholder(Instruction::JMP { offset: 0 });
            self.patch_jump(jump_to_else);
            self.generate_statement(else_branch)?;
            self.patch_jump(jump_to_end);
        } else {
            self.patch_jump(jump_to_else);
        }

        if self.is_temp_var(cond_var) {
            self.free_temp(cond_var);
        }

        Ok(())
    }

    fn generate_while(&mut self, while_stmt: &WhileStmt) -> Result<(), CodegenError> {
        let loop_start = self.current_address;

        self.break_targets.push(Vec::new());
        self.continue_targets.push(Vec::new());

        let cond_var = self.generate_expr(&while_stmt.condition)?;

        if self.is_comparison_expr(&while_stmt.condition) {
            self.emit(Instruction::TNZ);
        } else {
            let cond_type = self.get_expr_type(&while_stmt.condition);
            self.emit_compare_zero(cond_var, cond_type);
        }

        let jump_to_end = self.emit_jump_placeholder(Instruction::JZ { offset: 0 });

        self.generate_statement(&while_stmt.body)?;

        let offset = (loop_start as i32) - (self.current_address as i32);
        self.emit(Instruction::JMP { offset });

        self.patch_jump(jump_to_end);

        if let Some(breaks) = self.break_targets.pop() {
            for break_addr in breaks {
                self.patch_jump_at(break_addr);
            }
        }

        if let Some(continues) = self.continue_targets.pop() {
            for continue_addr in continues {
                let offset = (loop_start as i32) - (continue_addr as i32);
                if let Some(instr) = self.module.instructions.get_mut(continue_addr as usize) {
                    *instr = Instruction::JMP { offset };
                }
            }
        }

        if self.is_temp_var(cond_var) {
            self.free_temp(cond_var);
        }

        Ok(())
    }

    fn generate_do_while(&mut self, do_while: &DoWhileStmt) -> Result<(), CodegenError> {
        let loop_start = self.current_address;

        self.break_targets.push(Vec::new());
        self.continue_targets.push(Vec::new());

        self.generate_statement(&do_while.body)?;

        let cond_var = self.generate_expr(&do_while.condition)?;

        if self.is_comparison_expr(&do_while.condition) {
            self.emit(Instruction::TNZ);
        } else {
            let cond_type = self.get_expr_type(&do_while.condition);
            self.emit_compare_zero(cond_var, cond_type);
        }

        let offset = (loop_start as i32) - (self.current_address as i32);
        self.emit(Instruction::JNZ { offset });

        if let Some(breaks) = self.break_targets.pop() {
            for break_addr in breaks {
                self.patch_jump_at(break_addr);
            }
        }

        if let Some(continues) = self.continue_targets.pop() {
            let continue_target = self.current_address - 2;
            for continue_addr in continues {
                let offset = (continue_target as i32) - (continue_addr as i32);
                if let Some(instr) = self.module.instructions.get_mut(continue_addr as usize) {
                    *instr = Instruction::JMP { offset };
                }
            }
        }

        if self.is_temp_var(cond_var) {
            self.free_temp(cond_var);
        }

        Ok(())
    }

    fn generate_for(&mut self, for_stmt: &ForStmt) -> Result<(), CodegenError> {
        match &for_stmt.init {
            ForInit::Var(var) => self.generate_var_decl(var)?,
            ForInit::Expr(Some(expr)) => {
                let result = self.generate_expr(expr)?;
                if self.is_temp_var(result) {
                    self.free_temp(result);
                }
            }
            ForInit::Expr(None) => {}
        }

        let loop_start = self.current_address;

        self.break_targets.push(Vec::new());
        self.continue_targets.push(Vec::new());

        let jump_to_end = if let Some(condition) = &for_stmt.condition {
            let cond_var = self.generate_expr(condition)?;

            if self.is_comparison_expr(condition) {
                self.emit(Instruction::TNZ);
            } else {
                let cond_type = self.get_expr_type(condition);
                self.emit_compare_zero(cond_var, cond_type);
            }

            if self.is_temp_var(cond_var) {
                self.free_temp(cond_var);
            }

            Some(self.emit_jump_placeholder(Instruction::JZ { offset: 0 }))
        } else {
            None
        };

        self.generate_statement(&for_stmt.body)?;

        let continue_target = self.current_address;

        for increment in &for_stmt.increment {
            let result = self.generate_expr(increment)?;
            if self.is_temp_var(result) {
                self.free_temp(result);
            }
        }

        let offset = (loop_start as i32) - (self.current_address as i32);
        self.emit(Instruction::JMP { offset });

        if let Some(end_jump) = jump_to_end {
            self.patch_jump(end_jump);
        }

        if let Some(breaks) = self.break_targets.pop() {
            for break_addr in breaks {
                self.patch_jump_at(break_addr);
            }
        }

        if let Some(continues) = self.continue_targets.pop() {
            for continue_addr in continues {
                let offset = (continue_target as i32) - (continue_addr as i32);
                if let Some(instr) = self.module.instructions.get_mut(continue_addr as usize) {
                    *instr = Instruction::JMP { offset };
                }
            }
        }

        Ok(())
    }

    fn generate_foreach(&mut self, foreach: &ForEachStmt) -> Result<(), CodegenError> {
        // Generate foreach using AngelScript's foreach protocol
        let collection_var = self.generate_expr(&foreach.iterable)?;
        let collection_type = self.get_expr_type(&foreach.iterable);

        // Allocate iterator variable
        let iterator_var = self.allocate_temp(TYPE_INT32);

        // Call opForBegin()
        self.emit(Instruction::PshV {
            var: collection_var,
        });
        if let Some(begin_id) = self.find_operator_method(collection_type, "opForBegin") {
            self.emit(Instruction::CALL { func_id: begin_id });
            self.emit(Instruction::PopR);
            self.emit(Instruction::CpyRtoV { var: iterator_var });
        }

        let loop_start = self.current_address;

        self.break_targets.push(Vec::new());
        self.continue_targets.push(Vec::new());

        // Call opForEnd(iterator)
        self.emit(Instruction::PshV {
            var: collection_var,
        });
        self.emit(Instruction::PshV { var: iterator_var });
        if let Some(end_id) = self.find_operator_method(collection_type, "opForEnd") {
            self.emit(Instruction::CALL { func_id: end_id });
            self.emit(Instruction::PopR);
            self.emit(Instruction::TNZ);
        }

        let jump_to_end = self.emit_jump_placeholder(Instruction::JNZ { offset: 0 });

        // Get current value - opForValue(iterator)
        for (var_type, var_name) in &foreach.variables {
            let value_var = self.allocate_temp(self.resolve_type(var_type));

            self.emit(Instruction::PshV {
                var: collection_var,
            });
            self.emit(Instruction::PshV { var: iterator_var });
            if let Some(value_id) = self.find_operator_method(collection_type, "opForValue") {
                self.emit(Instruction::CALL { func_id: value_id });
                self.emit(Instruction::PopR);
                self.emit(Instruction::CpyRtoV { var: value_var });
            }

            // Register loop variable
            let var_info = LocalVarInfo {
                index: value_var,
                type_id: self.resolve_type(var_type),
                is_ref: false,
                is_temp: false,
                needs_cleanup: false,
            };
            self.local_vars.insert(var_name.clone(), var_info);
        }

        // Execute body
        self.generate_statement(&foreach.body)?;

        // Call opForNext(iterator)
        self.emit(Instruction::PshV {
            var: collection_var,
        });
        self.emit(Instruction::PshV { var: iterator_var });
        if let Some(next_id) = self.find_operator_method(collection_type, "opForNext") {
            self.emit(Instruction::CALL { func_id: next_id });
            self.emit(Instruction::PopR);
            self.emit(Instruction::CpyRtoV { var: iterator_var });
        }

        let offset = (loop_start as i32) - (self.current_address as i32);
        self.emit(Instruction::JMP { offset });

        self.patch_jump(jump_to_end);

        if let Some(breaks) = self.break_targets.pop() {
            for break_addr in breaks {
                self.patch_jump_at(break_addr);
            }
        }

        if let Some(continues) = self.continue_targets.pop() {
            for continue_addr in continues {
                let offset = (loop_start as i32) - (continue_addr as i32);
                if let Some(instr) = self.module.instructions.get_mut(continue_addr as usize) {
                    *instr = Instruction::JMP { offset };
                }
            }
        }

        if self.is_temp_var(collection_var) {
            self.free_temp(collection_var);
        }
        self.free_temp(iterator_var);

        Ok(())
    }

    fn find_operator_method(&self, type_id: TypeId, op_name: &str) -> Option<u32> {
        if let Some(type_info) = self
            .analyzer
            .script_types
            .values()
            .find(|t| t.type_id == type_id)
        {
            let method_name = format!("{}::{}", type_info.name, op_name);
            return self.function_map.get(&method_name).copied();
        }
        None
    }

    fn generate_return(&mut self, ret: &ReturnStmt) -> Result<(), CodegenError> {
        if let Some(value) = &ret.value {
            let result_var = self.generate_expr(value)?;
            self.emit(Instruction::CpyVtoR { var: result_var });

            if self.is_temp_var(result_var) {
                self.free_temp(result_var);
            }
        }

        // Cleanup local variables before return
        self.emit_cleanup_before_return();

        self.emit(Instruction::RET { stack_size: 0 });

        if let Some(func_ctx) = &mut self.current_function {
            func_ctx.has_return = true;
        }

        Ok(())
    }

    fn emit_cleanup_before_return(&mut self) {
        // Collect cleanup items first
        let cleanups: Vec<_> = self
            .cleanup_stack
            .iter()
            .filter(|c| c.needs_destructor)
            .map(|c| (c.var, c.type_id))
            .collect();

        // Process cleanups in reverse order
        for (var, type_id) in cleanups.into_iter().rev() {
            if let Some(dtor_id) = self.get_destructor_id(type_id) {
                self.emit(Instruction::LoadObj { var });
                self.emit(Instruction::CALL { func_id: dtor_id });
            }
        }
    }

    fn generate_break(&mut self) -> Result<(), CodegenError> {
        let jump_addr = self.emit_jump_placeholder(Instruction::JMP { offset: 0 });

        if let Some(breaks) = self.break_targets.last_mut() {
            breaks.push(jump_addr);
            Ok(())
        } else {
            Err(CodegenError::InvalidBreak)
        }
    }

    fn generate_continue(&mut self) -> Result<(), CodegenError> {
        let jump_addr = self.emit_jump_placeholder(Instruction::JMP { offset: 0 });

        if let Some(continues) = self.continue_targets.last_mut() {
            continues.push(jump_addr);
            Ok(())
        } else {
            Err(CodegenError::InvalidContinue)
        }
    }

    fn generate_switch(&mut self, switch: &SwitchStmt) -> Result<(), CodegenError> {
        let switch_var = self.generate_expr(&switch.value)?;
        let switch_type = self.get_expr_type(&switch.value);

        self.break_targets.push(Vec::new());

        let mut case_jumps = Vec::new();
        let mut default_jump = None;

        for case in &switch.cases {
            match &case.pattern {
                CasePattern::Value(expr) => {
                    let case_var = self.generate_expr(expr)?;

                    self.emit_comparison(switch_var, case_var, switch_type);
                    self.emit(Instruction::TZ);

                    let jump_addr = self.emit_jump_placeholder(Instruction::JNZ { offset: 0 });
                    case_jumps.push(jump_addr);

                    if self.is_temp_var(case_var) {
                        self.free_temp(case_var);
                    }
                }
                CasePattern::Default => {
                    default_jump = Some(self.emit_jump_placeholder(Instruction::JMP { offset: 0 }));
                }
            }
        }

        let jump_to_end = self.emit_jump_placeholder(Instruction::JMP { offset: 0 });

        let mut case_idx = 0;
        for case in &switch.cases {
            if matches!(case.pattern, CasePattern::Value(_)) {
                self.patch_jump(case_jumps[case_idx]);
                case_idx += 1;
            } else if let Some(default_addr) = default_jump {
                self.patch_jump(default_addr);
            }

            for stmt in &case.statements {
                self.generate_statement(stmt)?;
            }
        }

        self.patch_jump(jump_to_end);

        if let Some(breaks) = self.break_targets.pop() {
            for break_addr in breaks {
                self.patch_jump_at(break_addr);
            }
        }

        if self.is_temp_var(switch_var) {
            self.free_temp(switch_var);
        }

        Ok(())
    }

    fn generate_try(&mut self, try_stmt: &TryStmt) -> Result<(), CodegenError> {
        // AngelScript doesn't have full exception support in bytecode
        // This is a simplified implementation
        self.generate_statement_block(&try_stmt.try_block)?;
        self.generate_statement_block(&try_stmt.catch_block)?;
        Ok(())
    }

    // ==================== EXPRESSION GENERATION ====================

    fn generate_expr(&mut self, expr: &Expr) -> Result<u32, CodegenError> {
        // Get expression context from analyzer
        let ctx = self
            .analyzer
            .analyze_expr_context(expr)
            .map_err(|e| CodegenError::SemanticError(e.message))?;

        match expr {
            Expr::Literal(lit) => self.generate_literal(lit),
            Expr::VarAccess(scope, name) => self.generate_var_access(scope, name),
            Expr::Binary(left, op, right) => self.generate_binary(left, op, right, &ctx),
            Expr::Unary(op, operand) => self.generate_unary(op, operand),
            Expr::Postfix(expr, op) => self.generate_postfix(expr, op),
            Expr::Ternary(cond, then_expr, else_expr) => {
                self.generate_ternary(cond, then_expr, else_expr)
            }
            Expr::FuncCall(call) => self.generate_func_call(call),
            Expr::ConstructCall(type_def, args) => self.generate_construct_call(type_def, args),
            Expr::Cast(target_type, expr) => self.generate_cast(target_type, expr),
            Expr::Lambda(lambda) => self.generate_lambda(lambda),
            Expr::InitList(init_list) => {
                let temp = self.allocate_temp(TYPE_VOID);
                self.generate_init_list_expr(init_list)?;
                Ok(temp)
            }
            Expr::Void => {
                let temp = self.allocate_temp(TYPE_VOID);
                Ok(temp)
            }
        }
    }

    fn generate_literal(&mut self, lit: &Literal) -> Result<u32, CodegenError> {
        let (value, type_id) = match lit {
            Literal::Bool(b) => (ScriptValue::Bool(*b), TYPE_BOOL),
            Literal::Number(n) => {
                if n.contains('.') || n.contains('e') || n.contains('E') {
                    if n.ends_with('f') || n.ends_with('F') {
                        let val: f32 = n
                            .trim_end_matches(|c| c == 'f' || c == 'F')
                            .parse()
                            .unwrap_or(0.0);
                        (ScriptValue::Float(val), TYPE_FLOAT)
                    } else {
                        let val: f64 = n.parse().unwrap_or(0.0);
                        (ScriptValue::Double(val), TYPE_DOUBLE)
                    }
                } else {
                    if n.ends_with("ll") || n.ends_with("LL") {
                        let val: i64 = n
                            .trim_end_matches(|c| c == 'l' || c == 'L')
                            .parse()
                            .unwrap_or(0);
                        (ScriptValue::Int64(val), TYPE_INT64)
                    } else {
                        let val: i32 = n.parse().unwrap_or(0);
                        (ScriptValue::Int32(val), TYPE_INT32)
                    }
                }
            }
            Literal::String(s) => {
                let str_id = self.module.add_string(s.clone());
                let temp = self.allocate_temp(TYPE_STRING);
                self.emit(Instruction::Str { str_id });
                self.emit(Instruction::PopR);
                self.emit(Instruction::CpyRtoV { var: temp });
                return Ok(temp);
            }
            Literal::Null => (ScriptValue::Null, TYPE_VOID),
            Literal::Bits(b) => {
                let val: u32 = u32::from_str_radix(b.trim_start_matches("0x"), 16)
                    .or_else(|_| u32::from_str_radix(b.trim_start_matches("0b"), 2))
                    .unwrap_or(0);
                (ScriptValue::UInt32(val), TYPE_UINT32)
            }
        };

        let temp = self.allocate_temp(type_id);
        self.emit(Instruction::SetV { var: temp, value });
        Ok(temp)
    }

    fn generate_var_access(&mut self, _scope: &Scope, name: &str) -> Result<u32, CodegenError> {
        if let Some(var_info) = self.local_vars.get(name) {
            return Ok(var_info.index);
        }

        if let Some(&global_id) = self.global_map.get(name) {
            let temp = self.allocate_temp(TYPE_INT32);
            self.emit(Instruction::CpyGtoV {
                var: temp,
                global_id,
            });
            return Ok(temp);
        }

        Err(CodegenError::UndefinedVariable(name.to_string()))
    }

    fn generate_binary(
        &mut self,
        left: &Expr,
        op: &BinaryOp,
        right: &Expr,
        ctx: &ExprContext,
    ) -> Result<u32, CodegenError> {
        // Handle assignments specially
        if matches!(op, BinaryOp::Assign) {
            return self.generate_assignment(left, op, right);
        }

        // Handle compound assignments
        if matches!(
            op,
            BinaryOp::AddAssign
                | BinaryOp::SubAssign
                | BinaryOp::MulAssign
                | BinaryOp::DivAssign
                | BinaryOp::ModAssign
                | BinaryOp::PowAssign
                | BinaryOp::BitAndAssign
                | BinaryOp::BitOrAssign
                | BinaryOp::BitXorAssign
                | BinaryOp::ShlAssign
                | BinaryOp::ShrAssign
                | BinaryOp::UShrAssign
        ) {
            return self.generate_assignment(left, op, right);
        }

        // Handle logical operators with short-circuit evaluation
        if matches!(op, BinaryOp::And | BinaryOp::Or) {
            return self.generate_logical_op(left, op, right);
        }

        // Use context information for better code generation
        let left_var = self.generate_expr(left)?;
        let right_var = self.generate_expr(right)?;

        let result_var = self.allocate_temp(ctx.result_type);

        // Emit appropriate instruction based on context
        self.emit_binary_op(op, result_var, left_var, right_var, ctx.result_type)?;

        if self.is_temp_var(left_var) {
            self.free_temp(left_var);
        }
        if self.is_temp_var(right_var) {
            self.free_temp(right_var);
        }

        Ok(result_var)
    }

    fn emit_binary_op(
        &mut self,
        op: &BinaryOp,
        dst: u32,
        a: u32,
        b: u32,
        type_id: TypeId,
    ) -> Result<(), CodegenError> {
        let instr = match (op, type_id) {
            (BinaryOp::Add, TYPE_INT32 | TYPE_UINT32) => Instruction::ADDi { dst, a, b },
            (BinaryOp::Sub, TYPE_INT32 | TYPE_UINT32) => Instruction::SUBi { dst, a, b },
            (BinaryOp::Mul, TYPE_INT32 | TYPE_UINT32) => Instruction::MULi { dst, a, b },
            (BinaryOp::Div, TYPE_INT32 | TYPE_UINT32) => Instruction::DIVi { dst, a, b },
            (BinaryOp::Mod, TYPE_INT32 | TYPE_UINT32) => Instruction::MODi { dst, a, b },
            (BinaryOp::Pow, TYPE_INT32) => Instruction::POWi { dst, a, b },

            (BinaryOp::Add, TYPE_FLOAT) => Instruction::ADDf { dst, a, b },
            (BinaryOp::Sub, TYPE_FLOAT) => Instruction::SUBf { dst, a, b },
            (BinaryOp::Mul, TYPE_FLOAT) => Instruction::MULf { dst, a, b },
            (BinaryOp::Div, TYPE_FLOAT) => Instruction::DIVf { dst, a, b },
            (BinaryOp::Mod, TYPE_FLOAT) => Instruction::MODf { dst, a, b },
            (BinaryOp::Pow, TYPE_FLOAT) => Instruction::POWf { dst, a, b },

            (BinaryOp::Add, TYPE_DOUBLE) => Instruction::ADDd { dst, a, b },
            (BinaryOp::Sub, TYPE_DOUBLE) => Instruction::SUBd { dst, a, b },
            (BinaryOp::Mul, TYPE_DOUBLE) => Instruction::MULd { dst, a, b },
            (BinaryOp::Div, TYPE_DOUBLE) => Instruction::DIVd { dst, a, b },
            (BinaryOp::Mod, TYPE_DOUBLE) => Instruction::MODd { dst, a, b },
            (BinaryOp::Pow, TYPE_DOUBLE) => Instruction::POWd { dst, a, b },

            (BinaryOp::Add, TYPE_INT64 | TYPE_UINT64) => Instruction::ADDi64 { dst, a, b },
            (BinaryOp::Sub, TYPE_INT64 | TYPE_UINT64) => Instruction::SUBi64 { dst, a, b },
            (BinaryOp::Mul, TYPE_INT64 | TYPE_UINT64) => Instruction::MULi64 { dst, a, b },
            (BinaryOp::Div, TYPE_INT64 | TYPE_UINT64) => Instruction::DIVi64 { dst, a, b },
            (BinaryOp::Mod, TYPE_INT64 | TYPE_UINT64) => Instruction::MODi64 { dst, a, b },
            (BinaryOp::Pow, TYPE_INT64) => Instruction::POWi64 { dst, a, b },

            (BinaryOp::BitAnd, TYPE_INT32 | TYPE_UINT32) => Instruction::BAND { dst, a, b },
            (BinaryOp::BitOr, TYPE_INT32 | TYPE_UINT32) => Instruction::BOR { dst, a, b },
            (BinaryOp::BitXor, TYPE_INT32 | TYPE_UINT32) => Instruction::BXOR { dst, a, b },
            (BinaryOp::Shl, TYPE_INT32 | TYPE_UINT32) => Instruction::BSLL {
                dst,
                val: a,
                shift: b,
            },
            (BinaryOp::Shr, TYPE_INT32) => Instruction::BSRA {
                dst,
                val: a,
                shift: b,
            },
            (BinaryOp::Shr, TYPE_UINT32) => Instruction::BSRL {
                dst,
                val: a,
                shift: b,
            },
            (BinaryOp::UShr, TYPE_INT32 | TYPE_UINT32) => Instruction::BSRL {
                dst,
                val: a,
                shift: b,
            },

            (BinaryOp::BitAnd, TYPE_INT64 | TYPE_UINT64) => Instruction::BAND64 { dst, a, b },
            (BinaryOp::BitOr, TYPE_INT64 | TYPE_UINT64) => Instruction::BOR64 { dst, a, b },
            (BinaryOp::BitXor, TYPE_INT64 | TYPE_UINT64) => Instruction::BXOR64 { dst, a, b },
            (BinaryOp::Shl, TYPE_INT64 | TYPE_UINT64) => Instruction::BSLL64 {
                dst,
                val: a,
                shift: b,
            },
            (BinaryOp::Shr, TYPE_INT64) => Instruction::BSRA64 {
                dst,
                val: a,
                shift: b,
            },
            (BinaryOp::Shr, TYPE_UINT64) => Instruction::BSRL64 {
                dst,
                val: a,
                shift: b,
            },
            (BinaryOp::UShr, TYPE_INT64 | TYPE_UINT64) => Instruction::BSRL64 {
                dst,
                val: a,
                shift: b,
            },

            (
                BinaryOp::Eq
                | BinaryOp::Ne
                | BinaryOp::Lt
                | BinaryOp::Le
                | BinaryOp::Gt
                | BinaryOp::Ge,
                _,
            ) => {
                self.emit_comparison(a, b, type_id);
                self.emit_comparison_test(op);
                self.emit(Instruction::CpyRtoV { var: dst });
                return Ok(());
            }

            _ => {
                return Err(CodegenError::UnsupportedOperation(format!(
                    "{:?} on type {}",
                    op, type_id
                )));
            }
        };

        self.emit(instr);
        Ok(())
    }

    fn emit_comparison(&mut self, a: u32, b: u32, type_id: TypeId) {
        let instr = match type_id {
            TYPE_INT32 | TYPE_INT16 | TYPE_INT8 => Instruction::CMPi { a, b },
            TYPE_UINT32 | TYPE_UINT16 | TYPE_UINT8 => Instruction::CMPu { a, b },
            TYPE_FLOAT => Instruction::CMPf { a, b },
            TYPE_DOUBLE => Instruction::CMPd { a, b },
            TYPE_INT64 => Instruction::CMPi64 { a, b },
            TYPE_UINT64 => Instruction::CMPu64 { a, b },
            _ => Instruction::CmpPtr { a, b },
        };
        self.emit(instr);
    }

    fn emit_comparison_test(&mut self, op: &BinaryOp) {
        let instr = match op {
            BinaryOp::Eq => Instruction::TZ,
            BinaryOp::Ne => Instruction::TNZ,
            BinaryOp::Lt => Instruction::TS,
            BinaryOp::Le => Instruction::TNP,
            BinaryOp::Gt => Instruction::TP,
            BinaryOp::Ge => Instruction::TNS,
            _ => Instruction::Nop,
        };
        self.emit(instr);
    }

    fn generate_assignment(
        &mut self,
        left: &Expr,
        op: &BinaryOp,
        right: &Expr,
    ) -> Result<u32, CodegenError> {
        // Handle member access assignment
        if let Expr::Postfix(obj, PostfixOp::MemberAccess(member)) = left {
            let obj_var = self.generate_expr(obj)?;
            let right_var = self.generate_expr(right)?;

            // Register property name and get its ID
            let prop_name_id = self.module.add_property_name(member.clone());

            if matches!(op, BinaryOp::Assign) {
                // Simple assignment
                self.emit(Instruction::SetProperty {
                    obj_var,
                    prop_name_id,
                    src_var: right_var,
                });
            } else {
                // Compound assignment (+=, -=, etc.)
                let temp = self.allocate_temp(TYPE_INT32);

                // Get current value
                self.emit(Instruction::GetProperty {
                    obj_var,
                    prop_name_id,
                    dst_var: temp,
                });

                // Perform operation
                let result_type = self.get_expr_type(left);
                self.emit_compound_assignment_op(op, temp, temp, right_var, result_type)?;

                // Set new value
                self.emit(Instruction::SetProperty {
                    obj_var,
                    prop_name_id,
                    src_var: temp,
                });

                self.free_temp(temp);
            }

            if self.is_temp_var(obj_var) {
                self.free_temp(obj_var);
            }
            if self.is_temp_var(right_var) {
                self.free_temp(right_var);
            }

            return Ok(right_var);
        }

        // Handle regular variable assignment
        let lvalue = match left {
            Expr::VarAccess(_scope, name) => {
                if let Some(var_info) = self.local_vars.get(name) {
                    var_info.index
                } else if let Some(&global_id) = self.global_map.get(name) {
                    let right_var = self.generate_expr(right)?;

                    if matches!(op, BinaryOp::Assign) {
                        self.emit(Instruction::CpyVtoG {
                            global_id,
                            var: right_var,
                        });
                    } else {
                        let temp = self.allocate_temp(TYPE_INT32);
                        self.emit(Instruction::CpyGtoV {
                            var: temp,
                            global_id,
                        });

                        let result_type = self.get_expr_type(left);
                        self.emit_compound_assignment_op(op, temp, temp, right_var, result_type)?;

                        self.emit(Instruction::CpyVtoG {
                            global_id,
                            var: temp,
                        });
                        self.free_temp(temp);
                    }

                    if self.is_temp_var(right_var) {
                        self.free_temp(right_var);
                    }
                    return Ok(right_var);
                } else {
                    return Err(CodegenError::UndefinedVariable(name.to_string()));
                }
            }
            _ => {
                return Err(CodegenError::InvalidLValue);
            }
        };

        let right_var = self.generate_expr(right)?;

        if matches!(op, BinaryOp::Assign) {
            self.emit(Instruction::CpyV {
                dst: lvalue,
                src: right_var,
            });
        } else {
            let result_type = self.get_expr_type(left);
            self.emit_compound_assignment_op(op, lvalue, lvalue, right_var, result_type)?;
        }

        if self.is_temp_var(right_var) {
            self.free_temp(right_var);
        }

        Ok(lvalue)
    }

    fn emit_compound_assignment_op(
        &mut self,
        op: &BinaryOp,
        dst: u32,
        left: u32,
        right: u32,
        type_id: TypeId,
    ) -> Result<(), CodegenError> {
        let base_op = match op {
            BinaryOp::AddAssign => BinaryOp::Add,
            BinaryOp::SubAssign => BinaryOp::Sub,
            BinaryOp::MulAssign => BinaryOp::Mul,
            BinaryOp::DivAssign => BinaryOp::Div,
            BinaryOp::ModAssign => BinaryOp::Mod,
            BinaryOp::PowAssign => BinaryOp::Pow,
            BinaryOp::BitAndAssign => BinaryOp::BitAnd,
            BinaryOp::BitOrAssign => BinaryOp::BitOr,
            BinaryOp::BitXorAssign => BinaryOp::BitXor,
            BinaryOp::ShlAssign => BinaryOp::Shl,
            BinaryOp::ShrAssign => BinaryOp::Shr,
            BinaryOp::UShrAssign => BinaryOp::UShr,
            _ => return Err(CodegenError::UnsupportedOperation(format!("{:?}", op))),
        };

        self.emit_binary_op(&base_op, dst, left, right, type_id)
    }

    fn generate_logical_op(
        &mut self,
        left: &Expr,
        op: &BinaryOp,
        right: &Expr,
    ) -> Result<u32, CodegenError> {
        let result = self.allocate_temp(TYPE_BOOL);

        let left_var = self.generate_expr(left)?;
        let left_type = self.get_expr_type(left);

        self.emit_compare_zero(left_var, left_type);

        match op {
            BinaryOp::And => {
                self.emit(Instruction::TZ);
                let short_circuit = self.emit_jump_placeholder(Instruction::JNZ { offset: 0 });

                let right_var = self.generate_expr(right)?;
                let right_type = self.get_expr_type(right);
                self.emit_compare_zero(right_var, right_type);

                if self.is_temp_var(right_var) {
                    self.free_temp(right_var);
                }

                self.patch_jump(short_circuit);
                self.emit(Instruction::CpyRtoV { var: result });
            }
            BinaryOp::Or => {
                self.emit(Instruction::TNZ);
                let short_circuit = self.emit_jump_placeholder(Instruction::JNZ { offset: 0 });

                let right_var = self.generate_expr(right)?;
                let right_type = self.get_expr_type(right);
                self.emit_compare_zero(right_var, right_type);

                if self.is_temp_var(right_var) {
                    self.free_temp(right_var);
                }

                self.patch_jump(short_circuit);
                self.emit(Instruction::CpyRtoV { var: result });
            }
            _ => unreachable!(),
        }

        if self.is_temp_var(left_var) {
            self.free_temp(left_var);
        }

        Ok(result)
    }

    fn generate_unary(&mut self, op: &UnaryOp, operand: &Expr) -> Result<u32, CodegenError> {
        let operand_var = self.generate_expr(operand)?;
        let operand_type = self.get_expr_type(operand);
        let result = self.allocate_temp(operand_type);

        self.emit(Instruction::CpyV {
            dst: result,
            src: operand_var,
        });

        let instr = match (op, operand_type) {
            (UnaryOp::Neg, TYPE_INT32 | TYPE_UINT32) => Instruction::NEGi { var: result },
            (UnaryOp::Neg, TYPE_FLOAT) => Instruction::NEGf { var: result },
            (UnaryOp::Neg, TYPE_DOUBLE) => Instruction::NEGd { var: result },
            (UnaryOp::Neg, TYPE_INT64 | TYPE_UINT64) => Instruction::NEGi64 { var: result },

            (UnaryOp::Not, _) => Instruction::NOT { var: result },
            (UnaryOp::BitNot, TYPE_INT32 | TYPE_UINT32) => Instruction::BNOT { var: result },
            (UnaryOp::BitNot, TYPE_INT64 | TYPE_UINT64) => Instruction::BNOT64 { var: result },

            (UnaryOp::PreInc, TYPE_INT8) => Instruction::INCi8 { var: result },
            (UnaryOp::PreInc, TYPE_INT16) => Instruction::INCi16 { var: result },
            (UnaryOp::PreInc, TYPE_INT32 | TYPE_UINT32) => Instruction::INCi { var: result },
            (UnaryOp::PreInc, TYPE_INT64 | TYPE_UINT64) => Instruction::INCi64 { var: result },
            (UnaryOp::PreInc, TYPE_FLOAT) => Instruction::INCf { var: result },
            (UnaryOp::PreInc, TYPE_DOUBLE) => Instruction::INCd { var: result },

            (UnaryOp::PreDec, TYPE_INT8) => Instruction::DECi8 { var: result },
            (UnaryOp::PreDec, TYPE_INT16) => Instruction::DECi16 { var: result },
            (UnaryOp::PreDec, TYPE_INT32 | TYPE_UINT32) => Instruction::DECi { var: result },
            (UnaryOp::PreDec, TYPE_INT64 | TYPE_UINT64) => Instruction::DECi64 { var: result },
            (UnaryOp::PreDec, TYPE_FLOAT) => Instruction::DECf { var: result },
            (UnaryOp::PreDec, TYPE_DOUBLE) => Instruction::DECd { var: result },

            (UnaryOp::Handle, _) => {
                // Handle operator @ - creates a handle to the object
                return Ok(result);
            }

            (UnaryOp::Plus, _) => {
                // Unary plus is a no-op
                return Ok(result);
            }

            _ => {
                return Err(CodegenError::UnsupportedOperation(format!(
                    "{:?} on type {}",
                    op, operand_type
                )));
            }
        };

        self.emit(instr);

        if self.is_temp_var(operand_var) {
            self.free_temp(operand_var);
        }

        Ok(result)
    }

    fn generate_postfix(&mut self, expr: &Expr, op: &PostfixOp) -> Result<u32, CodegenError> {
        match op {
            PostfixOp::PostInc | PostfixOp::PostDec => {
                let var = self.generate_expr(expr)?;
                let expr_type = self.get_expr_type(expr);

                let result = self.allocate_temp(expr_type);
                self.emit(Instruction::CpyV {
                    dst: result,
                    src: var,
                });

                let instr = match (op, expr_type) {
                    (PostfixOp::PostInc, TYPE_INT8) => Instruction::INCi8 { var },
                    (PostfixOp::PostInc, TYPE_INT16) => Instruction::INCi16 { var },
                    (PostfixOp::PostInc, TYPE_INT32 | TYPE_UINT32) => Instruction::INCi { var },
                    (PostfixOp::PostInc, TYPE_INT64 | TYPE_UINT64) => Instruction::INCi64 { var },
                    (PostfixOp::PostInc, TYPE_FLOAT) => Instruction::INCf { var },
                    (PostfixOp::PostInc, TYPE_DOUBLE) => Instruction::INCd { var },

                    (PostfixOp::PostDec, TYPE_INT8) => Instruction::DECi8 { var },
                    (PostfixOp::PostDec, TYPE_INT16) => Instruction::DECi16 { var },
                    (PostfixOp::PostDec, TYPE_INT32 | TYPE_UINT32) => Instruction::DECi { var },
                    (PostfixOp::PostDec, TYPE_INT64 | TYPE_UINT64) => Instruction::DECi64 { var },
                    (PostfixOp::PostDec, TYPE_FLOAT) => Instruction::DECf { var },
                    (PostfixOp::PostDec, TYPE_DOUBLE) => Instruction::DECd { var },

                    _ => Instruction::Nop,
                };

                self.emit(instr);

                if self.is_temp_var(var) {
                    self.free_temp(var);
                }

                Ok(result)
            }

            PostfixOp::MemberAccess(member) => self.generate_member_access(expr, member),

            PostfixOp::MemberCall(call) => self.generate_method_call(expr, call),

            PostfixOp::Index(indices) => self.generate_index_access(expr, indices),

            PostfixOp::Call(args) => self.generate_functor_call(expr, args),
        }
    }

    fn generate_member_access(&mut self, obj: &Expr, member: &str) -> Result<u32, CodegenError> {
        let obj_var = self.generate_expr(obj)?;
        let obj_type = self.get_expr_type(obj);

        // Get member type
        let member_type = self.get_member_type(obj_type, member)?;

        // Register property name and get its ID
        let prop_name_id = self.module.add_property_name(member.to_string());

        // Allocate temp for result
        let result = self.allocate_temp(member_type);

        // Emit GetProperty instruction
        self.emit(Instruction::GetProperty {
            obj_var,
            prop_name_id,
            dst_var: result,
        });

        if self.is_temp_var(obj_var) {
            self.free_temp(obj_var);
        }

        Ok(result)
    }

    fn get_member_type(&self, obj_type: TypeId, member: &str) -> Result<TypeId, CodegenError> {
        if let Some(type_info) = self
            .analyzer
            .script_types
            .values()
            .find(|t| t.type_id == obj_type)
        {
            if let Some(member_info) = type_info.members.get(member) {
                return Ok(member_info.type_id);
            }
        }

        Err(CodegenError::UndefinedMember(member.to_string()))
    }

    fn generate_method_call(&mut self, obj: &Expr, call: &FuncCall) -> Result<u32, CodegenError> {
        // Push 'this' pointer
        let obj_var = self.generate_expr(obj)?;
        self.emit(Instruction::PshV { var: obj_var });

        // Push arguments (right to left)
        for arg in call.args.iter().rev() {
            let arg_var = self.generate_expr(&arg.value)?;
            self.emit(Instruction::PshV { var: arg_var });

            if self.is_temp_var(arg_var) {
                self.free_temp(arg_var);
            }
        }

        // Get method ID
        let obj_type = self.get_expr_type(obj);
        let method_id = self
            .find_method_id(obj_type, &call.name)
            .ok_or_else(|| CodegenError::UndefinedFunction(call.name.clone()))?;

        // Call method
        self.emit(Instruction::CALL { func_id: method_id });

        // Get return value
        let return_type = self.get_method_return_type(obj_type, &call.name);
        let result = self.allocate_temp(return_type);

        if return_type != TYPE_VOID {
            self.emit(Instruction::PopR);
            self.emit(Instruction::CpyRtoV { var: result });
        }

        if self.is_temp_var(obj_var) {
            self.free_temp(obj_var);
        }

        Ok(result)
    }

    fn find_method_id(&self, type_id: TypeId, method_name: &str) -> Option<u32> {
        // Look up method in function map
        if let Some(type_info) = self
            .analyzer
            .script_types
            .values()
            .find(|t| t.type_id == type_id)
        {
            let full_name = format!("{}::{}", type_info.name, method_name);
            return self.function_map.get(&full_name).copied();
        }
        None
    }

    fn get_method_return_type(&self, type_id: TypeId, method_name: &str) -> TypeId {
        // Query semantic analyzer for method return type
        if let Some(type_info) = self
            .analyzer
            .script_types
            .values()
            .find(|t| t.type_id == type_id)
        {
            if let Some(methods) = type_info.methods.get(method_name) {
                if let Some(method) = methods.first() {
                    return method.return_type;
                }
            }
        }
        TYPE_VOID
    }

    fn generate_index_access(
        &mut self,
        array: &Expr,
        indices: &[IndexArg],
    ) -> Result<u32, CodegenError> {
        let array_var = self.generate_expr(array)?;
        let array_type = self.get_expr_type(array);

        // For now, support single index
        if indices.len() != 1 {
            return Err(CodegenError::NotImplemented(
                "multi-dimensional indexing".to_string(),
            ));
        }

        let index_var = self.generate_expr(&indices[0].value)?;

        // Check if type has opIndex operator
        if let Some(op_index_id) = self.find_operator_method(array_type, "opIndex") {
            // Call opIndex method
            self.emit(Instruction::PshV { var: array_var });
            self.emit(Instruction::PshV { var: index_var });
            self.emit(Instruction::CALL {
                func_id: op_index_id,
            });

            let result = self.allocate_temp(TYPE_INT32);
            self.emit(Instruction::PopR);
            self.emit(Instruction::CpyRtoV { var: result });

            if self.is_temp_var(array_var) {
                self.free_temp(array_var);
            }
            if self.is_temp_var(index_var) {
                self.free_temp(index_var);
            }

            return Ok(result);
        }

        Err(CodegenError::NotImplemented(
            "built-in array indexing".to_string(),
        ))
    }

    fn generate_functor_call(&mut self, functor: &Expr, args: &[Arg]) -> Result<u32, CodegenError> {
        let functor_var = self.generate_expr(functor)?;
        let functor_type = self.get_expr_type(functor);

        // Check if type has opCall operator
        if let Some(op_call_id) = self.find_operator_method(functor_type, "opCall") {
            // Push functor object
            self.emit(Instruction::PshV { var: functor_var });

            // Push arguments
            for arg in args.iter().rev() {
                let arg_var = self.generate_expr(&arg.value)?;
                self.emit(Instruction::PshV { var: arg_var });

                if self.is_temp_var(arg_var) {
                    self.free_temp(arg_var);
                }
            }

            // Call opCall
            self.emit(Instruction::CALL {
                func_id: op_call_id,
            });

            let result = self.allocate_temp(TYPE_INT32);
            self.emit(Instruction::PopR);
            self.emit(Instruction::CpyRtoV { var: result });

            if self.is_temp_var(functor_var) {
                self.free_temp(functor_var);
            }

            return Ok(result);
        }

        Err(CodegenError::NotImplemented("functor call".to_string()))
    }

    fn generate_ternary(
        &mut self,
        cond: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
    ) -> Result<u32, CodegenError> {
        let result_type = self.get_expr_type(then_expr);
        let result = self.allocate_temp(result_type);

        let cond_var = self.generate_expr(cond)?;
        let cond_type = self.get_expr_type(cond);

        self.emit_compare_zero(cond_var, cond_type);
        let jump_to_else = self.emit_jump_placeholder(Instruction::JZ { offset: 0 });

        let then_var = self.generate_expr(then_expr)?;
        self.emit(Instruction::CpyV {
            dst: result,
            src: then_var,
        });
        if self.is_temp_var(then_var) {
            self.free_temp(then_var);
        }

        let jump_to_end = self.emit_jump_placeholder(Instruction::JMP { offset: 0 });

        self.patch_jump(jump_to_else);
        let else_var = self.generate_expr(else_expr)?;
        self.emit(Instruction::CpyV {
            dst: result,
            src: else_var,
        });
        if self.is_temp_var(else_var) {
            self.free_temp(else_var);
        }

        self.patch_jump(jump_to_end);

        if self.is_temp_var(cond_var) {
            self.free_temp(cond_var);
        }

        Ok(result)
    }

    fn generate_func_call(&mut self, call: &FuncCall) -> Result<u32, CodegenError> {
        // Push arguments (right to left)
        for arg in call.args.iter().rev() {
            let arg_var = self.generate_expr(&arg.value)?;
            self.emit(Instruction::PshV { var: arg_var });

            if self.is_temp_var(arg_var) {
                self.free_temp(arg_var);
            }
        }

        // Look up function
        let full_name = self.make_full_name(&call.name);
        if let Some(&func_id) = self.function_map.get(&full_name) {
            self.emit(Instruction::CALL { func_id });
        } else {
            // Check if it's an engine-registered function
            let sys_func = {
                let engine = self.analyzer.engine.read().unwrap();
                engine
                    .global_functions
                    .iter()
                    .position(|f| f.name == call.name)
            };

            if let Some(sys_func_id) = sys_func {
                self.emit(Instruction::CALLSYS {
                    sys_func_id: sys_func_id as u32,
                });
            } else {
                return Err(CodegenError::UndefinedFunction(call.name.clone()));
            }
        }

        // Get return value
        let return_type = self.lookup_type_id(&call.name).unwrap_or(TYPE_VOID);
        let result = self.allocate_temp(return_type);

        if return_type != TYPE_VOID {
            self.emit(Instruction::PopR);
            self.emit(Instruction::CpyRtoV { var: result });
        }

        Ok(result)
    }

    fn generate_construct_call(
        &mut self,
        type_def: &Type,
        args: &[Arg],
    ) -> Result<u32, CodegenError> {
        let type_id = self.resolve_type(type_def);

        let result = self.allocate_temp(type_id);

        // Find constructor
        let constructor_id = self
            .find_constructor(type_id, args.len())
            .ok_or_else(|| CodegenError::UndefinedFunction("constructor".to_string()))?;

        // Allocate object
        self.emit(Instruction::Alloc {
            type_id,
            func_id: constructor_id,
        });

        // Push arguments
        for arg in args.iter().rev() {
            let arg_var = self.generate_expr(&arg.value)?;
            self.emit(Instruction::PshV { var: arg_var });

            if self.is_temp_var(arg_var) {
                self.free_temp(arg_var);
            }
        }

        // Call constructor
        self.emit(Instruction::CALL {
            func_id: constructor_id,
        });

        // Store result
        self.emit(Instruction::StoreObj { var: result });

        Ok(result)
    }

    fn find_constructor(&self, type_id: TypeId, arg_count: usize) -> Option<u32> {
        if let Some(type_info) = self
            .analyzer
            .script_types
            .values()
            .find(|t| t.type_id == type_id)
        {
            if let Some(constructors) = type_info.methods.get(&type_info.name) {
                for ctor in constructors {
                    if ctor.params.len() == arg_count {
                        let ctor_name = format!("{}::{}", type_info.name, type_info.name);
                        return self.function_map.get(&ctor_name).copied();
                    }
                }
            }
        }
        None
    }

    fn generate_cast(&mut self, target_type: &Type, expr: &Expr) -> Result<u32, CodegenError> {
        let source_var = self.generate_expr(expr)?;
        let source_type = self.get_expr_type(expr);
        let target_type_id = self.resolve_type(target_type);

        let result = self.allocate_temp(target_type_id);

        self.emit(Instruction::CpyV {
            dst: result,
            src: source_var,
        });
        self.emit_type_conversion(result, source_type, target_type_id);

        if self.is_temp_var(source_var) {
            self.free_temp(source_var);
        }

        Ok(result)
    }

    fn emit_type_conversion(&mut self, var: u32, from_type: TypeId, to_type: TypeId) {
        if from_type == to_type {
            return;
        }

        let instr = match (from_type, to_type) {
            (TYPE_INT32, TYPE_INT8) => Instruction::iTOb { var },
            (TYPE_INT32, TYPE_INT16) => Instruction::iTOw { var },
            (TYPE_INT32, TYPE_FLOAT) => Instruction::iTOf { var },
            (TYPE_INT32, TYPE_DOUBLE) => Instruction::iTOd { var },
            (TYPE_INT32, TYPE_INT64) => Instruction::iTOi64 { var },

            (TYPE_INT8, TYPE_INT32) => Instruction::sbTOi { var },
            (TYPE_INT16, TYPE_INT32) => Instruction::swTOi { var },
            (TYPE_UINT8, TYPE_INT32) => Instruction::ubTOi { var },
            (TYPE_UINT16, TYPE_INT32) => Instruction::uwTOi { var },

            (TYPE_FLOAT, TYPE_INT32) => Instruction::fTOi { var },
            (TYPE_FLOAT, TYPE_DOUBLE) => Instruction::fTOd { var },
            (TYPE_FLOAT, TYPE_INT64) => Instruction::fTOi64 { var },
            (TYPE_FLOAT, TYPE_UINT64) => Instruction::fTOu64 { var },

            (TYPE_DOUBLE, TYPE_INT32) => Instruction::dTOi { var },
            (TYPE_DOUBLE, TYPE_FLOAT) => Instruction::dTOf { var },
            (TYPE_DOUBLE, TYPE_INT64) => Instruction::dTOi64 { var },
            (TYPE_DOUBLE, TYPE_UINT64) => Instruction::dTOu64 { var },

            (TYPE_INT64, TYPE_INT32) => Instruction::i64TOi { var },
            (TYPE_INT64, TYPE_FLOAT) => Instruction::i64TOf { var },
            (TYPE_INT64, TYPE_DOUBLE) => Instruction::i64TOd { var },

            (TYPE_UINT64, TYPE_FLOAT) => Instruction::u64TOf { var },
            (TYPE_UINT64, TYPE_DOUBLE) => Instruction::u64TOd { var },

            _ => return,
        };

        self.emit(instr);
    }

    fn generate_lambda(&mut self, lambda: &Lambda) -> Result<u32, CodegenError> {
        // Generate a unique name for the lambda
        let lambda_name = format!("$lambda_{}", self.lambda_count);
        self.lambda_count += 1;

        // Create a function from the lambda
        let lambda_func = Func {
            modifiers: vec![],
            visibility: None,
            return_type: None,
            is_ref: false,
            name: lambda_name.clone(),
            params: lambda
                .params
                .iter()
                .map(|p| Param {
                    param_type: p.param_type.clone().unwrap_or_else(|| Type {
                        is_const: false,
                        scope: Scope {
                            is_global: false,
                            path: vec![],
                        },
                        datatype: DataType::Auto,
                        template_types: vec![],
                        modifiers: vec![],
                    }),
                    type_mod: p.type_mod.clone(),
                    name: p.name.clone(),
                    default_value: None,
                    is_variadic: false,
                })
                .collect(),
            is_const: false,
            attributes: vec![],
            body: Some(lambda.body.clone()),
        };

        // Generate the lambda function
        self.generate_function(&lambda_func)?;

        // Return a function pointer
        let func_id = self
            .function_map
            .get(&lambda_name)
            .copied()
            .ok_or_else(|| CodegenError::UndefinedFunction(lambda_name.clone()))?;

        let result = self.allocate_temp(TYPE_INT32);
        self.emit(Instruction::FuncPtr { func_id });
        self.emit(Instruction::PopR);
        self.emit(Instruction::CpyRtoV { var: result });

        Ok(result)
    }

    fn generate_init_list_expr(&mut self, init_list: &InitList) -> Result<(), CodegenError> {
        // Begin init list
        self.emit(Instruction::BeginInitList);

        // Add each element
        for item in &init_list.items {
            match item {
                InitListItem::Expr(expr) => {
                    let item_var = self.generate_expr(expr)?;

                    // Push value onto value stack
                    self.emit(Instruction::PshV { var: item_var });

                    // Add to init list (pops from value stack)
                    self.emit(Instruction::AddToInitList);

                    if self.is_temp_var(item_var) {
                        self.free_temp(item_var);
                    }
                }
                InitListItem::InitList(nested) => {
                    self.generate_init_list_expr(nested)?;
                }
            }
        }

        // End init list
        self.emit(Instruction::EndInitList {
            element_type: TYPE_VOID,
            count: init_list.items.len() as u32,
        });

        Ok(())
    }

    fn generate_constructor_call(
        &mut self,
        var: u32,
        type_id: TypeId,
        args: &[Arg],
    ) -> Result<(), CodegenError> {
        // Find constructor
        let constructor_id = self
            .find_constructor(type_id, args.len())
            .ok_or_else(|| CodegenError::UndefinedFunction("constructor".to_string()))?;

        // Allocate object at variable location
        self.emit(Instruction::Alloc {
            type_id,
            func_id: constructor_id,
        });

        // Push arguments
        for arg in args.iter().rev() {
            let arg_var = self.generate_expr(&arg.value)?;
            self.emit(Instruction::PshV { var: arg_var });

            if self.is_temp_var(arg_var) {
                self.free_temp(arg_var);
            }
        }

        // Call constructor
        self.emit(Instruction::CALL {
            func_id: constructor_id,
        });

        // Store result
        self.emit(Instruction::StoreObj { var });

        Ok(())
    }

    // ==================== HELPER METHODS ====================

    fn emit(&mut self, instr: Instruction) -> u32 {
        let addr = self.current_address;
        self.module.instructions.push(instr);
        self.current_address += 1;
        addr
    }

    fn emit_jump_placeholder(&mut self, instr: Instruction) -> u32 {
        self.emit(instr)
    }

    fn patch_jump(&mut self, addr: u32) {
        self.patch_jump_at(addr);
    }

    fn patch_jump_at(&mut self, addr: u32) {
        let target = self.current_address;
        let offset = (target as i32) - (addr as i32);

        if let Some(instr) = self.module.instructions.get_mut(addr as usize) {
            match instr {
                Instruction::JMP { offset: o } => *o = offset,
                Instruction::JZ { offset: o } => *o = offset,
                Instruction::JNZ { offset: o } => *o = offset,
                Instruction::JS { offset: o } => *o = offset,
                Instruction::JNS { offset: o } => *o = offset,
                Instruction::JP { offset: o } => *o = offset,
                Instruction::JNP { offset: o } => *o = offset,
                _ => {}
            }
        }
    }

    fn allocate_temp(&mut self, type_id: TypeId) -> u32 {
        if let Some(var) = self.temp_var_pool.pop() {
            var
        } else {
            let var = self.local_count;
            self.local_count += 1;

            let var_info = LocalVarInfo {
                index: var,
                type_id,
                is_ref: false,
                is_temp: true,
                needs_cleanup: false,
            };

            self.local_vars.insert(format!("$temp{}", var), var_info);
            var
        }
    }

    fn free_temp(&mut self, var: u32) {
        if self.is_temp_var(var) {
            self.temp_var_pool.push(var);
        }
    }

    fn is_temp_var(&self, var: u32) -> bool {
        self.local_vars
            .values()
            .any(|info| info.index == var && info.is_temp)
    }

    fn emit_compare_zero(&mut self, var: u32, type_id: TypeId) {
        match type_id {
            TYPE_INT32 | TYPE_UINT32 => {
                self.emit(Instruction::CMPIi { var, imm: 0 });
            }
            TYPE_FLOAT => {
                self.emit(Instruction::CMPIf { var, imm: 0.0 });
            }
            _ => {
                self.emit(Instruction::CMPIi { var, imm: 0 });
            }
        }
    }

    fn is_comparison_expr(&self, expr: &Expr) -> bool {
        matches!(
            expr,
            Expr::Binary(
                _,
                BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Lt
                    | BinaryOp::Le
                    | BinaryOp::Gt
                    | BinaryOp::Ge,
                _
            )
        )
    }

    fn current_frame(&self) -> &HashMap<String, LocalVarInfo> {
        &self.local_vars
    }
}

// ==================== ERROR TYPES ====================

#[derive(Debug, Clone)]
pub enum CodegenError {
    UndefinedVariable(String),
    UndefinedFunction(String),
    UndefinedMember(String),
    UnknownType(String),
    UnsupportedOperation(String),
    InvalidLValue,
    InvalidBreak,
    InvalidContinue,
    NotImplemented(String),
    SemanticError(String),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CodegenError::UndefinedVariable(name) => {
                write!(f, "Undefined variable: {}", name)
            }
            CodegenError::UndefinedFunction(name) => {
                write!(f, "Undefined function: {}", name)
            }
            CodegenError::UndefinedMember(name) => {
                write!(f, "Undefined member: {}", name)
            }
            CodegenError::UnknownType(name) => {
                write!(f, "Unknown type: {}", name)
            }
            CodegenError::UnsupportedOperation(op) => {
                write!(f, "Unsupported operation: {}", op)
            }
            CodegenError::InvalidLValue => {
                write!(f, "Invalid left-hand side of assignment")
            }
            CodegenError::InvalidBreak => {
                write!(f, "Break statement outside of loop")
            }
            CodegenError::InvalidContinue => {
                write!(f, "Continue statement outside of loop")
            }
            CodegenError::NotImplemented(feature) => {
                write!(f, "Not yet implemented: {}", feature)
            }
            CodegenError::SemanticError(msg) => {
                write!(f, "Semantic error: {}", msg)
            }
        }
    }
}

impl std::error::Error for CodegenError {}

#[derive(Debug, Clone)]
pub enum CompileError {
    SemanticErrors(Vec<crate::compiler::semantic::SemanticError>),
    CodegenError(CodegenError),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CompileError::SemanticErrors(errors) => {
                writeln!(
                    f,
                    "Semantic analysis failed with {} error(s):",
                    errors.len()
                )?;
                for error in errors {
                    writeln!(f, "  - {}", error.message)?;
                }
                Ok(())
            }
            CompileError::CodegenError(error) => {
                write!(f, "Code generation failed: {}", error)
            }
        }
    }
}

impl std::error::Error for CompileError {}

// src/compiler/tests.rs - Comprehensive compiler unit tests

// src/compiler/tests.rs - Comprehensive tests with both unit and integration tests

#[cfg(test)]
mod tests {
    // ==================== TEST HELPERS ====================

    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};
    use std::sync::atomic::AtomicU32;
    use crate::{Compiler, SemanticAnalyzer};
    use crate::compiler::bytecode::{Instruction, ScriptValue};
    use crate::compiler::compiler::TYPE_INT32;
    use crate::core::engine::EngineInner;
    use crate::parser::ast::{BinaryOp, DataType, Expr, Func, IfStmt, Literal, ReturnStmt, Scope, Script, ScriptNode, StatBlock, Statement, Type, UnaryOp, Var, VarDecl, VarInit, WhileStmt};

    /// Create a minimal engine for testing
    fn create_test_engine() -> Arc<RwLock<EngineInner>> {
        Arc::new(RwLock::new(EngineInner {
            object_types: HashMap::new(),
            enum_types: HashMap::new(),
            interface_types: HashMap::new(),
            funcdefs: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            next_type_id: AtomicU32::new(100),
            modules: HashMap::new(),
        }))
    }

    /// Create a test compiler with pre-configured analyzer
    fn create_test_compiler() -> Compiler {
        let engine = create_test_engine();
        let analyzer = SemanticAnalyzer::new(engine);
        Compiler::with_analyzer(analyzer)
    }

    /// Helper to build an int type
    fn build_int_type() -> Type {
        Type {
            is_const: false,
            scope: Scope {
                is_global: false,
                path: vec![],
            },
            datatype: DataType::PrimType("int".to_string()),
            template_types: vec![],
            modifiers: vec![],
        }
    }

    /// Helper to build a simple expression
    fn build_int_literal(value: i32) -> Expr {
        Expr::Literal(Literal::Number(value.to_string()))
    }

    fn build_var_access(name: &str) -> Expr {
        Expr::VarAccess(
            Scope {
                is_global: false,
                path: vec![],
            },
            name.to_string(),
        )
    }

    fn build_binary(left: Expr, op: BinaryOp, right: Expr) -> Expr {
        Expr::Binary(Box::new(left), op, Box::new(right))
    }

    /// Helper to build a function
    fn build_function(name: &str, return_type: Option<Type>, body: StatBlock) -> Func {
        Func {
            modifiers: vec![],
            visibility: None,
            return_type,
            is_ref: false,
            name: name.to_string(),
            params: vec![],
            is_const: false,
            attributes: vec![],
            body: Some(body),
        }
    }

    // ==================== UNIT TESTS (with manual semantic analysis) ====================

    #[test]
    fn test_int_literal() {
        let mut compiler = create_test_compiler();

        let expr = build_int_literal(42);

        // Run semantic analysis first
        compiler.analyzer.analyze_expr_context(&expr).unwrap();

        // Now generate code
        let result = compiler.generate_expr(&expr).unwrap();

        assert!(compiler.is_temp_var(result));

        let instructions = &compiler.module.instructions;
        assert!(matches!(
            instructions.last(),
            Some(Instruction::SetV {
                var: _,
                value: ScriptValue::Int32(42)
            })
        ));
    }

    #[test]
    fn test_bool_literal() {
        let mut compiler = create_test_compiler();

        let expr = Expr::Literal(Literal::Bool(true));

        compiler.analyzer.analyze_expr_context(&expr).unwrap();

        let result = compiler.generate_expr(&expr).unwrap();

        assert!(compiler.is_temp_var(result));

        let instructions = &compiler.module.instructions;
        assert!(matches!(
            instructions.last(),
            Some(Instruction::SetV {
                var: _,
                value: ScriptValue::Bool(true)
            })
        ));
    }

    #[test]
    fn test_string_literal() {
        let mut compiler = create_test_compiler();

        let expr = Expr::Literal(Literal::String("Hello".to_string()));

        compiler.analyzer.analyze_expr_context(&expr).unwrap();

        let result = compiler.generate_expr(&expr).unwrap();

        assert!(compiler.is_temp_var(result));

        // Should have added string to string table
        assert_eq!(compiler.module.strings.len(), 1);
        assert_eq!(compiler.module.strings[0], "Hello");
    }

    #[test]
    fn test_addition() {
        let mut compiler = create_test_compiler();

        let expr = build_binary(build_int_literal(10), BinaryOp::Add, build_int_literal(20));

        // Run semantic analysis
        compiler.analyzer.analyze_expr_context(&expr).unwrap();

        let result = compiler.generate_expr(&expr).unwrap();
        assert!(compiler.is_temp_var(result));

        let instructions = &compiler.module.instructions;
        assert!(
            instructions
                .iter()
                .any(|i| matches!(i, Instruction::ADDi { .. }))
        );
    }

    // First, let's add a debug helper to see what's actually generated
    #[test]
    fn test_debug_equality() {
        let mut compiler = create_test_compiler();

        let expr = build_binary(
            build_int_literal(5),
            BinaryOp::Eq,
            build_int_literal(5),
        );

        compiler.analyzer.analyze_expr_context(&expr).unwrap();
        let result = compiler.generate_expr(&expr).unwrap();

        // Print all instructions to see what's generated
        println!("Generated instructions:");
        for (i, instr) in compiler.module.instructions.iter().enumerate() {
            println!("{}: {:?}", i, instr);
        }

        assert!(compiler.is_temp_var(result));
    }

    #[test]
    fn test_subtraction() {
        let mut compiler = create_test_compiler();

        let expr = build_binary(build_int_literal(30), BinaryOp::Sub, build_int_literal(10));

        compiler.analyzer.analyze_expr_context(&expr).unwrap();

        let result = compiler.generate_expr(&expr).unwrap();
        assert!(compiler.is_temp_var(result));

        let instructions = &compiler.module.instructions;
        assert!(
            instructions
                .iter()
                .any(|i| matches!(i, Instruction::SUBi { .. }))
        );
    }

    #[test]
    fn test_multiplication() {
        let mut compiler = create_test_compiler();

        let expr = build_binary(build_int_literal(5), BinaryOp::Mul, build_int_literal(6));

        compiler.analyzer.analyze_expr_context(&expr).unwrap();

        let result = compiler.generate_expr(&expr).unwrap();
        assert!(compiler.is_temp_var(result));

        let instructions = &compiler.module.instructions;
        assert!(
            instructions
                .iter()
                .any(|i| matches!(i, Instruction::MULi { .. }))
        );
    }

    #[test]
    fn test_division() {
        let mut compiler = create_test_compiler();

        let expr = build_binary(build_int_literal(20), BinaryOp::Div, build_int_literal(4));

        compiler.analyzer.analyze_expr_context(&expr).unwrap();

        let result = compiler.generate_expr(&expr).unwrap();
        assert!(compiler.is_temp_var(result));

        let instructions = &compiler.module.instructions;
        assert!(
            instructions
                .iter()
                .any(|i| matches!(i, Instruction::DIVi { .. }))
        );
    }

    #[test]
    fn test_equality() {
        let mut compiler = create_test_compiler();

        let expr = build_binary(build_int_literal(5), BinaryOp::Eq, build_int_literal(5));

        compiler.analyzer.analyze_expr_context(&expr).unwrap();

        let result = compiler.generate_expr(&expr).unwrap();
        assert!(compiler.is_temp_var(result));

        let instructions = &compiler.module.instructions;
        assert!(
            instructions
                .iter()
                .any(|i| matches!(i, Instruction::CMPi { .. }))
        );
        assert!(instructions.iter().any(|i| matches!(i, Instruction::TZ)));
    }

    #[test]
    fn test_less_than() {
        let mut compiler = create_test_compiler();

        let expr = build_binary(build_int_literal(5), BinaryOp::Lt, build_int_literal(10));

        compiler.analyzer.analyze_expr_context(&expr).unwrap();

        let result = compiler.generate_expr(&expr).unwrap();
        assert!(compiler.is_temp_var(result));

        let instructions = &compiler.module.instructions;
        assert!(
            instructions
                .iter()
                .any(|i| matches!(i, Instruction::CMPi { .. }))
        );
        assert!(instructions.iter().any(|i| matches!(i, Instruction::TS)));
    }

    #[test]
    fn test_negation() {
        let mut compiler = create_test_compiler();

        let expr = Expr::Unary(UnaryOp::Neg, Box::new(build_int_literal(42)));

        compiler.analyzer.analyze_expr_context(&expr).unwrap();

        let result = compiler.generate_expr(&expr).unwrap();
        assert!(compiler.is_temp_var(result));

        let instructions = &compiler.module.instructions;
        assert!(
            instructions
                .iter()
                .any(|i| matches!(i, Instruction::NEGi { .. }))
        );
    }

    #[test]
    fn test_logical_not() {
        let mut compiler = create_test_compiler();

        let expr = Expr::Unary(UnaryOp::Not, Box::new(Expr::Literal(Literal::Bool(true))));

        compiler.analyzer.analyze_expr_context(&expr).unwrap();

        let result = compiler.generate_expr(&expr).unwrap();
        assert!(compiler.is_temp_var(result));

        let instructions = &compiler.module.instructions;
        assert!(
            instructions
                .iter()
                .any(|i| matches!(i, Instruction::NOT { .. }))
        );
    }

    #[test]
    fn test_ternary_operator() {
        let mut compiler = create_test_compiler();

        let expr = Expr::Ternary(
            Box::new(build_int_literal(1)),
            Box::new(build_int_literal(42)),
            Box::new(build_int_literal(99)),
        );

        compiler.analyzer.analyze_expr_context(&expr).unwrap();

        let result = compiler.generate_expr(&expr).unwrap();
        assert!(compiler.is_temp_var(result));

        let instructions = &compiler.module.instructions;
        assert!(
            instructions
                .iter()
                .any(|i| matches!(i, Instruction::JZ { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|i| matches!(i, Instruction::JMP { .. }))
        );
    }

    #[test]
    fn test_nested_arithmetic() {
        let mut compiler = create_test_compiler();

        // (10 + 20) * (30 - 5)
        let expr = build_binary(
            build_binary(build_int_literal(10), BinaryOp::Add, build_int_literal(20)),
            BinaryOp::Mul,
            build_binary(build_int_literal(30), BinaryOp::Sub, build_int_literal(5)),
        );

        compiler.analyzer.analyze_expr_context(&expr).unwrap();

        let result = compiler.generate_expr(&expr).unwrap();
        assert!(compiler.is_temp_var(result));

        let instructions = &compiler.module.instructions;
        assert!(
            instructions
                .iter()
                .any(|i| matches!(i, Instruction::ADDi { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|i| matches!(i, Instruction::SUBi { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|i| matches!(i, Instruction::MULi { .. }))
        );
    }

    #[test]
    fn test_temp_allocation() {
        let mut compiler = create_test_compiler();

        let temp1 = compiler.allocate_temp(TYPE_INT32);
        let temp2 = compiler.allocate_temp(TYPE_INT32);

        assert_ne!(temp1, temp2);
        assert!(compiler.is_temp_var(temp1));
        assert!(compiler.is_temp_var(temp2));
    }

    #[test]
    fn test_temp_reuse() {
        let mut compiler = create_test_compiler();

        let temp1 = compiler.allocate_temp(TYPE_INT32);
        compiler.free_temp(temp1);

        let temp2 = compiler.allocate_temp(TYPE_INT32);

        // Should reuse the freed temp
        assert_eq!(temp1, temp2);
    }

    #[test]
    fn test_string_table() {
        let mut compiler = create_test_compiler();

        let id1 = compiler.module.add_string("Hello".to_string());
        let id2 = compiler.module.add_string("World".to_string());
        let id3 = compiler.module.add_string("Hello".to_string()); // Duplicate

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(id3, 0); // Should reuse existing string

        assert_eq!(compiler.module.strings.len(), 2);
    }

    #[test]
    fn test_property_name_registration() {
        let mut compiler = create_test_compiler();

        let id1 = compiler.module.add_property_name("health".to_string());
        let id2 = compiler.module.add_property_name("mana".to_string());
        let id3 = compiler.module.add_property_name("health".to_string()); // Duplicate

        assert_eq!(id1, id3); // Should reuse
        assert_ne!(id1, id2);

        assert_eq!(compiler.module.get_property_name(id1), Some("health"));
        assert_eq!(compiler.module.get_property_name(id2), Some("mana"));
    }

    // ==================== INTEGRATION TESTS (using compile()) ====================

    #[test]
    fn test_compile_simple_return() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_int_type()),
                StatBlock {
                    statements: vec![Statement::Return(ReturnStmt {
                        value: Some(build_int_literal(42)),
                    })],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);

        let module = compiler.compile(script).unwrap();

        // Check function was registered
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "test");

        // Check bytecode was generated
        assert!(!module.instructions.is_empty());
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::RET { .. }))
        );
    }

    #[test]
    fn test_compile_arithmetic_expression() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "add",
                Some(build_int_type()),
                StatBlock {
                    statements: vec![Statement::Return(ReturnStmt {
                        value: Some(build_binary(
                            build_int_literal(10),
                            BinaryOp::Add,
                            build_int_literal(20),
                        )),
                    })],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);

        let module = compiler.compile(script).unwrap();

        // Should have addition instruction
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::ADDi { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::RET { .. }))
        );
    }

    #[test]
    fn test_compile_variable_declaration() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_int_type()),
                StatBlock {
                    statements: vec![
                        Statement::Var(Var {
                            visibility: None,
                            var_type: build_int_type(),
                            declarations: vec![VarDecl {
                                name: "x".to_string(),
                                initializer: Some(VarInit::Expr(build_int_literal(42))),
                            }],
                        }),
                        Statement::Return(ReturnStmt {
                            value: Some(build_var_access("x")),
                        }),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);

        let module = compiler.compile(script).unwrap();

        // Should have variable initialization and copy
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::SetV { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::CpyV { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::RET { .. }))
        );
    }

    #[test]
    fn test_compile_if_statement() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_int_type()),
                StatBlock {
                    statements: vec![Statement::If(IfStmt {
                        condition: build_binary(
                            build_int_literal(5),
                            BinaryOp::Gt,
                            build_int_literal(3),
                        ),
                        then_branch: Box::new(Statement::Return(ReturnStmt {
                            value: Some(build_int_literal(1)),
                        })),
                        else_branch: Some(Box::new(Statement::Return(ReturnStmt {
                            value: Some(build_int_literal(0)),
                        }))),
                    })],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);

        let module = compiler.compile(script).unwrap();

        // Should have comparison and jumps
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::CMPi { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::JZ { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::JMP { .. }))
        );
    }

    #[test]
    fn test_compile_while_loop() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_int_type()),
                StatBlock {
                    statements: vec![
                        Statement::Var(Var {
                            visibility: None,
                            var_type: build_int_type(),
                            declarations: vec![VarDecl {
                                name: "x".to_string(),
                                initializer: Some(VarInit::Expr(build_int_literal(0))),
                            }],
                        }),
                        Statement::While(WhileStmt {
                            condition: build_binary(
                                build_var_access("x"),
                                BinaryOp::Lt,
                                build_int_literal(5),
                            ),
                            body: Box::new(Statement::Expr(Some(build_binary(
                                build_var_access("x"),
                                BinaryOp::Assign,
                                build_binary(
                                    build_var_access("x"),
                                    BinaryOp::Add,
                                    build_int_literal(1),
                                ),
                            )))),
                        }),
                        Statement::Return(ReturnStmt {
                            value: Some(build_var_access("x")),
                        }),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);

        let module = compiler.compile(script).unwrap();

        // Should have comparison, conditional jump, and backward jump
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::CMPi { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::JZ { .. }))
        );

        // Check for backward jump (negative offset)
        let has_backward_jump = module.instructions.iter().any(|i| {
            if let Instruction::JMP { offset } = i {
                *offset < 0
            } else {
                false
            }
        });
        assert!(has_backward_jump);
    }

    #[test]
    fn test_compile_complex_expression() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_int_type()),
                StatBlock {
                    statements: vec![Statement::Return(ReturnStmt {
                        value: Some(build_binary(
                            build_binary(
                                build_int_literal(10),
                                BinaryOp::Add,
                                build_int_literal(20),
                            ),
                            BinaryOp::Mul,
                            build_binary(
                                build_int_literal(30),
                                BinaryOp::Sub,
                                build_int_literal(5),
                            ),
                        )),
                    })],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);

        let module = compiler.compile(script).unwrap();

        // Should have all three operations
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::ADDi { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::SUBi { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::MULi { .. }))
        );

        // Verify operation order (ADD and SUB before MUL)
        let add_pos = module
            .instructions
            .iter()
            .position(|i| matches!(i, Instruction::ADDi { .. }))
            .unwrap();
        let sub_pos = module
            .instructions
            .iter()
            .position(|i| matches!(i, Instruction::SUBi { .. }))
            .unwrap();
        let mul_pos = module
            .instructions
            .iter()
            .position(|i| matches!(i, Instruction::MULi { .. }))
            .unwrap();

        assert!(add_pos < mul_pos);
        assert!(sub_pos < mul_pos);
    }

    #[test]
    fn test_compile_multiple_variables() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_int_type()),
                StatBlock {
                    statements: vec![
                        Statement::Var(Var {
                            visibility: None,
                            var_type: build_int_type(),
                            declarations: vec![VarDecl {
                                name: "a".to_string(),
                                initializer: Some(VarInit::Expr(build_int_literal(10))),
                            }],
                        }),
                        Statement::Var(Var {
                            visibility: None,
                            var_type: build_int_type(),
                            declarations: vec![VarDecl {
                                name: "b".to_string(),
                                initializer: Some(VarInit::Expr(build_int_literal(20))),
                            }],
                        }),
                        Statement::Var(Var {
                            visibility: None,
                            var_type: build_int_type(),
                            declarations: vec![VarDecl {
                                name: "c".to_string(),
                                initializer: Some(VarInit::Expr(build_binary(
                                    build_var_access("a"),
                                    BinaryOp::Add,
                                    build_var_access("b"),
                                ))),
                            }],
                        }),
                        Statement::Return(ReturnStmt {
                            value: Some(build_var_access("c")),
                        }),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);

        let module = compiler.compile(script).unwrap();

        // Should have variable initializations and addition
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::ADDi { .. }))
        );

        // Should have multiple SetV instructions for initializations
        let set_count = module
            .instructions
            .iter()
            .filter(|i| matches!(i, Instruction::SetV { .. }))
            .count();
        assert!(set_count >= 2); // At least for literals 10 and 20
    }
}
