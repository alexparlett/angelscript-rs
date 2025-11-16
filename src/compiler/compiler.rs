use crate::compiler::bytecode::{BytecodeModule, GlobalVar, Instruction};
use crate::compiler::semantic_analyzer::SemanticAnalyzer;
use crate::core::engine::EngineInner;
use crate::core::error::{CodegenError, CodegenResult, CompileError, CompileResult};
use crate::core::types::{BehaviourType, ScriptValue, TypeFlags, TypeId, TypeRegistration, TYPE_BOOL, TYPE_DOUBLE, TYPE_FLOAT, TYPE_INT16, TYPE_INT32, TYPE_INT64, TYPE_INT8, TYPE_STRING, TYPE_UINT16, TYPE_UINT32, TYPE_UINT64, TYPE_UINT8, TYPE_VOID};
use crate::parser::ast::*;
use std::sync::{Arc, RwLock};

pub struct Compiler {
    module: BytecodeModule,
    current_address: u32,
    analyzer: SemanticAnalyzer,
    current_function: Option<FunctionContext>,
    current_class: Option<String>,
    current_namespace: Vec<String>,
    break_targets: Vec<Vec<u32>>,
    continue_targets: Vec<Vec<u32>>,
    lambda_count: u32,
}

#[derive(Debug, Clone)]
struct FunctionContext {
    name: String,
    has_return: bool,
}

impl Compiler {
    pub fn new(engine: Arc<RwLock<EngineInner>>) -> Self {
        Self {
            module: BytecodeModule::new(),
            current_address: 0,
            analyzer: SemanticAnalyzer::new(engine),
            current_function: None,
            current_class: None,
            current_namespace: Vec::new(),
            break_targets: Vec::new(),
            continue_targets: Vec::new(),
            lambda_count: 0,
        }
    }

    pub fn with_analyzer(analyzer: SemanticAnalyzer) -> Self {
        Self {
            module: BytecodeModule::new(),
            current_address: 0,
            analyzer,
            current_function: None,
            current_class: None,
            current_namespace: Vec::new(),
            break_targets: Vec::new(),
            continue_targets: Vec::new(),
            lambda_count: 0,
        }
    }

    pub fn compile(mut self, script: Script) -> CompileResult<BytecodeModule> {
        self.analyzer
            .analyze(&script)
            .map_err(|errors| CompileError::SemanticErrors(errors))?;

        self.generate(script)
            .map_err(|e| CompileError::CodegenError(e))
    }

    fn generate(mut self, script: Script) -> CodegenResult<BytecodeModule> {
        for item in &script.items {
            match item {
                ScriptNode::Func(func) => self.generate_function(func)?,
                ScriptNode::Class(class) => self.generate_class(class)?,
                ScriptNode::Var(var) => self.generate_global_var(var)?,
                ScriptNode::Namespace(ns) => self.generate_namespace(ns)?,
                ScriptNode::Enum(_) => {}
                ScriptNode::Interface(_) => {}
                _ => {}
            }
        }

        Ok(self.module)
    }

    fn generate_function(&mut self, func: &Func) -> CodegenResult<()> {
        let func_address = self.current_address;

        let func_full_name = if let Some(class_name) = &self.current_class {
            format!("{}::{}", class_name, func.name)
        } else if !self.current_namespace.is_empty() {
            format!("{}::{}", self.current_namespace.join("::"), func.name)
        } else {
            func.name.clone()
        };

        let func_info = self
            .analyzer
            .symbol_table
            .get_function(&func_full_name)
            .ok_or_else(|| CodegenError::UndefinedFunction(func.name.clone()))?;

        let func_locals = self
            .analyzer
            .symbol_table
            .get_function_locals(&func_full_name)
            .ok_or_else(|| {
                CodegenError::Internal(format!("Function locals not found: {}", func_full_name))
            })?;

        self.current_function = Some(FunctionContext {
            name: func.name.clone(),
            has_return: false,
        });

        if self.is_constructor(func) {
            self.generate_constructor_prologue(&func.name)?;
        }

        if let Some(body) = &func.body {
            self.generate_statement_block(body)?;
        }

        if self.is_destructor(func) {
            self.generate_destructor_epilogue(&func.name)?;
        }

        if !self.last_instruction_is_return() {
            if func_info.return_type == TYPE_VOID {
                self.emit(Instruction::RET { stack_size: 0 });
            } else {
                self.emit_default_return(func_info.return_type);
            }
        }

        let bytecode_func_info = crate::compiler::bytecode::FunctionInfo {
            name: func_full_name.clone(),
            address: func_address,
            param_count: func_locals.param_count as u8,
            local_count: func_locals.total_count as u32,
            stack_size: 0,
            return_type: func_info.return_type,
            is_script_func: true,
        };

        self.module.functions.push(bytecode_func_info);

        self.analyzer
            .symbol_table
            .update_function_address(&func_full_name, func_address);

        self.current_function = None;

        Ok(())
    }

    fn is_constructor(&self, func: &Func) -> bool {
        if let Some(class_name) = &self.current_class {
            func.name == *class_name && func.return_type.is_none()
        } else {
            false
        }
    }

    fn is_destructor(&self, func: &Func) -> bool {
        func.name.starts_with('~')
    }

