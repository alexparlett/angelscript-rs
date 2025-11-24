# Execution Model

> **Status**: DESIGN PHASE - Awaiting approval before implementation

This document covers how compiled scripts are executed, including the interpreter, call stack, and environments.

## Overview

Following "Crafting Interpreters", we implement in two phases:

1. **Phase 1: Tree-Walk Interpreter** - Direct AST interpretation
2. **Phase 2: Bytecode VM** - Compile to bytecode, stack machine (optional/later)

This document focuses on Phase 1.

## Interpreter Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Interpreter                              │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌────────────────────────┐  │
│  │  Evaluator  │  │  CallStack  │  │   EnvironmentStack     │  │
│  │  (AST walk) │  │  (frames)   │  │   (scope chain)        │  │
│  └─────────────┘  └─────────────┘  └────────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌────────────────────────┐  │
│  │ TypeRegistry│  │FunctionTable│  │   GarbageCollector     │  │
│  │  (readonly) │  │ (readonly)  │  │   (when needed)        │  │
│  └─────────────┘  └─────────────┘  └────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Interpreter

```rust
/// Tree-walk interpreter for AngelScript.
pub struct Interpreter<'engine> {
    /// Reference to the engine (for type info, registered functions)
    engine: &'engine Engine,
    
    /// Call stack
    call_stack: CallStack,
    
    /// Current environment (variable bindings)
    environment: Environment,
    
    /// Execution state
    state: ExecutionState,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ExecutionState {
    /// Normal execution
    Running,
    /// Returning from function
    Returning,
    /// Breaking from loop
    Breaking,
    /// Continuing loop
    Continuing,
    /// Suspended (coroutine)
    Suspended,
    /// Exception thrown
    Exception,
}

impl<'engine> Interpreter<'engine> {
    pub fn new(engine: &'engine Engine) -> Self;
    
    /// Execute a compiled module's entry point
    pub fn execute(&mut self, module: &Module) -> Result<ScriptValue, RuntimeError>;
    
    /// Call a function by name
    pub fn call_function(
        &mut self,
        module: &Module,
        name: &str,
        args: Vec<ScriptValue>,
    ) -> Result<ScriptValue, RuntimeError>;
    
    /// Evaluate an expression
    fn evaluate(&mut self, expr: &Expr) -> Result<ScriptValue, RuntimeError>;
    
    /// Execute a statement
    fn execute_stmt(&mut self, stmt: &Stmt) -> Result<(), RuntimeError>;
}
```

## Expression Evaluation

```rust
impl<'engine> Interpreter<'engine> {
    fn evaluate(&mut self, expr: &Expr) -> Result<ScriptValue, RuntimeError> {
        match expr {
            // === Literals ===
            Expr::Literal(lit) => self.eval_literal(lit),
            
            // === Variables ===
            Expr::Variable(name) => self.eval_variable(name),
            Expr::Assignment { target, value } => self.eval_assignment(target, value),
            
            // === Operators ===
            Expr::Unary { op, operand } => self.eval_unary(op, operand),
            Expr::Binary { left, op, right } => self.eval_binary(left, op, right),
            Expr::Ternary { condition, then_expr, else_expr } => {
                self.eval_ternary(condition, then_expr, else_expr)
            }
            
            // === Member Access ===
            Expr::PropertyAccess { object, property } => {
                self.eval_property_access(object, property)
            }
            Expr::MethodCall { object, method, args } => {
                self.eval_method_call(object, method, args)
            }
            Expr::Index { object, index } => self.eval_index(object, index),
            
            // === Calls ===
            Expr::Call { callee, args } => self.eval_call(callee, args),
            Expr::Construct { type_name, args } => self.eval_construct(type_name, args),
            
            // === Handle Operations ===
            Expr::HandleOf(inner) => self.eval_handle_of(inner),
            Expr::Cast { target_type, expr } => self.eval_cast(target_type, expr),
            Expr::Is { expr, type_name } => self.eval_is(expr, type_name),
            Expr::NotIs { expr, type_name } => self.eval_not_is(expr, type_name),
            
            // === Lambda ===
            Expr::Lambda { params, body } => self.eval_lambda(params, body),
            
            // === Special ===
            Expr::This => self.eval_this(),
            Expr::Super => self.eval_super(),
            Expr::InitList(items) => self.eval_init_list(items),
        }
    }
}
```

### Literal Evaluation

