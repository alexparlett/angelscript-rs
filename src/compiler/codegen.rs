use crate::parser::*;
use crate::compiler::bytecode::*;
use crate::compiler::semantic::SemanticAnalyzer;
use std::collections::HashMap;
use crate::compiler::bytecode::{BytecodeModule, GlobalVar, Instruction, TypeInfo};

pub struct CodeGenerator {
    module: BytecodeModule,
    current_address: u32,
    local_vars: HashMap<String, u32>,
    local_count: u32,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            module: BytecodeModule::new(),
            current_address: 0,
            local_vars: HashMap::new(),
            local_count: 0,
        }
    }

    pub fn generate(&mut self, script: &Script, _analyzer: &SemanticAnalyzer) -> BytecodeModule {
        // Generate code for all script items
        for item in &script.items {
            match item {
                ScriptItem::Func(func) => self.generate_function(func),
                ScriptItem::Class(class) => self.generate_class(class),
                ScriptItem::Var(var) => self.generate_global_var(var),
                _ => {}
            }
        }

        self.module.clone()
    }

    fn generate_function(&mut self, func: &Func) {
        let func_address = self.current_address;

        // Reset local variable tracking
        self.local_vars.clear();
        self.local_count = 0;

        // Allocate space for parameters
        for param in &func.params {
            if let Some(name) = &param.name {
                self.local_vars.insert(name.clone(), self.local_count);
                self.local_count += 1;
            }
        }

        // Generate function body
        if let Some(body) = &func.body {
            self.generate_statement_block(body);
        }

        // Add return if not present
        if !matches!(self.module.instructions.last(), Some(Instruction::Return | Instruction::ReturnValue)) {
            self.emit(Instruction::Return);
        }

        // Register function info
        self.module.functions.push(FunctionInfo {
            name: func.name.clone(),
            address: func_address,
            param_count: func.params.len() as u8,
            local_count: self.local_count,
            return_type: 0,
        });
    }

    fn generate_class(&mut self, class: &Class) {
        // Generate methods
        for member in &class.members {
            if let ClassMember::Func(func) = member {
                self.generate_function(func);
            }
        }

        // Register type info
        self.module.types.push(TypeInfo {
            name: class.name.clone(),
            size: 0,
            members: Vec::new(),
            methods: Vec::new(),
        });
    }

    fn generate_global_var(&mut self, var: &Var) {
        for decl in &var.declarations {
            self.module.globals.push(GlobalVar {
                name: decl.name.clone(),
                type_id: 0,
                address: self.module.globals.len() as u32,
            });
        }
    }

    fn generate_statement_block(&mut self, block: &StatBlock) {
        for stmt in &block.statements {
            self.generate_statement(stmt);
        }
    }

    fn generate_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Var(var) => self.generate_var_decl(var),
            Statement::Expr(expr) => {
                if let Some(e) = expr {
                    self.generate_expr(e);
                    self.emit(Instruction::Pop);
                }
            }
            Statement::If(if_stmt) => self.generate_if(if_stmt),
            Statement::While(while_stmt) => self.generate_while(while_stmt),
            Statement::Return(ret) => self.generate_return(ret),
            Statement::Break => {
                // TODO: Implement break
            }
            Statement::Continue => {
                // TODO: Implement continue
            }
            Statement::Block(block) => self.generate_statement_block(block),
            _ => {}
        }
    }

    fn generate_var_decl(&mut self, var: &Var) {
        for decl in &var.declarations {
            let local_idx = self.local_count;
            self.local_vars.insert(decl.name.clone(), local_idx);
            self.local_count += 1;

            if let Some(init) = &decl.initializer {
                match init {
                    VarInit::Expr(expr) => {
                        self.generate_expr(expr);
                        self.emit(Instruction::StoreLocal(local_idx));
                    }
                    _ => {
                        self.emit(Instruction::Push(Value::Int32(0)));
                        self.emit(Instruction::StoreLocal(local_idx));
                    }
                }
            }
        }
    }

    fn generate_if(&mut self, if_stmt: &IfStmt) {
        self.generate_expr(&if_stmt.condition);

        let jump_to_else = self.emit_jump_placeholder(Instruction::JumpIfFalse(0));

        self.generate_statement(&if_stmt.then_branch);

        if let Some(else_branch) = &if_stmt.else_branch {
            let jump_to_end = self.emit_jump_placeholder(Instruction::Jump(0));
            self.patch_jump(jump_to_else);
            self.generate_statement(else_branch);
            self.patch_jump(jump_to_end);
        } else {
            self.patch_jump(jump_to_else);
        }
    }

    fn generate_while(&mut self, while_stmt: &WhileStmt) {
        let loop_start = self.current_address;

        self.generate_expr(&while_stmt.condition);

        let jump_to_end = self.emit_jump_placeholder(Instruction::JumpIfFalse(0));

        self.generate_statement(&while_stmt.body);

        self.emit(Instruction::Jump(loop_start));

        self.patch_jump(jump_to_end);
    }

    fn generate_return(&mut self, ret: &ReturnStmt) {
        if let Some(value) = &ret.value {
            self.generate_expr(value);
            self.emit(Instruction::ReturnValue);
        } else {
            self.emit(Instruction::Return);
        }
    }

    fn generate_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal(lit) => self.generate_literal(lit),
            Expr::VarAccess(scope, name) => {
                if let Some(&local_idx) = self.local_vars.get(name) {
                    self.emit(Instruction::LoadLocal(local_idx));
                } else {
                    // Try global variable
                    self.emit(Instruction::Push(Value::Int32(0)));
                }
            }
            Expr::Binary(left, op, right) => {
                self.generate_expr(left);
                self.generate_expr(right);
                self.generate_binary_op(op);
            }
            Expr::Unary(op, operand) => {
                self.generate_expr(operand);
                self.generate_unary_op(op);
            }
            Expr::FuncCall(call) => {
                for arg in &call.args {
                    self.generate_expr(&arg.value);
                }

                let func_id = self.find_function(&call.name);
                self.emit(Instruction::Call(func_id, call.args.len() as u8));
            }
            _ => {
                self.emit(Instruction::Push(Value::Void));
            }
        }
    }

    fn generate_literal(&mut self, lit: &Literal) {
        let value = match lit {
            Literal::Bool(b) => Value::Bool(*b),
            Literal::Number(n) => {
                if n.contains('.') {
                    Value::Double(n.parse().unwrap_or(0.0))
                } else {
                    Value::Int32(n.parse().unwrap_or(0))
                }
            }
            Literal::String(s) => {
                let idx = self.module.strings.len();
                self.module.strings.push(s.clone());
                Value::String(s.clone())
            }
            Literal::Null => Value::Null,
            _ => Value::Void,
        };

        self.emit(Instruction::Push(value));
    }

    fn generate_binary_op(&mut self, op: &BinaryOp) {
        let instr = match op {
            BinaryOp::Add => Instruction::Add,
            BinaryOp::Sub => Instruction::Sub,
            BinaryOp::Mul => Instruction::Mul,
            BinaryOp::Div => Instruction::Div,
            BinaryOp::Mod => Instruction::Mod,
            BinaryOp::Pow => Instruction::Pow,
            BinaryOp::Eq => Instruction::Eq,
            BinaryOp::Ne => Instruction::Ne,
            BinaryOp::Lt => Instruction::Lt,
            BinaryOp::Le => Instruction::Le,
            BinaryOp::Gt => Instruction::Gt,
            BinaryOp::Ge => Instruction::Ge,
            BinaryOp::And => Instruction::And,
            BinaryOp::Or => Instruction::Or,
            BinaryOp::BitAnd => Instruction::BitAnd,
            BinaryOp::BitOr => Instruction::BitOr,
            BinaryOp::BitXor => Instruction::BitXor,
            BinaryOp::Shl => Instruction::Shl,
            BinaryOp::Shr => Instruction::Shr,
            BinaryOp::UShr => Instruction::UShr,
            _ => Instruction::Nop,
        };

        self.emit(instr);
    }

    fn generate_unary_op(&mut self, op: &UnaryOp) {
        let instr = match op {
            UnaryOp::Neg => Instruction::Neg,
            UnaryOp::Not => Instruction::Not,
            UnaryOp::BitNot => Instruction::BitNot,
            UnaryOp::PreInc => Instruction::PreInc,
            UnaryOp::PreDec => Instruction::PreDec,
            _ => Instruction::Nop,
        };

        self.emit(instr);
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
        let target = self.current_address;
        if let Some(instr) = self.module.instructions.get_mut(addr as usize) {
            match instr {
                Instruction::Jump(_) => *instr = Instruction::Jump(target),
                Instruction::JumpIfFalse(_) => *instr = Instruction::JumpIfFalse(target),
                Instruction::JumpIfTrue(_) => *instr = Instruction::JumpIfTrue(target),
                _ => {}
            }
        }
    }

    fn find_function(&self, name: &str) -> u32 {
        self.module.functions
            .iter()
            .position(|f| f.name == name)
            .unwrap_or(0) as u32
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}