    fn generate_constructor_prologue(&mut self, class_name: &str) -> CodegenResult<()> {
        let type_id = self
            .analyzer
            .symbol_table
            .lookup_type(class_name)
            .ok_or_else(|| CodegenError::UnknownType(class_name.to_string()))?;

        let type_info = self
            .analyzer
            .symbol_table
            .get_type(type_id)
            .ok_or_else(|| CodegenError::UnknownType(class_name.to_string()))?;

        if let Some(base_type_id) = type_info.base_class {
            self.emit(Instruction::PshR);

            if let Some(base_ctor_id) = self.find_default_constructor(base_type_id) {
                self.emit(Instruction::CALL {
                    func_id: base_ctor_id,
                });
            }
        }

        let initializers = self
            .analyzer
            .symbol_table
            .get_member_initializers_cloned(class_name);

        if let Some(initializers) = initializers {
            for (member_name, init_expr) in initializers {
                let init_var = self.generate_expr(&init_expr)?;

                let prop_name_id = self.module.add_property_name(member_name.clone());

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

    fn generate_destructor_epilogue(&mut self, class_name: &str) -> CodegenResult<()> {
        let class_name = class_name.trim_start_matches('~');

        let type_id = self
            .analyzer
            .symbol_table
            .lookup_type(class_name)
            .ok_or_else(|| CodegenError::UnknownType(class_name.to_string()))?;

        let type_info = self
            .analyzer
            .symbol_table
            .get_type(type_id)
            .ok_or_else(|| CodegenError::UnknownType(class_name.to_string()))?;

        for (member_name, member_info) in &type_info.members {
            if self.type_has_destructor(member_info.type_id) {
                let prop_name_id = self.module.add_property_name(member_name.clone());
                let temp = self.allocate_temp(member_info.type_id);

                self.emit(Instruction::GetThisProperty {
                    prop_name_id,
                    dst_var: temp,
                });

                if let Some(dtor_id) = self.get_destructor_id(member_info.type_id) {
                    self.emit(Instruction::CALL { func_id: dtor_id });
                }

                self.free_temp(temp);
            }
        }

        Ok(())
    }

    fn find_default_constructor(&self, type_id: TypeId) -> Option<u32> {
        let type_info = self.analyzer.symbol_table.get_type(type_id)?;
        let ctor_name = format!("{}::{}", type_info.name, type_info.name);
        let func_info = self.analyzer.symbol_table.get_function(&ctor_name)?;
        Some(func_info.address)
    }

    fn type_has_destructor(&self, type_id: TypeId) -> bool {
        self.analyzer
            .symbol_table
            .get_type(type_id)
            .map(|t| t.flags.contains(TypeFlags::APP_CLASS_DESTRUCTOR))
            .unwrap_or(false)
    }

    fn get_destructor_id(&self, type_id: TypeId) -> Option<u32> {
        let type_info = self.analyzer.symbol_table.get_type(type_id)?;
        let dtor_name = format!("{}::~{}", type_info.name, type_info.name);
        let func_info = self.analyzer.symbol_table.get_function(&dtor_name)?;
        Some(func_info.address)
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

    fn generate_class(&mut self, class: &Class) -> CodegenResult<()> {
        self.current_class = Some(class.name.clone());

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

    fn generate_virtual_property(&mut self, prop: &VirtProp) -> CodegenResult<()> {
        for accessor in &prop.accessors {
            if let Some(body) = &accessor.body {
                let func_name = match accessor.kind {
                    AccessorKind::Get => format!("get_{}", prop.name),
                    AccessorKind::Set => format!("set_{}", prop.name),
                };

                let func = Func {
                    modifiers: vec![],
                    visibility: prop.visibility.clone(),
                    return_type: match accessor.kind {
                        AccessorKind::Get => Some(prop.prop_type.clone()),
                        AccessorKind::Set => None,
                    },
                    is_ref: prop.is_ref,
                    name: func_name,
                    params: match accessor.kind {
                        AccessorKind::Get => vec![],
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

                self.generate_function(&func)?;
            }
        }

        Ok(())
    }

    fn generate_global_var(&mut self, var: &Var) -> CodegenResult<()> {
        let var_type = self
            .analyzer
            .symbol_table
            .resolve_type_from_ast(&var.var_type);

        for decl in &var.declarations {
            if let Some(global_info) = self.analyzer.symbol_table.get_global(&decl.name) {
                self.module.globals.push(GlobalVar {
                    name: decl.name.clone(),
                    type_id: var_type,
                    address: global_info.address,
                    is_const: global_info.is_const,
                });

                if let Some(VarInit::Expr(expr)) = &decl.initializer {
                    if let Expr::Literal(lit) = expr {
                        self.generate_global_literal_init(global_info.address, lit);
                    }
                }
            }
        }
        Ok(())
    }

    fn generate_global_literal_init(&mut self, global_id: u32, lit: &Literal) {
        let value = match lit {
            Literal::Number(n) => {
                if n.ends_with("ull") || n.ends_with("ULL") {
                    let val: u64 = n
                        .trim_end_matches(|c: char| c.is_alphabetic())
                        .parse()
                        .unwrap_or(0);
                    ScriptValue::UInt64(val)
                } else if n.ends_with("ll") || n.ends_with("LL") {
                    let val: i64 = n
                        .trim_end_matches(|c: char| c.is_alphabetic())
                        .parse()
                        .unwrap_or(0);
                    ScriptValue::Int64(val)
                } else if n.ends_with("ul")
                    || n.ends_with("UL")
                    || n.ends_with("lu")
                    || n.ends_with("LU")
                {
                    let val: u32 = n
                        .trim_end_matches(|c: char| c.is_alphabetic())
                        .parse()
                        .unwrap_or(0);
                    ScriptValue::UInt32(val)
                } else if n.ends_with("u") || n.ends_with("U") {
                    let val: u32 = n
                        .trim_end_matches(|c: char| c.is_alphabetic())
                        .parse()
                        .unwrap_or(0);
                    ScriptValue::UInt32(val)
                } else if n.ends_with("l") || n.ends_with("L") {
                    let val: i64 = n
                        .trim_end_matches(|c: char| c.is_alphabetic())
                        .parse()
                        .unwrap_or(0);
                    ScriptValue::Int64(val)
                } else if n.ends_with("f") || n.ends_with("F") {
                    let val: f32 = n
                        .trim_end_matches(|c: char| c.is_alphabetic())
                        .parse()
                        .unwrap_or(0.0);
                    ScriptValue::Float(val)
                } else if n.contains('.') || n.contains('e') || n.contains('E') {
                    let val: f64 = n.parse().unwrap_or(0.0);
                    ScriptValue::Double(val)
                } else {
                    let val: i32 = n.parse().unwrap_or(0);
                    ScriptValue::Int32(val)
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

    fn generate_namespace(&mut self, namespace: &Namespace) -> CodegenResult<()> {
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

        self.current_namespace
            .truncate(self.current_namespace.len() - namespace.name.len());

        Ok(())
    }

    fn generate_statement_block(&mut self, block: &StatBlock) -> CodegenResult<()> {
        for stmt in &block.statements {
            self.generate_statement(stmt)?;
        }
        Ok(())
    }

    fn generate_statement(&mut self, stmt: &Statement) -> CodegenResult<()> {
        match stmt {
            Statement::Var(var) => self.generate_var_decl(var),
            Statement::Expr(Some(e)) => {
                let result_var = self.generate_expr(e)?;
                if self.is_temp_var(result_var) {
                    self.free_temp(result_var);
                }
                Ok(())
            }
            Statement::Expr(None) => Ok(()),
            Statement::If(if_stmt) => self.generate_if(if_stmt),
            Statement::While(while_stmt) => self.generate_while(while_stmt),
            Statement::DoWhile(do_while) => self.generate_do_while(do_while),
            Statement::For(for_stmt) => self.generate_for(for_stmt),
            Statement::Return(ret) => self.generate_return(ret),
            Statement::Break => self.generate_break(),
            Statement::Continue => self.generate_continue(),
            Statement::Block(block) => self.generate_statement_block(block),
            Statement::ForEach(foreach_stmt) => self.generate_foreach(foreach_stmt),
            Statement::Switch(switch_stmt) => self.generate_switch(switch_stmt),
            Statement::Try(try_stmt) => self.generate_try(try_stmt),
            Statement::Using(_) => Ok(()),
        }
    }

    fn generate_var_decl(&mut self, var: &Var) -> CodegenResult<()> {
        let var_type = self
            .analyzer
            .symbol_table
            .resolve_type_from_ast(&var.var_type);

        let func_name = self
            .current_function
            .as_ref()
            .map(|f| f.name.clone())
            .ok_or_else(|| CodegenError::Internal("No current function".to_string()))?;

        let func_full_name = if let Some(class_name) = &self.current_class {
            format!("{}::{}", class_name, func_name)
        } else if !self.current_namespace.is_empty() {
            format!("{}::{}", self.current_namespace.join("::"), func_name)
        } else {
            func_name
        };

        let func_locals = self
            .analyzer
            .symbol_table
            .get_function_locals(&func_full_name)
            .ok_or_else(|| CodegenError::Internal("Function locals not found".to_string()))?;

        for decl in &var.declarations {
            let var_idx = func_locals
                .variable_map
                .get(&decl.name)
                .copied()
                .ok_or_else(|| CodegenError::UndefinedVariable(decl.name.clone()))?;

            if let Some(init) = &decl.initializer {
                match init {
                    VarInit::Expr(expr) => {
                        let result_var = self.generate_expr(expr)?;
                        self.emit(Instruction::CpyV {
                            dst: var_idx as u32,
                            src: result_var,
                        });
                        if self.is_temp_var(result_var) {
                            self.free_temp(result_var);
                        }
                    }
                    VarInit::InitList(init_list) => {
                        self.generate_init_list_expr(init_list)?;
                        self.emit(Instruction::PopR);
                        self.emit(Instruction::CpyRtoV {
                            var: var_idx as u32,
                        });
                    }
                    VarInit::ArgList(args) => {
                        let temp = self.generate_construct_call(&var.var_type, args)?;
                        self.emit(Instruction::CpyV {
                            dst: var_idx as u32,
                            src: temp,
                        });
                        if self.is_temp_var(temp) {
                            self.free_temp(temp);
                        }
                    }
                }
            } else {
                self.emit_default_init(var_idx as u32, var_type);
            }
        }
        Ok(())
    }

    fn generate_construct_call(&mut self, type_def: &Type, args: &[Arg]) -> CodegenResult<u32> {
        let type_id = self.analyzer.symbol_table.resolve_type_from_ast(type_def);

        let type_info = self
            .analyzer
            .symbol_table
            .get_type(type_id)
            .ok_or_else(|| CodegenError::UnknownType(format!("type {}", type_id)))?;

        let func_id = if type_info.registration == TypeRegistration::Application {
            self.find_construct_behaviour(type_id, args.len())?
        } else {
            self.find_constructor(type_id, args.len()).unwrap_or(0)
        };

        self.emit(Instruction::Alloc { type_id, func_id });

        for arg in args.iter().rev() {
            let arg_var = self.generate_expr(&arg.value)?;
            self.emit(Instruction::PshV { var: arg_var });

            if self.is_temp_var(arg_var) {
                self.free_temp(arg_var);
            }
        }

        if func_id != 0 {
            if type_info.registration == TypeRegistration::Application {
                self.emit(Instruction::CALLSYS {
                    sys_func_id: func_id,
                });
            } else {
                self.emit(Instruction::CALL { func_id });
            }
        }

        let result = self.allocate_temp(type_id);
        self.emit(Instruction::StoreObj { var: result });

        Ok(result)
    }

    fn find_constructor(&self, type_id: TypeId, arg_count: usize) -> Option<u32> {
        let type_info = self.analyzer.symbol_table.get_type(type_id)?;
        let ctor_name = format!("{}::{}", type_info.name, type_info.name);
        let func_info = self.analyzer.symbol_table.get_function(&ctor_name)?;

        if func_info.params.len() == arg_count {
            Some(func_info.address)
        } else {
            None
        }
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

    fn generate_if(&mut self, if_stmt: &IfStmt) -> CodegenResult<()> {
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

    fn generate_while(&mut self, while_stmt: &WhileStmt) -> CodegenResult<()> {
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

    fn generate_do_while(&mut self, do_while: &DoWhileStmt) -> CodegenResult<()> {
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

    fn generate_for(&mut self, for_stmt: &ForStmt) -> CodegenResult<()> {
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

    fn generate_foreach(&mut self, foreach_stmt: &ForEachStmt) -> CodegenResult<()> {
        let collection_var = self.generate_expr(&foreach_stmt.iterable)?;
        let collection_type = self.get_expr_type(&foreach_stmt.iterable);

        let func_name = self
            .current_function
            .as_ref()
            .map(|f| f.name.clone())
            .ok_or_else(|| CodegenError::Internal("No current function".to_string()))?;

        let func_full_name = if let Some(class_name) = &self.current_class {
            format!("{}::{}", class_name, func_name)
        } else if !self.current_namespace.is_empty() {
            format!("{}::{}", self.current_namespace.join("::"), func_name)
        } else {
            func_name
        };

        let func_locals = self
            .analyzer
            .symbol_table
            .get_function_locals(&func_full_name)
            .ok_or_else(|| CodegenError::Internal("Function locals not found".to_string()))?;

        let iterator_var = func_locals.total_count as u32 + self.lambda_count;
        self.lambda_count += 1;

        if let Some(begin_id) = self.find_operator_method(collection_type, "opForBegin") {
            self.emit(Instruction::PshV {
                var: collection_var,
            });
            self.emit(Instruction::CALL { func_id: begin_id });
            self.emit(Instruction::PopR);
            self.emit(Instruction::CpyRtoV { var: iterator_var });
        } else {
            return Err(CodegenError::NotImplemented(
                "foreach - collection type doesn't implement opForBegin".to_string(),
            ));
        }

        let loop_start = self.current_address;

        self.break_targets.push(Vec::new());
        self.continue_targets.push(Vec::new());

        if let Some(end_id) = self.find_operator_method(collection_type, "opForEnd") {
            self.emit(Instruction::PshV {
                var: collection_var,
            });
            self.emit(Instruction::PshV { var: iterator_var });
            self.emit(Instruction::CALL { func_id: end_id });
            self.emit(Instruction::PopR);
            self.emit(Instruction::TNZ);
        } else {
            return Err(CodegenError::NotImplemented(
                "foreach - collection type doesn't implement opForEnd".to_string(),
            ));
        }

        let jump_to_end = self.emit_jump_placeholder(Instruction::JNZ { offset: 0 });

        for (_var_type, var_name) in &foreach_stmt.variables {
            let value_var = func_locals
                .variable_map
                .get(var_name)
                .copied()
                .ok_or_else(|| CodegenError::UndefinedVariable(var_name.clone()))?;

            if let Some(value_id) = self.find_operator_method(collection_type, "opForValue") {
                self.emit(Instruction::PshV {
                    var: collection_var,
                });
                self.emit(Instruction::PshV { var: iterator_var });
                self.emit(Instruction::CALL { func_id: value_id });
                self.emit(Instruction::PopR);
                self.emit(Instruction::CpyRtoV {
                    var: value_var as u32,
                });
            } else {
                return Err(CodegenError::NotImplemented(
                    "foreach - collection type doesn't implement opForValue".to_string(),
                ));
            }
        }

        self.generate_statement(&foreach_stmt.body)?;

        if let Some(next_id) = self.find_operator_method(collection_type, "opForNext") {
            self.emit(Instruction::PshV {
                var: collection_var,
            });
            self.emit(Instruction::PshV { var: iterator_var });
            self.emit(Instruction::CALL { func_id: next_id });
            self.emit(Instruction::PopR);
            self.emit(Instruction::CpyRtoV { var: iterator_var });
        } else {
            return Err(CodegenError::NotImplemented(
                "foreach - collection type doesn't implement opForNext".to_string(),
            ));
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

        Ok(())
    }

    fn generate_switch(&mut self, switch_stmt: &SwitchStmt) -> CodegenResult<()> {
        let switch_var = self.generate_expr(&switch_stmt.value)?;
        let switch_type = self.get_expr_type(&switch_stmt.value);

        self.break_targets.push(Vec::new());

        let mut case_jumps = Vec::new();
        let mut default_jump = None;

        for case in &switch_stmt.cases {
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
        for case in &switch_stmt.cases {
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

    fn generate_try(&mut self, try_stmt: &TryStmt) -> CodegenResult<()> {
        self.generate_statement_block(&try_stmt.try_block)?;
        self.generate_statement_block(&try_stmt.catch_block)?;
        Ok(())
    }

    fn generate_return(&mut self, ret: &ReturnStmt) -> CodegenResult<()> {
        if let Some(value) = &ret.value {
            let result_var = self.generate_expr(value)?;
            self.emit(Instruction::CpyVtoR { var: result_var });

            if self.is_temp_var(result_var) {
                self.free_temp(result_var);
            }
        }

        self.emit(Instruction::RET { stack_size: 0 });

        if let Some(func_ctx) = &mut self.current_function {
            func_ctx.has_return = true;
        }

        Ok(())
    }

    fn generate_break(&mut self) -> CodegenResult<()> {
        let jump_addr = self.emit_jump_placeholder(Instruction::JMP { offset: 0 });

        if let Some(breaks) = self.break_targets.last_mut() {
            breaks.push(jump_addr);
            Ok(())
        } else {
            Err(CodegenError::InvalidBreak)
        }
    }

    fn generate_continue(&mut self) -> CodegenResult<()> {
        let jump_addr = self.emit_jump_placeholder(Instruction::JMP { offset: 0 });

        if let Some(continues) = self.continue_targets.last_mut() {
            continues.push(jump_addr);
            Ok(())
        } else {
            Err(CodegenError::InvalidContinue)
        }
    }

    fn generate_expr(&mut self, expr: &Expr) -> CodegenResult<u32> {
        match expr {
            Expr::Literal(lit) => self.generate_literal(lit),
            Expr::VarAccess(scope, name) => self.generate_var_access(expr, scope, name),
            Expr::Binary(left, op, right) => self.generate_binary(left, op, right),
            Expr::Unary(op, operand) => self.generate_unary(op, operand),
            Expr::Postfix(expr_inner, op) => self.generate_postfix(expr_inner, op),
            Expr::Ternary(cond, then_expr, else_expr) => {
                self.generate_ternary(cond, then_expr, else_expr)
            }
            Expr::FuncCall(call) => self.generate_func_call(call),
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
            Expr::ConstructCall(type_def, args) => self.generate_construct_call(type_def, args),
            Expr::Cast(target_type, expr_inner) => self.generate_cast(target_type, expr_inner),
        }
    }

    fn generate_literal(&mut self, lit: &Literal) -> CodegenResult<u32> {
        let (value, type_id) = match lit {
            Literal::Bool(b) => (ScriptValue::Bool(*b), TYPE_BOOL),
            Literal::Number(n) => {
                if n.ends_with("ull") || n.ends_with("ULL") {
                    let val: u64 = n
                        .trim_end_matches(|c: char| c.is_alphabetic())
                        .parse()
                        .unwrap_or(0);
                    (ScriptValue::UInt64(val), TYPE_UINT64)
                } else if n.ends_with("ll") || n.ends_with("LL") {
                    let val: i64 = n
                        .trim_end_matches(|c: char| c.is_alphabetic())
                        .parse()
                        .unwrap_or(0);
                    (ScriptValue::Int64(val), TYPE_INT64)
                } else if n.ends_with("ul")
                    || n.ends_with("UL")
                    || n.ends_with("lu")
                    || n.ends_with("LU")
                {
                    let val: u32 = n
                        .trim_end_matches(|c: char| c.is_alphabetic())
                        .parse()
                        .unwrap_or(0);
                    (ScriptValue::UInt32(val), TYPE_UINT32)
                } else if n.ends_with("u") || n.ends_with("U") {
                    let val: u32 = n
                        .trim_end_matches(|c: char| c.is_alphabetic())
                        .parse()
                        .unwrap_or(0);
                    (ScriptValue::UInt32(val), TYPE_UINT32)
                } else if n.ends_with("l") || n.ends_with("L") {
                    let val: i64 = n
                        .trim_end_matches(|c: char| c.is_alphabetic())
                        .parse()
                        .unwrap_or(0);
                    (ScriptValue::Int64(val), TYPE_INT64)
                } else if n.ends_with("f") || n.ends_with("F") {
                    let val: f32 = n
                        .trim_end_matches(|c: char| c.is_alphabetic())
                        .parse()
                        .unwrap_or(0.0);
                    (ScriptValue::Float(val), TYPE_FLOAT)
                } else if n.contains('.') || n.contains('e') || n.contains('E') {
                    let val: f64 = n.parse().unwrap_or(0.0);
                    (ScriptValue::Double(val), TYPE_DOUBLE)
                } else {
                    let val: i32 = n.parse().unwrap_or(0);
                    (ScriptValue::Int32(val), TYPE_INT32)
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

    fn generate_var_access(
        &mut self,
        expr: &Expr,
        _scope: &crate::parser::ast::Scope,
        name: &str,
    ) -> CodegenResult<u32> {
        if let Some(ctx) = self.analyzer.symbol_table.get_expr_context(expr) {
            if let Some(index) = ctx.resolved_var_index {
                return Ok(index as u32);
            }

            if ctx.is_global {
                if let Some(global_addr) = ctx.global_address {
                    let temp = self.allocate_temp(ctx.result_type);
                    self.emit(Instruction::CpyGtoV {
                        var: temp,
                        global_id: global_addr,
                    });
                    return Ok(temp);
                }
            }
        }

        Err(CodegenError::Internal(format!(
            "Variable '{}' not resolved during semantic analysis",
            name
        )))
    }

    fn get_common_type(&self, type1: TypeId, type2: TypeId) -> TypeId {
        if type1 == type2 {
            return type1;
        }

        let rank = |t: TypeId| -> u32 {
            match t {
                TYPE_DOUBLE => 6,
                TYPE_FLOAT => 5,
                TYPE_INT64 | TYPE_UINT64 => 4,
                TYPE_UINT32 => 3,
                TYPE_INT32 => 2,
                TYPE_INT16 | TYPE_UINT16 => 1,
                TYPE_INT8 | TYPE_UINT8 | TYPE_BOOL => 0,
                _ => 0,
            }
        };

        if rank(type1) > rank(type2) {
            type1
        } else {
            type2
        }
    }

    // src/compiler/compiler.rs - continued

    fn generate_binary(&mut self, left: &Expr, op: &BinaryOp, right: &Expr) -> CodegenResult<u32> {
        if matches!(op, BinaryOp::Assign) {
            return self.generate_assignment(left, op, right);
        }

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

        if matches!(op, BinaryOp::And | BinaryOp::Or) {
            return self.generate_logical_op(left, op, right);
        }

        let left_var = self.generate_expr(left)?;
        let right_var = self.generate_expr(right)?;

        let result_type = self.get_expr_type(&Expr::Binary(
            Box::new(left.clone()),
            op.clone(),
            Box::new(right.clone()),
        ));

        let result_var = self.allocate_temp(result_type);

        let operation_type = if matches!(
            op,
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
        ) {
            let left_type = self.get_expr_type(left);
            let right_type = self.get_expr_type(right);
            self.get_common_type(left_type, right_type)
        } else {
            result_type
        };

        self.emit_binary_op(op, result_var, left_var, right_var, operation_type)?;

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
    ) -> CodegenResult<()> {
        let instr = match (op, type_id) {
            (BinaryOp::Add, TYPE_INT32) => Instruction::ADDi { dst, a, b },
            (BinaryOp::Sub, TYPE_INT32) => Instruction::SUBi { dst, a, b },
            (BinaryOp::Mul, TYPE_INT32) => Instruction::MULi { dst, a, b },
            (BinaryOp::Div, TYPE_INT32) => Instruction::DIVi { dst, a, b },
            (BinaryOp::Mod, TYPE_INT32) => Instruction::MODi { dst, a, b },
            (BinaryOp::Pow, TYPE_INT32) => Instruction::POWi { dst, a, b },

            (BinaryOp::Add, TYPE_UINT32) => Instruction::ADDi { dst, a, b },
            (BinaryOp::Sub, TYPE_UINT32) => Instruction::SUBi { dst, a, b },
            (BinaryOp::Mul, TYPE_UINT32) => Instruction::MULi { dst, a, b },
            (BinaryOp::Div, TYPE_UINT32) => Instruction::DIVu { dst, a, b },
            (BinaryOp::Mod, TYPE_UINT32) => Instruction::MODu { dst, a, b },
            (BinaryOp::Pow, TYPE_UINT32) => Instruction::POWu { dst, a, b },

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

            (BinaryOp::Add, TYPE_INT64) => Instruction::ADDi64 { dst, a, b },
            (BinaryOp::Sub, TYPE_INT64) => Instruction::SUBi64 { dst, a, b },
            (BinaryOp::Mul, TYPE_INT64) => Instruction::MULi64 { dst, a, b },
            (BinaryOp::Div, TYPE_INT64) => Instruction::DIVi64 { dst, a, b },
            (BinaryOp::Mod, TYPE_INT64) => Instruction::MODi64 { dst, a, b },
            (BinaryOp::Pow, TYPE_INT64) => Instruction::POWi64 { dst, a, b },

            (BinaryOp::Add, TYPE_UINT64) => Instruction::ADDi64 { dst, a, b },
            (BinaryOp::Sub, TYPE_UINT64) => Instruction::SUBi64 { dst, a, b },
            (BinaryOp::Mul, TYPE_UINT64) => Instruction::MULi64 { dst, a, b },
            (BinaryOp::Div, TYPE_UINT64) => Instruction::DIVu64 { dst, a, b },
            (BinaryOp::Mod, TYPE_UINT64) => Instruction::MODu64 { dst, a, b },
            (BinaryOp::Pow, TYPE_UINT64) => Instruction::POWu64 { dst, a, b },

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
            _ => Instruction::CMPu64 { a, b },
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
    ) -> CodegenResult<u32> {
        if let Expr::Postfix(obj, PostfixOp::MemberAccess(member)) = left {
            let right_var = self.generate_expr(right)?;

            if let Expr::VarAccess(_, name) = obj.as_ref() {
                if name == "this" {
                    let prop_name_id = self.module.add_property_name(member.clone());

                    if matches!(op, BinaryOp::Assign) {
                        self.emit(Instruction::SetThisProperty {
                            prop_name_id,
                            src_var: right_var,
                        });
                    } else {
                        let temp = self.allocate_temp(TYPE_INT32);

                        self.emit(Instruction::GetThisProperty {
                            prop_name_id,
                            dst_var: temp,
                        });

                        let result_type = self.get_expr_type(left);
                        self.emit_compound_assignment_op(op, temp, temp, right_var, result_type)?;

                        self.emit(Instruction::SetThisProperty {
                            prop_name_id,
                            src_var: temp,
                        });

                        self.free_temp(temp);
                    }

                    if self.is_temp_var(right_var) {
                        self.free_temp(right_var);
                    }

                    return Ok(right_var);
                }
            }

            let obj_var = self.generate_expr(obj)?;
            let prop_name_id = self.module.add_property_name(member.clone());

            if matches!(op, BinaryOp::Assign) {
                self.emit(Instruction::SetProperty {
                    obj_var,
                    prop_name_id,
                    src_var: right_var,
                });
            } else {
                let temp = self.allocate_temp(TYPE_INT32);

                self.emit(Instruction::GetProperty {
                    obj_var,
                    prop_name_id,
                    dst_var: temp,
                });

                let result_type = self.get_expr_type(left);
                self.emit_compound_assignment_op(op, temp, temp, right_var, result_type)?;

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

        if let Expr::VarAccess(_scope, name) = left {
            let ctx = self
                .analyzer
                .symbol_table
                .get_expr_context(left)
                .cloned()
                .ok_or_else(|| {
                    CodegenError::Internal(format!(
                        "Variable '{}' not resolved during semantic analysis",
                        name
                    ))
                })?;

            let right_var = self.generate_expr(right)?;

            if ctx.is_global {
                if let Some(global_addr) = ctx.global_address {
                    if matches!(op, BinaryOp::Assign) {
                        self.emit(Instruction::CpyVtoG {
                            global_id: global_addr,
                            var: right_var,
                        });
                    } else {
                        let temp = self.allocate_temp(ctx.result_type);
                        self.emit(Instruction::CpyGtoV {
                            var: temp,
                            global_id: global_addr,
                        });

                        self.emit_compound_assignment_op(
                            op,
                            temp,
                            temp,
                            right_var,
                            ctx.result_type,
                        )?;

                        self.emit(Instruction::CpyVtoG {
                            global_id: global_addr,
                            var: temp,
                        });
                        self.free_temp(temp);
                    }

                    if self.is_temp_var(right_var) {
                        self.free_temp(right_var);
                    }
                    return Ok(right_var);
                }
            }

            if let Some(lvalue) = ctx.resolved_var_index {
                let lvalue = lvalue as u32;

                if matches!(op, BinaryOp::Assign) {
                    if self.is_value_type(ctx.result_type) {
                        self.emit(Instruction::COPY {
                            dst: lvalue,
                            src: right_var,
                        });
                    } else {
                        self.emit(Instruction::CpyV {
                            dst: lvalue,
                            src: right_var,
                        });
                    }
                } else {
                    self.emit_compound_assignment_op(
                        op,
                        lvalue,
                        lvalue,
                        right_var,
                        ctx.result_type,
                    )?;
                }

                if self.is_temp_var(right_var) {
                    self.free_temp(right_var);
                }

                return Ok(lvalue);
            }

            return Err(CodegenError::Internal(format!(
                "Variable '{}' has no resolved index or global address",
                name
            )));
        }

        Err(CodegenError::InvalidLValue)
    }

    fn emit_compound_assignment_op(
        &mut self,
        op: &BinaryOp,
        dst: u32,
        left: u32,
        right: u32,
        type_id: TypeId,
    ) -> CodegenResult<()> {
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
    ) -> CodegenResult<u32> {
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

    // src/compiler/compiler.rs - continued

    fn generate_unary(&mut self, op: &UnaryOp, operand: &Expr) -> CodegenResult<u32> {
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
                return Ok(result);
            }

            (UnaryOp::Plus, _) => {
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

    fn generate_postfix(&mut self, expr: &Expr, op: &PostfixOp) -> CodegenResult<u32> {
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

    fn generate_member_access(&mut self, obj: &Expr, member: &str) -> CodegenResult<u32> {
        if let Expr::VarAccess(_, name) = obj {
            if name == "this" {
                let class_type = if let Some(class_name) = &self.current_class {
                    self.analyzer
                        .symbol_table
                        .lookup_type(class_name)
                        .ok_or_else(|| CodegenError::UnknownType(class_name.clone()))?
                } else {
                    return Err(CodegenError::Internal(
                        "'this' used outside of class".to_string(),
                    ));
                };

                let member_type = self.get_member_type(class_type, member)?;
                let prop_name_id = self.module.add_property_name(member.to_string());
                let result = self.allocate_temp(member_type);

                self.emit(Instruction::GetThisProperty {
                    prop_name_id,
                    dst_var: result,
                });

                return Ok(result);
            }
        }

        let obj_var = self.generate_expr(obj)?;
        let obj_type = self.get_expr_type(obj);
        let member_type = self.get_member_type(obj_type, member)?;
        let prop_name_id = self.module.add_property_name(member.to_string());
        let result = self.allocate_temp(member_type);

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

    fn get_member_type(&self, obj_type: TypeId, member: &str) -> CodegenResult<TypeId> {
        let type_info = self
            .analyzer
            .symbol_table
            .get_type(obj_type)
            .ok_or_else(|| CodegenError::UnknownType(format!("type {}", obj_type)))?;

        let member_info = type_info
            .members
            .get(member)
            .ok_or_else(|| CodegenError::UndefinedMember(member.to_string()))?;

        Ok(member_info.type_id)
    }

    fn generate_method_call(&mut self, obj: &Expr, call: &FuncCall) -> CodegenResult<u32> {
        let obj_var = self.generate_expr(obj)?;
        self.emit(Instruction::PshV { var: obj_var });

        for arg in call.args.iter().rev() {
            let arg_var = self.generate_expr(&arg.value)?;
            self.emit(Instruction::PshV { var: arg_var });

            if self.is_temp_var(arg_var) {
                self.free_temp(arg_var);
            }
        }

        let obj_type = self.get_expr_type(obj);
        let method_id = self
            .find_method_id(obj_type, &call.name)
            .ok_or_else(|| CodegenError::UndefinedFunction(call.name.clone()))?;

        let is_system_method = self.is_system_method(obj_type, &call.name);

        if is_system_method {
            self.emit(Instruction::CALLSYS {
                sys_func_id: method_id,
            });
        } else {
            self.emit(Instruction::CALL { func_id: method_id });
        }

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
        let type_info = self.analyzer.symbol_table.get_type(type_id)?;
        let method_names = type_info.methods.get(method_name)?;
        let full_name = method_names.first()?;
        let func_info = self.analyzer.symbol_table.get_function(full_name)?;

        if func_info.is_system_func {
            func_info.system_func_id
        } else {
            Some(func_info.address)
        }
    }

    fn is_system_method(&self, type_id: TypeId, method_name: &str) -> bool {
        if let Some(type_info) = self.analyzer.symbol_table.get_type(type_id) {
            if let Some(method_names) = type_info.methods.get(method_name) {
                if let Some(full_name) = method_names.first() {
                    if let Some(func_info) = self.analyzer.symbol_table.get_function(full_name) {
                        return func_info.is_system_func;
                    }
                }
            }
        }
        false
    }

    fn get_method_return_type(&self, type_id: TypeId, method_name: &str) -> TypeId {
        if let Some(type_info) = self.analyzer.symbol_table.get_type(type_id) {
            if let Some(method_names) = type_info.methods.get(method_name) {
                if let Some(full_name) = method_names.first() {
                    if let Some(func_info) = self.analyzer.symbol_table.get_function(full_name) {
                        return func_info.return_type;
                    }
                }
            }
        }
        TYPE_VOID
    }

    fn generate_index_access(&mut self, array: &Expr, indices: &[IndexArg]) -> CodegenResult<u32> {
        let array_var = self.generate_expr(array)?;
        let array_type = self.get_expr_type(array);

        if indices.len() != 1 {
            return Err(CodegenError::NotImplemented(
                "multi-dimensional indexing".to_string(),
            ));
        }

        let index_var = self.generate_expr(&indices[0].value)?;

        if let Some(op_index_id) = self.find_operator_method(array_type, "opIndex") {
            let is_system = self.is_system_method(array_type, "opIndex");

            self.emit(Instruction::PshV { var: array_var });
            self.emit(Instruction::PshV { var: index_var });

            if is_system {
                self.emit(Instruction::CALLSYS {
                    sys_func_id: op_index_id,
                });
            } else {
                self.emit(Instruction::CALL {
                    func_id: op_index_id,
                });
            }

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
            "array indexing - type doesn't implement opIndex operator".to_string(),
        ))
    }

    fn find_operator_method(&self, type_id: TypeId, op_name: &str) -> Option<u32> {
        let type_info = self.analyzer.symbol_table.get_type(type_id)?;
        let method_names = type_info.methods.get(op_name)?;
        let full_name = method_names.first()?;
        let func_info = self.analyzer.symbol_table.get_function(full_name)?;

        if func_info.is_system_func {
            func_info.system_func_id
        } else {
            Some(func_info.address)
        }
    }

    fn generate_functor_call(&mut self, functor: &Expr, args: &[Arg]) -> CodegenResult<u32> {
        let functor_var = self.generate_expr(functor)?;
        let functor_type = self.get_expr_type(functor);

        if let Some(op_call_id) = self.find_operator_method(functor_type, "opCall") {
            let is_system = self.is_system_method(functor_type, "opCall");

            self.emit(Instruction::PshV { var: functor_var });

            for arg in args.iter().rev() {
                let arg_var = self.generate_expr(&arg.value)?;
                self.emit(Instruction::PshV { var: arg_var });

                if self.is_temp_var(arg_var) {
                    self.free_temp(arg_var);
                }
            }

            if is_system {
                self.emit(Instruction::CALLSYS {
                    sys_func_id: op_call_id,
                });
            } else {
                self.emit(Instruction::CALL {
                    func_id: op_call_id,
                });
            }

            let result = self.allocate_temp(TYPE_INT32);
            self.emit(Instruction::PopR);
            self.emit(Instruction::CpyRtoV { var: result });

            if self.is_temp_var(functor_var) {
                self.free_temp(functor_var);
            }

            return Ok(result);
        }

        Err(CodegenError::NotImplemented(
            "functor call - type doesn't implement opCall operator".to_string(),
        ))
    }

    fn generate_ternary(
        &mut self,
        cond: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
    ) -> CodegenResult<u32> {
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

    fn generate_func_call(&mut self, call: &FuncCall) -> CodegenResult<u32> {
        for arg in call.args.iter().rev() {
            let arg_var = self.generate_expr(&arg.value)?;
            self.emit(Instruction::PshV { var: arg_var });

            if self.is_temp_var(arg_var) {
                self.free_temp(arg_var);
            }
        }

        let func_info = self
            .analyzer
            .symbol_table
            .get_function(&call.name)
            .ok_or_else(|| CodegenError::UndefinedFunction(call.name.clone()))?;

        if func_info.is_system_func {
            if let Some(sys_func_id) = func_info.system_func_id {
                self.emit(Instruction::CALLSYS { sys_func_id });
            } else {
                return Err(CodegenError::Internal(
                    "System function missing system_func_id".to_string(),
                ));
            }
        } else {
            self.emit(Instruction::CALL {
                func_id: func_info.address,
            });
        }

        let result = self.allocate_temp(func_info.return_type);

        if func_info.return_type != TYPE_VOID {
            self.emit(Instruction::PopR);
            self.emit(Instruction::CpyRtoV { var: result });
        }

        Ok(result)
    }

    fn find_construct_behaviour(&self, type_id: TypeId, arg_count: usize) -> CodegenResult<u32> {
        let engine = self.analyzer.engine.read().unwrap();

        for obj_type in engine.object_types.values() {
            if obj_type.type_id == type_id {
                for behaviour in &obj_type.behaviours {
                    if behaviour.behaviour_type == BehaviourType::Construct {
                        if behaviour.params.len() == arg_count {
                            return Ok(behaviour.function_id);
                        }
                    }
                }
            }
        }

        Err(CodegenError::UndefinedFunction("constructor".to_string()))
    }

    fn generate_cast(&mut self, target_type: &Type, expr: &Expr) -> CodegenResult<u32> {
        let source_var = self.generate_expr(expr)?;
        let source_type = self.get_expr_type(expr);
        let target_type_id = self
            .analyzer
            .symbol_table
            .resolve_type_from_ast(target_type);

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
            (TYPE_FLOAT, TYPE_UINT32) => Instruction::fTOu { var },

            (TYPE_DOUBLE, TYPE_INT32) => Instruction::dTOi { var },
            (TYPE_DOUBLE, TYPE_UINT32) => Instruction::dTOu { var },
            (TYPE_DOUBLE, TYPE_FLOAT) => Instruction::dTOf { var },
            (TYPE_DOUBLE, TYPE_INT64) => Instruction::dTOi64 { var },
            (TYPE_DOUBLE, TYPE_UINT64) => Instruction::dTOu64 { var },

            (TYPE_INT64, TYPE_INT32) => Instruction::i64TOi { var },
            (TYPE_INT64, TYPE_FLOAT) => Instruction::i64TOf { var },
            (TYPE_INT64, TYPE_DOUBLE) => Instruction::i64TOd { var },

            (TYPE_UINT64, TYPE_FLOAT) => Instruction::u64TOf { var },
            (TYPE_UINT64, TYPE_DOUBLE) => Instruction::u64TOd { var },
            (TYPE_UINT32, TYPE_INT64) => Instruction::uTOi64 { var },
            (TYPE_UINT32, TYPE_FLOAT) => Instruction::uTOf { var },
            (TYPE_UINT32, TYPE_DOUBLE) => Instruction::uTOd { var },

            _ => return,
        };

        self.emit(instr);
    }

    // src/compiler/compiler.rs - continued

    fn generate_lambda(&mut self, lambda: &Lambda) -> CodegenResult<u32> {
        let lambda_name = format!("$lambda_{}", self.lambda_count);
        let lambda_count_save = self.lambda_count;
        self.lambda_count += 1;

        let mut return_type = None;
        for stmt in &lambda.body.statements {
            if let Statement::Return(ret) = stmt {
                if let Some(value) = &ret.value {
                    return_type = Some(self.get_expr_type(value));
                    break;
                }
            }
        }

        let lambda_func = Func {
            modifiers: vec![],
            visibility: None,
            return_type: return_type.map(|type_id| {
                if let Some(type_info) = self.analyzer.symbol_table.get_type(type_id) {
                    Type {
                        is_const: false,
                        scope: crate::parser::ast::Scope {
                            is_global: false,
                            path: vec![],
                        },
                        datatype: DataType::PrimType(type_info.name.clone()),
                        template_types: vec![],
                        modifiers: vec![],
                    }
                } else {
                    Type {
                        is_const: false,
                        scope: crate::parser::ast::Scope {
                            is_global: false,
                            path: vec![],
                        },
                        datatype: DataType::Auto,
                        template_types: vec![],
                        modifiers: vec![],
                    }
                }
            }),
            is_ref: false,
            name: lambda_name.clone(),
            params: lambda
                .params
                .iter()
                .map(|p| Param {
                    param_type: p.param_type.clone().unwrap_or_else(|| Type {
                        is_const: false,
                        scope: crate::parser::ast::Scope {
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

        let saved_function = self.current_function.clone();
        let saved_class = self.current_class.clone();
        let saved_namespace = self.current_namespace.clone();

        self.generate_function(&lambda_func)?;

        self.current_function = saved_function;
        self.current_class = saved_class;
        self.current_namespace = saved_namespace;
        self.lambda_count = lambda_count_save + 1;

        let func_id = self
            .analyzer
            .symbol_table
            .get_function(&lambda_name)
            .map(|f| f.address)
            .unwrap_or(0);

        let result = self.allocate_temp(TYPE_VOID);
        self.emit(Instruction::FuncPtr { func_id });
        self.emit(Instruction::PopR);
        self.emit(Instruction::CpyRtoV { var: result });

        Ok(result)
    }

    fn generate_init_list_expr(&mut self, init_list: &InitList) -> CodegenResult<()> {
        self.emit(Instruction::BeginInitList);

        let element_type = if let Some(first_item) = init_list.items.first() {
            match first_item {
                InitListItem::Expr(expr) => self.get_expr_type(expr),
                InitListItem::InitList(_) => TYPE_VOID,
            }
        } else {
            TYPE_VOID
        };

        for item in &init_list.items {
            match item {
                InitListItem::Expr(expr) => {
                    let item_var = self.generate_expr(expr)?;

                    self.emit(Instruction::PshV { var: item_var });
                    self.emit(Instruction::AddToInitList);

                    if self.is_temp_var(item_var) {
                        self.free_temp(item_var);
                    }
                }
                InitListItem::InitList(nested) => {
                    self.generate_init_list_expr(nested)?;
                    self.emit(Instruction::AddToInitList);
                }
            }
        }

        self.emit(Instruction::EndInitList {
            element_type,
            count: init_list.items.len() as u32,
        });

        Ok(())
    }

    fn get_expr_type(&self, expr: &Expr) -> TypeId {
        self.analyzer
            .symbol_table
            .get_expr_type(expr)
            .unwrap_or(TYPE_VOID)
    }

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

    fn allocate_temp(&mut self, _type_id: TypeId) -> u32 {
        if let Some(func_ctx) = &self.current_function {
            let func_full_name = if let Some(class_name) = &self.current_class {
                format!("{}::{}", class_name, func_ctx.name)
            } else if !self.current_namespace.is_empty() {
                format!("{}::{}", self.current_namespace.join("::"), func_ctx.name)
            } else {
                func_ctx.name.clone()
            };

            if let Some(func_locals) = self
                .analyzer
                .symbol_table
                .get_function_locals(&func_full_name)
            {
                return func_locals.total_count as u32 + self.lambda_count;
            }
        }

        let temp = self.lambda_count;
        self.lambda_count += 1;
        temp
    }

    fn free_temp(&mut self, _var: u32) {}

    fn is_temp_var(&self, var: u32) -> bool {
        if let Some(func_ctx) = &self.current_function {
            let func_full_name = if let Some(class_name) = &self.current_class {
                format!("{}::{}", class_name, func_ctx.name)
            } else if !self.current_namespace.is_empty() {
                format!("{}::{}", self.current_namespace.join("::"), func_ctx.name)
            } else {
                func_ctx.name.clone()
            };

            if let Some(func_locals) = self
                .analyzer
                .symbol_table
                .get_function_locals(&func_full_name)
            {
                return var >= func_locals.total_count as u32;
            }
        }

        true
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

    fn is_value_type(&self, type_id: TypeId) -> bool {
        if type_id <= TYPE_STRING {
            return false;
        }

        self.analyzer
            .symbol_table
            .get_type(type_id)
            .map(|t| t.flags.contains(TypeFlags::VALUE_TYPE))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use crate::compiler::bytecode::Instruction;
    use crate::core::engine::EngineInner;
    use crate::core::types::{allocate_type_id, AccessSpecifier, BehaviourInfo, BehaviourType, GlobalFunction, MethodParam, ObjectMethod, ObjectProperty, ObjectType, ScriptValue, TypeFlags, TYPE_INT32, TYPE_STRING, TYPE_VOID};
    use crate::parser::ast::{
        Arg, BinaryOp, Case, CasePattern, Class, ClassMember, DataType, DoWhileStmt, Expr, ForInit,
        ForStmt, Func, FuncCall, IfStmt, InitList, InitListItem, Literal, Namespace, Param,
        PostfixOp, ReturnStmt, Scope, Script, ScriptNode, StatBlock, Statement, SwitchStmt, Type,
        UnaryOp, Var, VarDecl, VarInit, WhileStmt,
    };
    use crate::Compiler;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};
    // ==================== TEST HELPERS ====================

    fn create_test_engine() -> Arc<RwLock<EngineInner>> {
        Arc::new(RwLock::new(EngineInner {
            object_types: HashMap::new(),
            enum_types: HashMap::new(),
            interface_types: HashMap::new(),
            funcdefs: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
        }))
    }

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

    fn build_bool_type() -> Type {
        Type {
            is_const: false,
            scope: Scope {
                is_global: false,
                path: vec![],
            },
            datatype: DataType::PrimType("bool".to_string()),
            template_types: vec![],
            modifiers: vec![],
        }
    }

    fn build_float_type() -> Type {
        Type {
            is_const: false,
            scope: Scope {
                is_global: false,
                path: vec![],
            },
            datatype: DataType::PrimType("float".to_string()),
            template_types: vec![],
            modifiers: vec![],
        }
    }

    fn build_void_type() -> Type {
        Type {
            is_const: false,
            scope: Scope {
                is_global: false,
                path: vec![],
            },
            datatype: DataType::PrimType("void".to_string()),
            template_types: vec![],
            modifiers: vec![],
        }
    }

    fn build_class_type(name: &str) -> Type {
        Type {
            is_const: false,
            scope: Scope {
                is_global: false,
                path: vec![],
            },
            datatype: DataType::Identifier(name.to_string()),
            template_types: vec![],
            modifiers: vec![],
        }
    }

    fn build_int_literal(value: i32) -> Expr {
        Expr::Literal(Literal::Number(value.to_string()))
    }

    fn build_bool_literal(value: bool) -> Expr {
        Expr::Literal(Literal::Bool(value))
    }

    fn build_float_literal(value: f32) -> Expr {
        Expr::Literal(Literal::Number(format!("{:.1}f", value)))
    }

    fn build_string_literal(value: &str) -> Expr {
        Expr::Literal(Literal::String(value.to_string()))
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

    fn build_func_with_params(
        name: &str,
        return_type: Option<Type>,
        params: Vec<Param>,
        body: StatBlock,
    ) -> Func {
        Func {
            modifiers: vec![],
            visibility: None,
            return_type,
            is_ref: false,
            name: name.to_string(),
            params,
            is_const: false,
            attributes: vec![],
            body: Some(body),
        }
    }

    fn build_param(name: &str, param_type: Type) -> Param {
        Param {
            param_type,
            type_mod: None,
            name: Some(name.to_string()),
            default_value: None,
            is_variadic: false,
        }
    }

    // ==================== INTEGRATION TESTS ====================
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

        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "test");
        assert!(!module.instructions.is_empty());
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::RET { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::SetV { .. }))
        );
    }

    #[test]
    fn test_compile_all_literal_types() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
                StatBlock {
                    statements: vec![
                        Statement::Expr(Some(build_int_literal(42))),
                        Statement::Expr(Some(build_bool_literal(true))),
                        Statement::Expr(Some(build_float_literal(3.14))),
                        Statement::Expr(Some(build_string_literal("hello"))),
                        Statement::Expr(Some(Expr::Literal(Literal::Number("42u".to_string())))),
                        Statement::Expr(Some(Expr::Literal(Literal::Number("42ll".to_string())))),
                        Statement::Expr(Some(Expr::Literal(Literal::Number("42ull".to_string())))),
                        Statement::Expr(Some(Expr::Literal(Literal::Number("3.14".to_string())))),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        // Verify all literal types are generated
        assert!(module.instructions.iter().any(|i| matches!(
            i,
            Instruction::SetV {
                value: ScriptValue::Int32(_),
                ..
            }
        )));
        assert!(module.instructions.iter().any(|i| matches!(
            i,
            Instruction::SetV {
                value: ScriptValue::Bool(_),
                ..
            }
        )));
        assert!(module.instructions.iter().any(|i| matches!(
            i,
            Instruction::SetV {
                value: ScriptValue::Float(_),
                ..
            }
        )));
        assert!(module.instructions.iter().any(|i| matches!(
            i,
            Instruction::SetV {
                value: ScriptValue::UInt32(_),
                ..
            }
        )));
        assert!(module.instructions.iter().any(|i| matches!(
            i,
            Instruction::SetV {
                value: ScriptValue::Int64(_),
                ..
            }
        )));
        assert!(module.instructions.iter().any(|i| matches!(
            i,
            Instruction::SetV {
                value: ScriptValue::UInt64(_),
                ..
            }
        )));
        assert!(module.instructions.iter().any(|i| matches!(
            i,
            Instruction::SetV {
                value: ScriptValue::Double(_),
                ..
            }
        )));
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::Str { .. }))
        );
    }

    #[test]
    fn test_compile_all_arithmetic_ops() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
                StatBlock {
                    statements: vec![
                        Statement::Expr(Some(build_binary(
                            build_int_literal(10),
                            BinaryOp::Add,
                            build_int_literal(5),
                        ))),
                        Statement::Expr(Some(build_binary(
                            build_int_literal(10),
                            BinaryOp::Sub,
                            build_int_literal(5),
                        ))),
                        Statement::Expr(Some(build_binary(
                            build_int_literal(3),
                            BinaryOp::Mul,
                            build_int_literal(4),
                        ))),
                        Statement::Expr(Some(build_binary(
                            build_int_literal(20),
                            BinaryOp::Div,
                            build_int_literal(4),
                        ))),
                        Statement::Expr(Some(build_binary(
                            build_int_literal(10),
                            BinaryOp::Mod,
                            build_int_literal(3),
                        ))),
                        Statement::Expr(Some(build_binary(
                            build_int_literal(2),
                            BinaryOp::Pow,
                            build_int_literal(8),
                        ))),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

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
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::DIVi { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::MODi { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::POWi { .. }))
        );
    }

    #[test]
    fn test_compile_float_operations() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
                StatBlock {
                    statements: vec![
                        Statement::Expr(Some(build_binary(
                            build_float_literal(3.14),
                            BinaryOp::Add,
                            build_float_literal(2.86),
                        ))),
                        Statement::Expr(Some(build_binary(
                            build_float_literal(10.0),
                            BinaryOp::Sub,
                            build_float_literal(3.5),
                        ))),
                        Statement::Expr(Some(build_binary(
                            build_float_literal(2.5),
                            BinaryOp::Mul,
                            build_float_literal(4.0),
                        ))),
                        Statement::Expr(Some(build_binary(
                            build_float_literal(10.0),
                            BinaryOp::Div,
                            build_float_literal(2.0),
                        ))),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::ADDf { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::SUBf { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::MULf { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::DIVf { .. }))
        );
    }

    #[test]
    fn test_compile_all_comparisons() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
                StatBlock {
                    statements: vec![
                        // ✅ Store results in variables so they're not optimized away
                        Statement::Var(Var {
                            visibility: None,
                            var_type: build_bool_type(),
                            declarations: vec![VarDecl {
                                name: "r1".to_string(),
                                initializer: Some(VarInit::Expr(build_binary(
                                    build_int_literal(5),
                                    BinaryOp::Eq,
                                    build_int_literal(5),
                                ))),
                            }],
                        }),
                        Statement::Var(Var {
                            visibility: None,
                            var_type: build_bool_type(),
                            declarations: vec![VarDecl {
                                name: "r2".to_string(),
                                initializer: Some(VarInit::Expr(build_binary(
                                    build_int_literal(5),
                                    BinaryOp::Ne,
                                    build_int_literal(3),
                                ))),
                            }],
                        }),
                        Statement::Var(Var {
                            visibility: None,
                            var_type: build_bool_type(),
                            declarations: vec![VarDecl {
                                name: "r3".to_string(),
                                initializer: Some(VarInit::Expr(build_binary(
                                    build_int_literal(3),
                                    BinaryOp::Lt,
                                    build_int_literal(5),
                                ))),
                            }],
                        }),
                        Statement::Var(Var {
                            visibility: None,
                            var_type: build_bool_type(),
                            declarations: vec![VarDecl {
                                name: "r4".to_string(),
                                initializer: Some(VarInit::Expr(build_binary(
                                    build_int_literal(5),
                                    BinaryOp::Le,
                                    build_int_literal(5),
                                ))),
                            }],
                        }),
                        Statement::Var(Var {
                            visibility: None,
                            var_type: build_bool_type(),
                            declarations: vec![VarDecl {
                                name: "r5".to_string(),
                                initializer: Some(VarInit::Expr(build_binary(
                                    build_int_literal(5),
                                    BinaryOp::Gt,
                                    build_int_literal(3),
                                ))),
                            }],
                        }),
                        Statement::Var(Var {
                            visibility: None,
                            var_type: build_bool_type(),
                            declarations: vec![VarDecl {
                                name: "r6".to_string(),
                                initializer: Some(VarInit::Expr(build_binary(
                                    build_int_literal(5),
                                    BinaryOp::Ge,
                                    build_int_literal(5),
                                ))),
                            }],
                        }),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

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
                .any(|i| matches!(i, Instruction::TZ))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::TNZ))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::TS))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::TNS))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::TP))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::TNP))
        );
    }

    #[test]
    fn test_compile_all_bitwise_ops() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
                StatBlock {
                    statements: vec![
                        Statement::Expr(Some(build_binary(
                            build_int_literal(0xFF),
                            BinaryOp::BitAnd,
                            build_int_literal(0x0F),
                        ))),
                        Statement::Expr(Some(build_binary(
                            build_int_literal(0xF0),
                            BinaryOp::BitOr,
                            build_int_literal(0x0F),
                        ))),
                        Statement::Expr(Some(build_binary(
                            build_int_literal(0xFF),
                            BinaryOp::BitXor,
                            build_int_literal(0x0F),
                        ))),
                        Statement::Expr(Some(build_binary(
                            build_int_literal(1),
                            BinaryOp::Shl,
                            build_int_literal(3),
                        ))),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::BAND { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::BOR { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::BXOR { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::BSLL { .. }))
        );
    }

    #[test]
    fn test_compile_all_unary_ops() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
                StatBlock {
                    statements: vec![
                        Statement::Expr(Some(Expr::Unary(
                            UnaryOp::Neg,
                            Box::new(build_int_literal(42)),
                        ))),
                        Statement::Expr(Some(Expr::Unary(
                            UnaryOp::Not,
                            Box::new(build_bool_literal(true)),
                        ))),
                        Statement::Expr(Some(Expr::Unary(
                            UnaryOp::BitNot,
                            Box::new(build_int_literal(0xFF)),
                        ))),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::NEGi { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::NOT { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::BNOT { .. }))
        );
    }

    #[test]
    fn test_compile_ternary() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_int_type()),
                StatBlock {
                    statements: vec![Statement::Return(ReturnStmt {
                        value: Some(Expr::Ternary(
                            Box::new(build_bool_literal(true)),
                            Box::new(build_int_literal(42)),
                            Box::new(build_int_literal(99)),
                        )),
                    })],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

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
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::CpyV { .. }))
        );
    }

    #[test]
    fn test_compile_nested_arithmetic() {
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

        // Verify operation order
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

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::ADDi { .. }))
        );
        let set_count = module
            .instructions
            .iter()
            .filter(|i| matches!(i, Instruction::SetV { .. }))
            .count();
        assert!(set_count >= 2);
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
    fn test_compile_for_loop() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_int_type()),
                StatBlock {
                    statements: vec![
                        Statement::For(ForStmt {
                            init: ForInit::Var(Var {
                                visibility: None,
                                var_type: build_int_type(),
                                declarations: vec![VarDecl {
                                    name: "i".to_string(),
                                    initializer: Some(VarInit::Expr(build_int_literal(0))),
                                }],
                            }),
                            condition: Some(build_binary(
                                build_var_access("i"),
                                BinaryOp::Lt,
                                build_int_literal(10),
                            )),
                            increment: vec![Expr::Unary(
                                UnaryOp::PreInc,
                                Box::new(build_var_access("i")),
                            )],
                            body: Box::new(Statement::Expr(None)),
                        }),
                        Statement::Return(ReturnStmt {
                            value: Some(build_int_literal(0)),
                        }),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

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
                .any(|i| matches!(i, Instruction::INCi { .. }))
        );

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
    fn test_compile_do_while_loop() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
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
                        Statement::DoWhile(DoWhileStmt {
                            body: Box::new(Statement::Expr(Some(Expr::Unary(
                                UnaryOp::PreInc,
                                Box::new(build_var_access("x")),
                            )))),
                            condition: build_binary(
                                build_var_access("x"),
                                BinaryOp::Lt,
                                build_int_literal(5),
                            ),
                        }),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::INCi { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::JNZ { .. }))
        );
    }

    #[test]
    fn test_compile_function_with_parameters() {
        let script = Script {
            items: vec![ScriptNode::Func(build_func_with_params(
                "add",
                Some(build_int_type()),
                vec![
                    build_param("a", build_int_type()),
                    build_param("b", build_int_type()),
                ],
                StatBlock {
                    statements: vec![Statement::Return(ReturnStmt {
                        value: Some(build_binary(
                            build_var_access("a"),
                            BinaryOp::Add,
                            build_var_access("b"),
                        )),
                    })],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].param_count, 2);
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::ADDi { .. }))
        );
    }

    #[test]
    fn test_compile_function_call() {
        let script = Script {
            items: vec![
                ScriptNode::Func(build_function(
                    "helper",
                    Some(build_int_type()),
                    StatBlock {
                        statements: vec![Statement::Return(ReturnStmt {
                            value: Some(build_int_literal(42)),
                        })],
                    },
                )),
                ScriptNode::Func(build_function(
                    "test",
                    Some(build_int_type()),
                    StatBlock {
                        statements: vec![Statement::Return(ReturnStmt {
                            value: Some(Expr::FuncCall(FuncCall {
                                scope: Scope {
                                    is_global: false,
                                    path: vec![],
                                },
                                name: "helper".to_string(),
                                template_types: vec![],
                                args: vec![],
                            })),
                        })],
                    },
                )),
            ],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert_eq!(module.functions.len(), 2);
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::CALL { .. }))
        );
    }

    #[test]
    fn test_compile_assignment() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
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
                        Statement::Expr(Some(build_binary(
                            build_var_access("x"),
                            BinaryOp::Assign,
                            build_int_literal(42),
                        ))),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::CpyV { .. }))
        );
    }

    #[test]
    fn test_compile_compound_assignment() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
                StatBlock {
                    statements: vec![
                        Statement::Var(Var {
                            visibility: None,
                            var_type: build_int_type(),
                            declarations: vec![VarDecl {
                                name: "x".to_string(),
                                initializer: Some(VarInit::Expr(build_int_literal(10))),
                            }],
                        }),
                        Statement::Expr(Some(build_binary(
                            build_var_access("x"),
                            BinaryOp::AddAssign,
                            build_int_literal(5),
                        ))),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::ADDi { .. }))
        );
    }

    #[test]
    fn test_compile_pre_increment() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
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
                        Statement::Expr(Some(Expr::Unary(
                            UnaryOp::PreInc,
                            Box::new(build_var_access("x")),
                        ))),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::INCi { .. }))
        );
    }

    #[test]
    fn test_compile_post_increment() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
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
                        Statement::Expr(Some(Expr::Postfix(
                            Box::new(build_var_access("x")),
                            PostfixOp::PostInc,
                        ))),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::INCi { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::CpyV { .. }))
        );
    }