```rust
fn eval_literal(&self, lit: &Literal) -> Result<ScriptValue, RuntimeError> {
    Ok(match lit {
        Literal::Null => ScriptValue::Null,
        Literal::Bool(b) => ScriptValue::Bool(*b),
        Literal::Int(i) => ScriptValue::Int32(*i as i32),
        Literal::Int64(i) => ScriptValue::Int64(*i),
        Literal::UInt(u) => ScriptValue::UInt32(*u as u32),
        Literal::UInt64(u) => ScriptValue::UInt64(*u),
        Literal::Float(f) => ScriptValue::Float(*f),
        Literal::Double(d) => ScriptValue::Double(*d),
        Literal::String(s) => {
            // Use string factory if registered
            self.create_string(s)?
        }
    })
}
```

### Binary Operations

```rust
fn eval_binary(
    &mut self,
    left: &Expr,
    op: &BinaryOp,
    right: &Expr,
) -> Result<ScriptValue, RuntimeError> {
    // Short-circuit evaluation for && and ||
    if *op == BinaryOp::And {
        let left_val = self.evaluate(left)?;
        if !left_val.as_bool().unwrap_or(false) {
            return Ok(ScriptValue::Bool(false));
        }
        return self.evaluate(right);
    }
    if *op == BinaryOp::Or {
        let left_val = self.evaluate(left)?;
        if left_val.as_bool().unwrap_or(false) {
            return Ok(ScriptValue::Bool(true));
        }
        return self.evaluate(right);
    }
    
    let left_val = self.evaluate(left)?;
    let right_val = self.evaluate(right)?;
    
    // Try primitive operations first
    if let Some(result) = self.try_primitive_binary(op, &left_val, &right_val)? {
        return Ok(result);
    }
    
    // Try operator overload
    self.call_binary_operator(op, left_val, right_val)
}

fn try_primitive_binary(
    &self,
    op: &BinaryOp,
    left: &ScriptValue,
    right: &ScriptValue,
) -> Result<Option<ScriptValue>, RuntimeError> {
    // Match on combinations of primitive types
    match (left, right) {
        (ScriptValue::Int32(a), ScriptValue::Int32(b)) => {
            Ok(Some(match op {
                BinaryOp::Add => ScriptValue::Int32(a.wrapping_add(*b)),
                BinaryOp::Sub => ScriptValue::Int32(a.wrapping_sub(*b)),
                BinaryOp::Mul => ScriptValue::Int32(a.wrapping_mul(*b)),
                BinaryOp::Div => {
                    if *b == 0 {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    ScriptValue::Int32(a / b)
                }
                BinaryOp::Equal => ScriptValue::Bool(a == b),
                BinaryOp::NotEqual => ScriptValue::Bool(a != b),
                BinaryOp::Less => ScriptValue::Bool(a < b),
                BinaryOp::LessEqual => ScriptValue::Bool(a <= b),
                BinaryOp::Greater => ScriptValue::Bool(a > b),
                BinaryOp::GreaterEqual => ScriptValue::Bool(a >= b),
                // ... etc
                _ => return Ok(None),
            }))
        }
        // ... other type combinations
        _ => Ok(None),
    }
}
```

### Operator Overload Resolution

```rust
fn call_binary_operator(
    &mut self,
    op: &BinaryOp,
    left: ScriptValue,
    right: ScriptValue,
) -> Result<ScriptValue, RuntimeError> {
    let left_type = self.engine.types().get(left.type_id())?;
    let right_type = self.engine.types().get(right.type_id())?;
    
    let method_name = match op {
        BinaryOp::Add => "opAdd",
        BinaryOp::Sub => "opSub",
        BinaryOp::Mul => "opMul",
        // ... etc
    };
    let reverse_name = format!("{}_r", method_name);
    
    // Try left.opAdd(right)
    if let Some(method) = left_type.find_method(method_name, &[right.type_id()]) {
        return self.call_method(&left, method, vec![right]);
    }
    
    // Try right.opAdd_r(left)
    if let Some(method) = right_type.find_method(&reverse_name, &[left.type_id()]) {
        return self.call_method(&right, method, vec![left]);
    }
    
    Err(RuntimeError::NoOperator {
        op: format!("{:?}", op),
        left_type: left_type.name.clone(),
        right_type: right_type.name.clone(),
    })
}
```

## Statement Execution

