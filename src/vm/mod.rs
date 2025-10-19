use crate::compiler::bytecode::{BytecodeModule, Instruction, Value};

pub struct VM {
    pub(crate) stack: Vec<Value>,
    pub(crate) call_stack: Vec<CallFrame>,
    pub(crate) return_value: Value,
    pub(crate) aborted: bool,
}

struct CallFrame {
    return_address: u32,
    base_pointer: usize,
}

impl VM {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            call_stack: Vec::new(),
            return_value: Value::Void,
            aborted: false,
        }
    }

    /// Execute bytecode with arguments
    pub fn execute_with_args(
        &mut self,
        bytecode: &BytecodeModule,
        start_address: u32,
        args: &[Value],
    ) -> Result<(), String> {
        // Push arguments onto stack
        for arg in args {
            self.stack.push(arg.clone());
        }

        // Execute
        self.execute_bytecode(bytecode, start_address)
    }

    /// Get the return value from the last execution
    pub fn get_return_value(&self) -> &Value {
        &self.return_value
    }

    /// Reset the VM state
    pub fn reset(&mut self) {
        self.stack.clear();
        self.call_stack.clear();
        self.return_value = Value::Void;
        self.aborted = false;
    }

    /// Abort execution
    pub fn abort(&mut self) {
        self.aborted = true;
    }

    pub(crate) fn execute_bytecode(
        &mut self,
        bytecode: &BytecodeModule,
        start_address: u32,
    ) -> Result<(), String> {
        let mut ip = start_address as usize;

        loop {
            // Check for abort
            if self.aborted {
                return Err("Execution aborted".to_string());
            }

            if ip >= bytecode.instructions.len() {
                break;
            }

            match &bytecode.instructions[ip] {
                Instruction::Push(val) => {
                    self.stack.push(val.clone());
                    ip += 1;
                }

                Instruction::Pop => {
                    self.stack.pop().ok_or("Stack underflow")?;
                    ip += 1;
                }

                Instruction::Dup => {
                    let val = self.stack.last().ok_or("Stack underflow")?.clone();
                    self.stack.push(val);
                    ip += 1;
                }

                Instruction::LoadLocal(idx) => {
                    // TODO: Implement proper local variable access
                    self.stack.push(Value::Int32(0));
                    ip += 1;
                }

                Instruction::StoreLocal(idx) => {
                    // TODO: Implement proper local variable storage
                    self.stack.pop().ok_or("Stack underflow")?;
                    ip += 1;
                }

                Instruction::Add => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.add_values(a, b)?;
                    self.stack.push(result);
                    ip += 1;
                }

                Instruction::Sub => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.sub_values(a, b)?;
                    self.stack.push(result);
                    ip += 1;
                }

                Instruction::Mul => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.mul_values(a, b)?;
                    self.stack.push(result);
                    ip += 1;
                }

                Instruction::Div => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.div_values(a, b)?;
                    self.stack.push(result);
                    ip += 1;
                }

                Instruction::Neg => {
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.neg_value(a)?;
                    self.stack.push(result);
                    ip += 1;
                }

                Instruction::Eq => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.eq_values(a, b)?;
                    self.stack.push(Value::Bool(result));
                    ip += 1;
                }

                Instruction::Ne => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    let result = !self.eq_values(a, b)?;
                    self.stack.push(Value::Bool(result));
                    ip += 1;
                }

                Instruction::Lt => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.lt_values(a, b)?;
                    self.stack.push(Value::Bool(result));
                    ip += 1;
                }

                Instruction::Le => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.le_values(a, b)?;
                    self.stack.push(Value::Bool(result));
                    ip += 1;
                }

                Instruction::Gt => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.lt_values(b, a)?;
                    self.stack.push(Value::Bool(result));
                    ip += 1;
                }

                Instruction::Ge => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.le_values(b, a)?;
                    self.stack.push(Value::Bool(result));
                    ip += 1;
                }

                Instruction::Jump(addr) => {
                    ip = *addr as usize;
                }

                Instruction::JumpIfFalse(addr) => {
                    let val = self.stack.pop().ok_or("Stack underflow")?;
                    if !self.is_truthy(&val) {
                        ip = *addr as usize;
                    } else {
                        ip += 1;
                    }
                }

                Instruction::JumpIfTrue(addr) => {
                    let val = self.stack.pop().ok_or("Stack underflow")?;
                    if self.is_truthy(&val) {
                        ip = *addr as usize;
                    } else {
                        ip += 1;
                    }
                }

                Instruction::Return => {
                    self.return_value = Value::Void;
                    break;
                }

                Instruction::ReturnValue => {
                    self.return_value = self.stack.pop().ok_or("Stack underflow")?;
                    break;
                }

                Instruction::Halt => {
                    break;
                }

                _ => {
                    // Unimplemented instruction
                    ip += 1;
                }
            }
        }

        Ok(())
    }

    fn add_values(&self, a: Value, b: Value) -> Result<Value, String> {
        match (a, b) {
            (Value::Int32(a), Value::Int32(b)) => Ok(Value::Int32(a + b)),
            (Value::Int64(a), Value::Int64(b)) => Ok(Value::Int64(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a + b)),
            _ => Err("Type mismatch in addition".to_string()),
        }
    }

    fn sub_values(&self, a: Value, b: Value) -> Result<Value, String> {
        match (a, b) {
            (Value::Int32(a), Value::Int32(b)) => Ok(Value::Int32(a - b)),
            (Value::Int64(a), Value::Int64(b)) => Ok(Value::Int64(a - b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a - b)),
            _ => Err("Type mismatch in subtraction".to_string()),
        }
    }

    fn mul_values(&self, a: Value, b: Value) -> Result<Value, String> {
        match (a, b) {
            (Value::Int32(a), Value::Int32(b)) => Ok(Value::Int32(a * b)),
            (Value::Int64(a), Value::Int64(b)) => Ok(Value::Int64(a * b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a * b)),
            _ => Err("Type mismatch in multiplication".to_string()),
        }
    }

    fn div_values(&self, a: Value, b: Value) -> Result<Value, String> {
        match (a, b) {
            (Value::Int32(a), Value::Int32(b)) => {
                if b == 0 {
                    return Err("Division by zero".to_string());
                }
                Ok(Value::Int32(a / b))
            }
            (Value::Int64(a), Value::Int64(b)) => {
                if b == 0 {
                    return Err("Division by zero".to_string());
                }
                Ok(Value::Int64(a / b))
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
            (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a / b)),
            _ => Err("Type mismatch in division".to_string()),
        }
    }

    fn neg_value(&self, a: Value) -> Result<Value, String> {
        match a {
            Value::Int32(a) => Ok(Value::Int32(-a)),
            Value::Int64(a) => Ok(Value::Int64(-a)),
            Value::Float(a) => Ok(Value::Float(-a)),
            Value::Double(a) => Ok(Value::Double(-a)),
            _ => Err("Type mismatch in negation".to_string()),
        }
    }

    fn eq_values(&self, a: Value, b: Value) -> Result<bool, String> {
        Ok(a == b)
    }

    fn lt_values(&self, a: Value, b: Value) -> Result<bool, String> {
        match (a, b) {
            (Value::Int32(a), Value::Int32(b)) => Ok(a < b),
            (Value::Int64(a), Value::Int64(b)) => Ok(a < b),
            (Value::Float(a), Value::Float(b)) => Ok(a < b),
            (Value::Double(a), Value::Double(b)) => Ok(a < b),
            _ => Err("Type mismatch in comparison".to_string()),
        }
    }

    fn le_values(&self, a: Value, b: Value) -> Result<bool, String> {
        match (a, b) {
            (Value::Int32(a), Value::Int32(b)) => Ok(a <= b),
            (Value::Int64(a), Value::Int64(b)) => Ok(a <= b),
            (Value::Float(a), Value::Float(b)) => Ok(a <= b),
            (Value::Double(a), Value::Double(b)) => Ok(a <= b),
            _ => Err("Type mismatch in comparison".to_string()),
        }
    }

    fn is_truthy(&self, val: &Value) -> bool {
        match val {
            Value::Bool(b) => *b,
            Value::Int32(i) => *i != 0,
            Value::Int64(i) => *i != 0,
            Value::Null => false,
            Value::Void => false,
            _ => true,
        }
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}