    #[test]
    fn test_compile_logical_and() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_bool_type()),
                StatBlock {
                    statements: vec![Statement::Return(ReturnStmt {
                        value: Some(build_binary(
                            build_bool_literal(true),
                            BinaryOp::And,
                            build_bool_literal(false),
                        )),
                    })],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::JNZ { .. }))
        );
    }

    #[test]
    fn test_compile_logical_or() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_bool_type()),
                StatBlock {
                    statements: vec![Statement::Return(ReturnStmt {
                        value: Some(build_binary(
                            build_bool_literal(false),
                            BinaryOp::Or,
                            build_bool_literal(true),
                        )),
                    })],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::JNZ { .. }))
        );
    }

    #[test]
    fn test_compile_cast() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_float_type()),
                StatBlock {
                    statements: vec![Statement::Return(ReturnStmt {
                        value: Some(Expr::Cast(
                            build_float_type(),
                            Box::new(build_int_literal(42)),
                        )),
                    })],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::iTOf { .. }))
        );
    }

    #[test]
    fn test_compile_type_conversion() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
                StatBlock {
                    statements: vec![
                        Statement::Var(Var {
                            visibility: None,
                            var_type: build_int_type(),
                            declarations: vec![VarDecl {
                                name: "i".to_string(),
                                initializer: Some(VarInit::Expr(build_int_literal(42))),
                            }],
                        }),
                        Statement::Var(Var {
                            visibility: None,
                            var_type: build_float_type(),
                            declarations: vec![VarDecl {
                                name: "f".to_string(),
                                initializer: Some(VarInit::Expr(Expr::Cast(
                                    build_float_type(),
                                    Box::new(build_var_access("i")),
                                ))),
                            }],
                        }),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::iTOf { .. }))
        );
    }

    #[test]
    fn test_compile_switch_statement() {
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
                                initializer: Some(VarInit::Expr(build_int_literal(1))),
                            }],
                        }),
                        Statement::Switch(SwitchStmt {
                            value: build_var_access("x"),
                            cases: vec![
                                Case {
                                    pattern: CasePattern::Value(build_int_literal(1)),
                                    statements: vec![Statement::Return(ReturnStmt {
                                        value: Some(build_int_literal(10)),
                                    })],
                                },
                                Case {
                                    pattern: CasePattern::Default,
                                    statements: vec![Statement::Return(ReturnStmt {
                                        value: Some(build_int_literal(0)),
                                    })],
                                },
                            ],
                        }),
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

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
                .any(|i| matches!(i, Instruction::JNZ { .. }))
        );
    }

    #[test]
    fn test_compile_break_in_loop() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
                StatBlock {
                    statements: vec![Statement::While(WhileStmt {
                        condition: build_bool_literal(true),
                        body: Box::new(Statement::Break),
                    })],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        let jmp_count = module
            .instructions
            .iter()
            .filter(|i| matches!(i, Instruction::JMP { .. }))
            .count();
        assert!(jmp_count >= 1);
    }

    #[test]
    fn test_compile_continue_in_loop() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
                StatBlock {
                    statements: vec![Statement::While(WhileStmt {
                        condition: build_bool_literal(true),
                        body: Box::new(Statement::Continue),
                    })],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        let jmp_count = module
            .instructions
            .iter()
            .filter(|i| matches!(i, Instruction::JMP { .. }))
            .count();
        assert!(jmp_count >= 1);
    }

    #[test]
    fn test_compile_global_variable() {
        let script = Script {
            items: vec![
                ScriptNode::Var(Var {
                    visibility: None,
                    var_type: build_int_type(),
                    declarations: vec![VarDecl {
                        name: "globalVar".to_string(),
                        initializer: Some(VarInit::Expr(build_int_literal(42))),
                    }],
                }),
                ScriptNode::Func(build_function(
                    "test",
                    Some(build_int_type()),
                    StatBlock {
                        statements: vec![Statement::Return(ReturnStmt {
                            value: Some(build_var_access("globalVar")),
                        })],
                    },
                )),
            ],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert_eq!(module.globals.len(), 1);
        assert_eq!(module.globals[0].name, "globalVar");
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::SetG { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::CpyGtoV { .. }))
        );
    }

    #[test]
    fn test_compile_class_with_members() {
        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "MyClass".to_string(),
                extends: vec![],
                members: vec![
                    ClassMember::Var(Var {
                        visibility: None,
                        var_type: build_int_type(),
                        declarations: vec![VarDecl {
                            name: "value".to_string(),
                            initializer: Some(VarInit::Expr(build_int_literal(100))),
                        }],
                    }),
                    ClassMember::Func(build_function(
                        "getValue",
                        Some(build_int_type()),
                        StatBlock {
                            statements: vec![Statement::Return(ReturnStmt {
                                value: Some(Expr::Postfix(
                                    Box::new(build_var_access("this")),
                                    PostfixOp::MemberAccess("value".to_string()),
                                )),
                            })],
                        },
                    )),
                ],
            })],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "MyClass::getValue");
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::GetThisProperty { .. }))
        );
    }

    #[test]
    fn test_compile_class_constructor() {
        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "MyClass".to_string(),
                extends: vec![],
                members: vec![
                    ClassMember::Var(Var {
                        visibility: None,
                        var_type: build_int_type(),
                        declarations: vec![VarDecl {
                            name: "value".to_string(),
                            initializer: None,
                        }],
                    }),
                    ClassMember::Func(build_func_with_params(
                        "MyClass",
                        None,
                        vec![build_param("v", build_int_type())],
                        StatBlock {
                            statements: vec![Statement::Expr(Some(build_binary(
                                Expr::Postfix(
                                    Box::new(build_var_access("this")),
                                    PostfixOp::MemberAccess("value".to_string()),
                                ),
                                BinaryOp::Assign,
                                build_var_access("v"),
                            )))],
                        },
                    )),
                ],
            })],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert_eq!(module.functions.len(), 1);
        // ✅ Fix: Accept SetThisProperty (optimized) instead of SetProperty
        assert!(module.instructions.iter().any(|i| {
            matches!(i, Instruction::SetThisProperty { .. })
                || matches!(i, Instruction::SetProperty { .. })
        }));
    }

    #[test]
    fn test_compile_class_with_initializers() {
        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "Player".to_string(),
                extends: vec![],
                members: vec![
                    ClassMember::Var(Var {
                        visibility: None,
                        var_type: build_int_type(),
                        declarations: vec![VarDecl {
                            name: "health".to_string(),
                            initializer: Some(VarInit::Expr(build_int_literal(100))),
                        }],
                    }),
                    ClassMember::Func(build_function(
                        "Player",
                        None,
                        StatBlock { statements: vec![] },
                    )),
                ],
            })],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::SetThisProperty { .. }))
        );
    }

    #[test]
    fn test_compile_member_access() {
        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "MyClass".to_string(),
                    extends: vec![],
                    members: vec![ClassMember::Var(Var {
                        visibility: None,
                        var_type: build_int_type(),
                        declarations: vec![VarDecl {
                            name: "value".to_string(),
                            initializer: None,
                        }],
                    })],
                }),
                ScriptNode::Func(build_function(
                    "test",
                    Some(build_int_type()),
                    StatBlock {
                        statements: vec![
                            Statement::Var(Var {
                                visibility: None,
                                var_type: build_class_type("MyClass"),
                                declarations: vec![VarDecl {
                                    name: "obj".to_string(),
                                    initializer: None,
                                }],
                            }),
                            Statement::Return(ReturnStmt {
                                value: Some(Expr::Postfix(
                                    Box::new(build_var_access("obj")),
                                    PostfixOp::MemberAccess("value".to_string()),
                                )),
                            }),
                        ],
                    },
                )),
            ],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::GetProperty { .. }))
        );
    }

    #[test]
    fn test_compile_member_assignment() {
        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "MyClass".to_string(),
                    extends: vec![],
                    members: vec![ClassMember::Var(Var {
                        visibility: None,
                        var_type: build_int_type(),
                        declarations: vec![VarDecl {
                            name: "value".to_string(),
                            initializer: None,
                        }],
                    })],
                }),
                ScriptNode::Func(build_function(
                    "test",
                    Some(build_void_type()),
                    StatBlock {
                        statements: vec![
                            Statement::Var(Var {
                                visibility: None,
                                var_type: build_class_type("MyClass"),
                                declarations: vec![VarDecl {
                                    name: "obj".to_string(),
                                    initializer: None,
                                }],
                            }),
                            Statement::Expr(Some(build_binary(
                                Expr::Postfix(
                                    Box::new(build_var_access("obj")),
                                    PostfixOp::MemberAccess("value".to_string()),
                                ),
                                BinaryOp::Assign,
                                build_int_literal(42),
                            ))),
                        ],
                    },
                )),
            ],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::SetProperty { .. }))
        );
    }

    #[test]
    fn test_compile_method_call() {
        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "MyClass".to_string(),
                    extends: vec![],
                    members: vec![ClassMember::Func(build_function(
                        "method",
                        Some(build_int_type()),
                        StatBlock {
                            statements: vec![Statement::Return(ReturnStmt {
                                value: Some(build_int_literal(42)),
                            })],
                        },
                    ))],
                }),
                ScriptNode::Func(build_function(
                    "test",
                    Some(build_int_type()),
                    StatBlock {
                        statements: vec![
                            Statement::Var(Var {
                                visibility: None,
                                var_type: build_class_type("MyClass"),
                                declarations: vec![VarDecl {
                                    name: "obj".to_string(),
                                    initializer: None,
                                }],
                            }),
                            Statement::Return(ReturnStmt {
                                value: Some(Expr::Postfix(
                                    Box::new(build_var_access("obj")),
                                    PostfixOp::MemberCall(FuncCall {
                                        scope: Scope {
                                            is_global: false,
                                            path: vec![],
                                        },
                                        name: "method".to_string(),
                                        template_types: vec![],
                                        args: vec![],
                                    }),
                                )),
                            }),
                        ],
                    },
                )),
            ],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        let call_count = module
            .instructions
            .iter()
            .filter(|i| matches!(i, Instruction::CALL { .. }))
            .count();
        assert!(call_count >= 1);
    }

    #[test]
    fn test_compile_namespace() {
        let script = Script {
            items: vec![ScriptNode::Namespace(Namespace {
                name: vec!["MyNamespace".to_string()],
                items: vec![ScriptNode::Func(build_function(
                    "test",
                    Some(build_void_type()),
                    StatBlock { statements: vec![] },
                ))],
            })],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "MyNamespace::test");
    }

    #[test]
    fn test_compile_class_inheritance() {
        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "Base".to_string(),
                    extends: vec![],
                    members: vec![ClassMember::Func(build_function(
                        "Base",
                        None,
                        StatBlock { statements: vec![] },
                    ))],
                }),
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "Derived".to_string(),
                    extends: vec!["Base".to_string()],
                    members: vec![ClassMember::Func(build_function(
                        "Derived",
                        None,
                        StatBlock { statements: vec![] },
                    ))],
                }),
            ],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert_eq!(module.functions.len(), 2);
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::PshR))
        );
    }

    #[test]
    fn test_compile_init_list() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
                StatBlock {
                    statements: vec![Statement::Expr(Some(Expr::InitList(InitList {
                        items: vec![
                            InitListItem::Expr(build_int_literal(1)),
                            InitListItem::Expr(build_int_literal(2)),
                            InitListItem::Expr(build_int_literal(3)),
                        ],
                    })))],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::BeginInitList))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::EndInitList { .. }))
        );

        let add_count = module
            .instructions
            .iter()
            .filter(|i| matches!(i, Instruction::AddToInitList))
            .count();
        assert_eq!(add_count, 3);
    }

    #[test]
    fn test_compile_nested_init_list() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
                StatBlock {
                    statements: vec![Statement::Expr(Some(Expr::InitList(InitList {
                        items: vec![
                            InitListItem::InitList(InitList {
                                items: vec![
                                    InitListItem::Expr(build_int_literal(1)),
                                    InitListItem::Expr(build_int_literal(2)),
                                ],
                            }),
                            InitListItem::InitList(InitList {
                                items: vec![
                                    InitListItem::Expr(build_int_literal(3)),
                                    InitListItem::Expr(build_int_literal(4)),
                                ],
                            }),
                        ],
                    })))],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        let begin_count = module
            .instructions
            .iter()
            .filter(|i| matches!(i, Instruction::BeginInitList))
            .count();
        assert_eq!(begin_count, 3);
    }

    #[test]
    fn test_compile_complete_program() {
        let script = Script {
            items: vec![
                ScriptNode::Var(Var {
                    visibility: None,
                    var_type: build_int_type(),
                    declarations: vec![VarDecl {
                        name: "counter".to_string(),
                        initializer: Some(VarInit::Expr(build_int_literal(0))),
                    }],
                }),
                ScriptNode::Func(build_func_with_params(
                    "add",
                    Some(build_int_type()),
                    vec![
                        build_param("a", build_int_type()),
                        build_param("b", build_int_type()),
                    ],
                    StatBlock {
                        statements: vec![Statement::Return(ReturnStmt {
                            value: Some(build_binary(
                                build_var_access("a"),
                                BinaryOp::Add,
                                build_var_access("b"),
                            )),
                        })],
                    },
                )),
                ScriptNode::Func(build_function(
                    "main",
                    Some(build_int_type()),
                    StatBlock {
                        statements: vec![
                            Statement::Var(Var {
                                visibility: None,
                                var_type: build_int_type(),
                                declarations: vec![VarDecl {
                                    name: "result".to_string(),
                                    initializer: Some(VarInit::Expr(Expr::FuncCall(FuncCall {
                                        scope: Scope {
                                            is_global: false,
                                            path: vec![],
                                        },
                                        name: "add".to_string(),
                                        template_types: vec![],
                                        args: vec![
                                            Arg {
                                                name: None,
                                                value: build_int_literal(10),
                                            },
                                            Arg {
                                                name: None,
                                                value: build_int_literal(20),
                                            },
                                        ],
                                    }))),
                                }],
                            }),
                            Statement::Expr(Some(build_binary(
                                build_var_access("counter"),
                                BinaryOp::AddAssign,
                                build_var_access("result"),
                            ))),
                            Statement::Return(ReturnStmt {
                                value: Some(build_var_access("counter")),
                            }),
                        ],
                    },
                )),
            ],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert_eq!(module.functions.len(), 2);
        assert_eq!(module.functions[0].name, "add");
        assert_eq!(module.functions[1].name, "main");
        assert_eq!(module.globals.len(), 1);
        assert_eq!(module.globals[0].name, "counter");
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::CALL { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::CpyGtoV { .. }))
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::CpyVtoG { .. }))
        );
    }

    #[test]
    fn test_string_table_deduplication() {
        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
                StatBlock {
                    statements: vec![
                        Statement::Expr(Some(build_string_literal("Hello"))),
                        Statement::Expr(Some(build_string_literal("World"))),
                        Statement::Expr(Some(build_string_literal("Hello"))), // Duplicate
                    ],
                },
            ))],
        };

        let engine = create_test_engine();
        let compiler = Compiler::new(engine);
        let module = compiler.compile(script).unwrap();

        assert_eq!(module.strings.len(), 2); // Only 2 unique strings
        assert_eq!(module.strings[0], "Hello");
        assert_eq!(module.strings[1], "World");
    }

    // src/compiler/compiler.rs - Add to tests module (after existing tests)

    #[test]
    fn test_compile_registered_global_function() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: HashMap::new(),
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: vec![GlobalFunction {
                name: "print".to_string(),
                return_type_id: TYPE_VOID,
                params: vec![MethodParam {
                    name: "msg".to_string(),
                    type_id: TYPE_STRING,
                    is_ref: true,
                    is_out: false,
                    is_const: true,
                }],
                function_id: 0,
            }],
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let compiler = Compiler::new(engine);

        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
                StatBlock {
                    statements: vec![Statement::Expr(Some(Expr::FuncCall(FuncCall {
                        scope: Scope {
                            is_global: false,
                            path: vec![],
                        },
                        name: "print".to_string(),
                        template_types: vec![],
                        args: vec![Arg {
                            name: None,
                            value: build_string_literal("Hello"),
                        }],
                    })))],
                },
            ))],
        };

        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::CALLSYS { .. })),
            "Should generate CALLSYS for registered function"
        );
    }

    #[test]
    fn test_compile_registered_type_property_access() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: {
                let mut map = HashMap::new();
                map.insert(
                    "Enemy".to_string(),
                    ObjectType {
                        type_id: 100,
                        name: "Enemy".to_string(),
                        flags: TypeFlags::REF_TYPE,
                        properties: vec![ObjectProperty {
                            name: "health".to_string(),
                            type_id: TYPE_INT32,
                            is_handle: false,
                            is_const: false,
                            access: AccessSpecifier::Public,
                        }],
                        methods: vec![],
                        behaviours: vec![],
                        rust_type_id: None,
                    },
                );
                map
            },
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let compiler = Compiler::new(engine);

        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_int_type()),
                StatBlock {
                    statements: vec![
                        Statement::Var(Var {
                            visibility: None,
                            var_type: build_class_type("Enemy"),
                            declarations: vec![VarDecl {
                                name: "enemy".to_string(),
                                initializer: None,
                            }],
                        }),
                        Statement::Return(ReturnStmt {
                            value: Some(Expr::Postfix(
                                Box::new(build_var_access("enemy")),
                                PostfixOp::MemberAccess("health".to_string()),
                            )),
                        }),
                    ],
                },
            ))],
        };

        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::GetProperty { .. })),
            "Should generate GetProperty for registered type property"
        );
    }

    #[test]
    fn test_compile_registered_type_method_call() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: {
                let mut map = HashMap::new();
                map.insert(
                    "Enemy".to_string(),
                    ObjectType {
                        type_id: 100,
                        name: "Enemy".to_string(),
                        flags: TypeFlags::REF_TYPE,
                        properties: vec![],
                        methods: vec![ObjectMethod {
                            name: "takeDamage".to_string(),
                            return_type_id: TYPE_VOID,
                            params: vec![MethodParam {
                                name: "amount".to_string(),
                                type_id: TYPE_INT32,
                                is_ref: false,
                                is_out: false,
                                is_const: false,
                            }],
                            is_const: false,
                            is_virtual: false,
                            is_final: false,
                            access: AccessSpecifier::Public,
                            function_id: 0,
                        }],
                        behaviours: vec![],
                        rust_type_id: None,
                    },
                );
                map
            },
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let compiler = Compiler::new(engine);

        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
                StatBlock {
                    statements: vec![
                        Statement::Var(Var {
                            visibility: None,
                            var_type: build_class_type("Enemy"),
                            declarations: vec![VarDecl {
                                name: "enemy".to_string(),
                                initializer: None,
                            }],
                        }),
                        Statement::Expr(Some(Expr::Postfix(
                            Box::new(build_var_access("enemy")),
                            PostfixOp::MemberCall(FuncCall {
                                scope: Scope {
                                    is_global: false,
                                    path: vec![],
                                },
                                name: "takeDamage".to_string(),
                                template_types: vec![],
                                args: vec![Arg {
                                    name: None,
                                    value: build_int_literal(50),
                                }],
                            }),
                        ))),
                    ],
                },
            ))],
        };

        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::CALLSYS { .. })),
            "Should generate CALLSYS for registered method"
        );
    }

    #[test]
    fn test_compile_registered_type_construction() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: {
                let mut map = HashMap::new();
                map.insert(
                    "Enemy".to_string(),
                    ObjectType {
                        type_id: allocate_type_id(),
                        name: "Enemy".to_string(),
                        flags: TypeFlags::REF_TYPE,
                        properties: vec![],
                        methods: vec![],
                        behaviours: vec![
                            BehaviourInfo {
                                behaviour_type: BehaviourType::Construct,
                                function_id: allocate_type_id(),
                                return_type_id: 100,
                                params: vec![],
                            },
                            BehaviourInfo {
                                behaviour_type: BehaviourType::AddRef,
                                function_id: allocate_type_id(),
                                return_type_id: TYPE_VOID,
                                params: vec![],
                            },
                            BehaviourInfo {
                                behaviour_type: BehaviourType::Release,
                                function_id: allocate_type_id(),
                                return_type_id: TYPE_VOID,
                                params: vec![],
                            },
                        ],
                        rust_type_id: None,
                    },
                );
                map
            },
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let compiler = Compiler::new(engine);

        let script = Script {
            items: vec![ScriptNode::Func(build_function(
                "test",
                Some(build_void_type()),
                StatBlock {
                    statements: vec![Statement::Var(Var {
                        visibility: None,
                        var_type: build_class_type("Enemy"),
                        declarations: vec![VarDecl {
                            name: "enemy".to_string(),
                            initializer: Some(VarInit::Expr(Expr::ConstructCall(
                                build_class_type("Enemy"),
                                vec![],
                            ))),
                        }],
                    })],
                },
            ))],
        };

        let module = compiler.compile(script).unwrap();

        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::Alloc { .. })),
            "Should generate Alloc for construction"
        );
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::CALLSYS { .. })),
            "Should generate CALLSYS for factory behaviour"
        );
    }

    #[test]
    fn test_compile_mixed_script_and_registered_types() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: {
                let mut map = HashMap::new();
                map.insert(
                    "Enemy".to_string(),
                    ObjectType {
                        type_id: 100,
                        name: "Enemy".to_string(),
                        flags: TypeFlags::REF_TYPE,
                        properties: vec![ObjectProperty {
                            name: "health".to_string(),
                            type_id: TYPE_INT32,
                            is_handle: false,
                            is_const: false,
                            access: AccessSpecifier::Public,
                        }],
                        methods: vec![],
                        behaviours: vec![],
                        rust_type_id: None,
                    },
                );
                map
            },
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let compiler = Compiler::new(engine);

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "Player".to_string(),
                    extends: vec![],
                    members: vec![ClassMember::Var(Var {
                        visibility: None,
                        var_type: build_class_type("Enemy"),
                        declarations: vec![VarDecl {
                            name: "target".to_string(),
                            initializer: None,
                        }],
                    })],
                }),
                ScriptNode::Func(build_function(
                    "test",
                    Some(build_int_type()),
                    StatBlock {
                        statements: vec![
                            Statement::Var(Var {
                                visibility: None,
                                var_type: build_class_type("Player"),
                                declarations: vec![VarDecl {
                                    name: "player".to_string(),
                                    initializer: None,
                                }],
                            }),
                            Statement::Return(ReturnStmt {
                                value: Some(Expr::Postfix(
                                    Box::new(Expr::Postfix(
                                        Box::new(build_var_access("player")),
                                        PostfixOp::MemberAccess("target".to_string()),
                                    )),
                                    PostfixOp::MemberAccess("health".to_string()),
                                )),
                            }),
                        ],
                    },
                )),
            ],
        };

        let module = compiler.compile(script).unwrap();

        assert_eq!(module.functions.len(), 1);
        assert!(
            module
                .instructions
                .iter()
                .any(|i| matches!(i, Instruction::GetProperty { .. })),
            "Should access registered type property"
        );
    }
}