```rust
impl<'engine> Interpreter<'engine> {
    fn execute_stmt(&mut self, stmt: &Stmt) -> Result<(), RuntimeError> {
        match stmt {
            Stmt::Expression(expr) => {
                self.evaluate(expr)?;
            }
            
            Stmt::VarDecl { name, type_hint, initializer } => {
                self.exec_var_decl(name, type_hint, initializer)?;
            }
            
            Stmt::Block(stmts) => {
                self.exec_block(stmts)?;
            }
            
            Stmt::If { condition, then_branch, else_branch } => {
                self.exec_if(condition, then_branch, else_branch)?;
            }
            
            Stmt::While { condition, body } => {
                self.exec_while(condition, body)?;
            }
            
            Stmt::DoWhile { body, condition } => {
                self.exec_do_while(body, condition)?;
            }
            
            Stmt::For { init, condition, update, body } => {
                self.exec_for(init, condition, update, body)?;
            }
            
            Stmt::ForEach { var_type, var_name, iterable, body } => {
                self.exec_foreach(var_type, var_name, iterable, body)?;
            }
            
            Stmt::Switch { expr, cases } => {
                self.exec_switch(expr, cases)?;
            }
            
            Stmt::Return(expr) => {
                self.exec_return(expr)?;
            }
            
            Stmt::Break => {
                self.state = ExecutionState::Breaking;
            }
            
            Stmt::Continue => {
                self.state = ExecutionState::Continuing;
            }
            
            Stmt::Try { try_block, catch_block } => {
                self.exec_try_catch(try_block, catch_block)?;
            }
        }
        Ok(())
    }
}
```

### Control Flow

```rust
fn exec_if(
    &mut self,
    condition: &Expr,
    then_branch: &Stmt,
    else_branch: &Option<Box<Stmt>>,
) -> Result<(), RuntimeError> {
    let cond_val = self.evaluate(condition)?;
    let cond_bool = self.to_bool(cond_val)?;
    
    if cond_bool {
        self.execute_stmt(then_branch)?;
    } else if let Some(else_stmt) = else_branch {
        self.execute_stmt(else_stmt)?;
    }
    Ok(())
}

fn exec_while(&mut self, condition: &Expr, body: &Stmt) -> Result<(), RuntimeError> {
    loop {
        let cond_val = self.evaluate(condition)?;
        if !self.to_bool(cond_val)? {
            break;
        }
        
        self.execute_stmt(body)?;
        
        match self.state {
            ExecutionState::Breaking => {
                self.state = ExecutionState::Running;
                break;
            }
            ExecutionState::Continuing => {
                self.state = ExecutionState::Running;
                continue;
            }
            ExecutionState::Returning => break,
            _ => {}
        }
    }
    Ok(())
}
```

## Environment (Variable Scopes)

```rust
/// Environment manages variable bindings across scopes.
pub struct Environment {
    /// Stack of scopes (innermost last)
    scopes: Vec<Scope>,
}

/// A single scope containing variable bindings.
pub struct Scope {
    /// Variable bindings
    variables: HashMap<String, ScriptValue>,
    
    /// Scope kind (for different cleanup behavior)
    kind: ScopeKind,
}

pub enum ScopeKind {
    /// Global scope
    Global,
    /// Function scope  
    Function,
    /// Block scope (if, while, etc.)
    Block,
    /// Class instance scope (this)
    Instance,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new(ScopeKind::Global)],
        }
    }
    
    /// Push a new scope
    pub fn push_scope(&mut self, kind: ScopeKind) {
        self.scopes.push(Scope::new(kind));
    }
    
    /// Pop the innermost scope
    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }
    
    /// Define a new variable in current scope
    pub fn define(&mut self, name: String, value: ScriptValue) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.variables.insert(name, value);
        }
    }
    
    /// Get a variable (searches outward through scopes)
    pub fn get(&self, name: &str) -> Option<&ScriptValue> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.variables.get(name) {
                return Some(value);
            }
        }
        None
    }
    
    /// Set a variable (must already exist)
    pub fn set(&mut self, name: &str, value: ScriptValue) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if scope.variables.contains_key(name) {
                scope.variables.insert(name.to_string(), value);
                return true;
            }
        }
        false
    }
}
```

## Call Stack

```rust
/// Call stack for function calls.
pub struct CallStack {
    frames: Vec<CallFrame>,
    max_depth: usize,
}

/// A single call frame.
pub struct CallFrame {
    /// Function being executed
    function: FunctionId,
    
    /// Module containing the function
    module: ModuleId,
    
    /// Instruction pointer / current AST node
    location: SourceLocation,
    
    /// Base pointer into environment stack
    env_base: usize,
    
    /// Return value slot
    return_value: Option<ScriptValue>,
    
    /// The 'this' object for method calls
    this_object: Option<Handle>,
}

impl CallStack {
    pub fn new(max_depth: usize) -> Self {
        Self {
            frames: Vec::with_capacity(64),
            max_depth,
        }
    }
    
    /// Push a new call frame
    pub fn push(&mut self, frame: CallFrame) -> Result<(), RuntimeError> {
        if self.frames.len() >= self.max_depth {
            return Err(RuntimeError::StackOverflow);
        }
        self.frames.push(frame);
        Ok(())
    }
    
    /// Pop the top frame
    pub fn pop(&mut self) -> Option<CallFrame> {
        self.frames.pop()
    }
    
    /// Get current frame
    pub fn current(&self) -> Option<&CallFrame> {
        self.frames.last()
    }
    
    /// Get current frame mutably
    pub fn current_mut(&mut self) -> Option<&mut CallFrame> {
        self.frames.last_mut()
    }
    
    /// Get call stack depth
    pub fn depth(&self) -> usize {
        self.frames.len()
    }
    
    /// Generate stack trace
    pub fn stack_trace(&self) -> Vec<StackTraceEntry> {
        self.frames
            .iter()
            .rev()
            .map(|f| StackTraceEntry {
                function: f.function,
                location: f.location.clone(),
            })
            .collect()
    }
}
```

## Function Calls

```rust
impl<'engine> Interpreter<'engine> {
    /// Call a script function
    fn call_script_function(
        &mut self,
        func: &ScriptFunction,
        args: Vec<ScriptValue>,
        this: Option<Handle>,
    ) -> Result<ScriptValue, RuntimeError> {
        // Push call frame
        self.call_stack.push(CallFrame {
            function: func.id,
            module: func.module,
            location: func.location.clone(),
            env_base: self.environment.scopes.len(),
            return_value: None,
            this_object: this,
        })?;
        
        // Create new scope for function
        self.environment.push_scope(ScopeKind::Function);
        
        // Bind parameters
        for (param, arg) in func.params.iter().zip(args) {
            self.environment.define(param.name.clone(), arg);
        }
        
        // Execute function body
        let result = self.execute_stmt(&func.body);
        
        // Get return value
        let return_value = self.call_stack.current()
            .and_then(|f| f.return_value.take())
            .unwrap_or(ScriptValue::Void);
        
        // Clean up
        self.environment.pop_scope();
        self.call_stack.pop();
        self.state = ExecutionState::Running;
        
        result?;
        Ok(return_value)
    }
    
    /// Call a native function
    fn call_native_function(
        &mut self,
        func: &NativeFunction,
        args: Vec<ScriptValue>,
        this: Option<&mut ScriptValue>,
    ) -> Result<ScriptValue, RuntimeError> {
        // Native functions are stored as boxed closures
        (func.callable)(this, &args)
    }
}
```

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Null handle dereference")]
    NullHandle,
    
    #[error("Division by zero")]
    DivisionByZero,
    
    #[error("Stack overflow")]
    StackOverflow,
    
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
    
    #[error("Undefined variable: {name}")]
    UndefinedVariable { name: String },
    
    #[error("No operator {op} for types {left_type} and {right_type}")]
    NoOperator { op: String, left_type: String, right_type: String },
    
    #[error("Index out of bounds: {index} (length {length})")]
    IndexOutOfBounds { index: i64, length: usize },
    
    #[error("Script exception: {message}")]
    ScriptException { message: String },
    
    #[error("Invalid cast from {from} to {to}")]
    InvalidCast { from: String, to: String },
}

impl RuntimeError {
    /// Add location information
    pub fn with_location(self, location: SourceLocation) -> LocatedError {
        LocatedError {
            error: self,
            location,
        }
    }
}
```

## Try/Catch

```rust
fn exec_try_catch(
    &mut self,
    try_block: &Stmt,
    catch_block: &Stmt,
) -> Result<(), RuntimeError> {
    let result = self.execute_stmt(try_block);
    
    match result {
        Ok(()) => Ok(()),
        Err(RuntimeError::ScriptException { message }) => {
            // Enter catch block
            self.environment.push_scope(ScopeKind::Block);
            // Optionally bind exception message
            self.execute_stmt(catch_block)?;
            self.environment.pop_scope();
            Ok(())
        }
        Err(e) => Err(e), // Propagate non-script errors
    }
}
```

## Suspend/Resume (Future)

For coroutine support:

```rust
/// Snapshot of interpreter state for suspension.
pub struct SuspendedState {
    call_stack: CallStack,
    environment: Environment,
    // ... other state
}

impl Interpreter<'_> {
    /// Suspend execution and return current state
    pub fn suspend(&mut self) -> SuspendedState;
    
    /// Resume from suspended state
    pub fn resume(&mut self, state: SuspendedState) -> Result<ScriptValue, RuntimeError>;
}
```

## Open Questions

1. **Maximum stack depth** - What default? 
   - Proposal: 256 frames default, configurable

2. **Tail call optimization** - Support it?
   - Proposal: Not in Phase 1, consider for bytecode VM

3. **Coroutine support** - When to add?
   - Proposal: After basic interpreter works

4. **Debug hooks** - Breakpoints, stepping?
   - Proposal: Design hooks interface, implement later
