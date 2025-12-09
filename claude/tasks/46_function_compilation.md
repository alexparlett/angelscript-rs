# Task 46: Function Compilation Pass

## Overview

Implement the second pass of the compiler that generates bytecode for function bodies. This pass runs after registration (Pass 1) and compiles all script functions.

## Goals

1. Orchestrate compilation of all script functions
2. Handle function-level setup (params, locals allocation)
3. Verify all code paths return (for non-void functions)
4. Finalize bytecode and store in registry

## Dependencies

- Task 37: Registration Pass (provides function signatures)
- Task 43-43: Statement Compilation
- Task 40-41: Expression Compilation
- Task 39: Bytecode Emitter

## Files to Create/Modify

```
crates/angelscript-compiler/src/
├── pass/
│   ├── mod.rs                    # Pass modules
│   ├── registration.rs           # Pass 1 (from Task 37)
│   └── compilation.rs            # Pass 2 - function compilation
├── function_compiler.rs          # Single function compilation
└── return_checker.rs             # Return path verification
```

## Detailed Implementation

### Compilation Pass (pass/compilation.rs)

```rust
use angelscript_core::{TypeHash, UnitId};
use angelscript_parser::ast::{Module, Decl, FunctionDecl, ClassDecl};

use crate::context::CompilationContext;
use crate::error::{CompileError, Result};
use crate::function_compiler::FunctionCompiler;

/// Pass 2: Compile function bodies to bytecode.
pub struct CompilationPass<'ctx> {
    ctx: &'ctx mut CompilationContext<'ctx>,
    unit_id: UnitId,
}

impl<'ctx> CompilationPass<'ctx> {
    pub fn new(ctx: &'ctx mut CompilationContext<'ctx>, unit_id: UnitId) -> Self {
        Self { ctx, unit_id }
    }

    /// Run compilation pass on a module.
    pub fn run(&mut self, module: &Module) -> Result<CompilationOutput> {
        let mut output = CompilationOutput::new(self.unit_id);

        // Compile all declarations
        for decl in &module.declarations {
            self.compile_decl(decl, &mut output)?;
        }

        Ok(output)
    }

    /// Compile a declaration.
    fn compile_decl(&mut self, decl: &Decl, output: &mut CompilationOutput) -> Result<()> {
        match decl {
            Decl::Function(func) => {
                self.compile_function(func, None, output)?;
            }

            Decl::Class(class) => {
                self.compile_class(class, output)?;
            }

            Decl::Interface(_) => {
                // Interfaces have no implementation
            }

            Decl::Mixin(mixin) => {
                // Mixins are expanded during registration, not compiled here
            }

            Decl::Enum(_) => {
                // Enums have no runtime code (values are constants)
            }

            Decl::Namespace { name, declarations } => {
                self.ctx.push_namespace(name.clone());
                for inner in declarations {
                    self.compile_decl(inner, output)?;
                }
                self.ctx.pop_namespace();
            }

            Decl::Import(_) | Decl::Typedef(_) | Decl::Funcdef(_) => {
                // No code generation needed
            }

            Decl::GlobalVar(var) => {
                // Global variable initializers are compiled separately
                self.compile_global_init(var, output)?;
            }
        }

        Ok(())
    }

    /// Compile a function declaration.
    fn compile_function(
        &mut self,
        func: &FunctionDecl,
        owner: Option<TypeHash>,  // Some for methods
        output: &mut CompilationOutput,
    ) -> Result<()> {
        // Skip if no body (external, abstract, or native)
        let body = match &func.body {
            Some(b) => b,
            None => return Ok(()),
        };

        // Get function hash from registration
        let func_hash = self.ctx.lookup_function_hash(&func.name, owner)?;

        // Get function entry for signature info
        let func_entry = self.ctx.get_function(func_hash)
            .ok_or_else(|| CompileError::Internal {
                message: format!("Function not registered: {}", func.name),
            })?;

        // Create function compiler
        let mut compiler = FunctionCompiler::new(
            self.ctx,
            func_entry.def.clone(),
            owner,
        );

        // Compile body
        compiler.compile_body(body)?;

        // Verify return paths
        compiler.verify_returns(func.span)?;

        // Get compiled bytecode
        let bytecode = compiler.finish();

        // Store bytecode
        output.add_bytecode(func_hash, bytecode);

        Ok(())
    }

    /// Compile a class's methods.
    fn compile_class(&mut self, class: &ClassDecl, output: &mut CompilationOutput) -> Result<()> {
        // Get class hash
        let class_hash = self.ctx.lookup_type_hash(&class.name)?;

        // Compile each method
        for member in &class.members {
            match member {
                ClassMember::Method(func) => {
                    self.compile_function(func, Some(class_hash), output)?;
                }

                ClassMember::Constructor(ctor) => {
                    self.compile_constructor(ctor, class_hash, output)?;
                }

                ClassMember::Destructor(dtor) => {
                    self.compile_destructor(dtor, class_hash, output)?;
                }

                ClassMember::Property(_) | ClassMember::Field(_) => {
                    // No code (getters/setters are methods)
                }
            }
        }

        Ok(())
    }

    /// Compile a constructor.
    fn compile_constructor(
        &mut self,
        ctor: &ConstructorDecl,
        class_hash: TypeHash,
        output: &mut CompilationOutput,
    ) -> Result<()> {
        let body = match &ctor.body {
            Some(b) => b,
            None => return Ok(()),
        };

        // Get constructor hash
        let param_hashes: Vec<_> = ctor.params.iter()
            .map(|p| self.ctx.resolve_type(&p.type_expr, ctor.span).map(|t| t.type_hash))
            .collect::<Result<_>>()?;

        let ctor_hash = TypeHash::from_constructor(class_hash, &param_hashes);

        // Get entry
        let ctor_entry = self.ctx.get_function(ctor_hash)?;

        // Create compiler
        let mut compiler = FunctionCompiler::new(
            self.ctx,
            ctor_entry.def.clone(),
            Some(class_hash),
        );

        // Compile member initializer list
        if let Some(init_list) = &ctor.initializer_list {
            compiler.compile_initializer_list(init_list, class_hash)?;
        } else {
            // Auto-initialize members
            compiler.compile_default_member_init(class_hash)?;
        }

        // Compile body
        compiler.compile_body(body)?;

        // Constructors implicitly return void
        let bytecode = compiler.finish();
        output.add_bytecode(ctor_hash, bytecode);

        Ok(())
    }

    /// Compile a destructor.
    fn compile_destructor(
        &mut self,
        dtor: &DestructorDecl,
        class_hash: TypeHash,
        output: &mut CompilationOutput,
    ) -> Result<()> {
        let body = match &dtor.body {
            Some(b) => b,
            None => return Ok(()),
        };

        let dtor_hash = TypeHash::from_destructor(class_hash);
        let dtor_entry = self.ctx.get_function(dtor_hash)?;

        let mut compiler = FunctionCompiler::new(
            self.ctx,
            dtor_entry.def.clone(),
            Some(class_hash),
        );

        // Compile body
        compiler.compile_body(body)?;

        // Auto-destroy members after body
        compiler.compile_member_destruction(class_hash)?;

        let bytecode = compiler.finish();
        output.add_bytecode(dtor_hash, bytecode);

        Ok(())
    }

    /// Compile global variable initializer.
    fn compile_global_init(
        &mut self,
        var: &GlobalVarDecl,
        output: &mut CompilationOutput,
    ) -> Result<()> {
        if let Some(init) = &var.initializer {
            // Create init function for this global
            let init_hash = TypeHash::from_ident(&var.name);

            let mut compiler = FunctionCompiler::new_for_global_init(
                self.ctx,
                var.name.clone(),
            );

            let var_type = self.ctx.resolve_type(&var.type_expr, var.span)?;
            compiler.compile_global_initializer(init, &var_type)?;

            let bytecode = compiler.finish();
            output.add_bytecode(init_hash, bytecode);
            output.add_global_init(init_hash);
        }

        Ok(())
    }
}

/// Output from compilation pass.
/// Note: Bytecode is stored in the registry (FunctionImpl::Script.bytecode),
/// not in this struct. This tracks what was compiled.
pub struct CompilationOutput {
    pub unit_id: UnitId,
    /// Shared constant pool for all functions
    pub constants: ConstantPool,
    /// Functions that were compiled (bytecode stored in registry)
    pub compiled_functions: Vec<TypeHash>,
    /// Global initializers in order
    pub global_inits: Vec<TypeHash>,
}

impl CompilationOutput {
    pub fn new(unit_id: UnitId) -> Self {
        Self {
            unit_id,
            constants: ConstantPool::new(),
            compiled_functions: Vec::new(),
            global_inits: Vec::new(),
        }
    }

    /// Store bytecode in registry and track what was compiled.
    pub fn store_bytecode(
        &mut self,
        ctx: &mut CompilationContext,
        hash: TypeHash,
        bytecode: BytecodeChunk,
    ) {
        // Store in registry
        ctx.set_function_bytecode(hash, bytecode);
        // Track what was compiled
        self.compiled_functions.push(hash);
    }

    pub fn add_global_init(&mut self, hash: TypeHash) {
        self.global_inits.push(hash);
    }

    /// Get mutable constant pool for emitter.
    pub fn constants_mut(&mut self) -> &mut ConstantPool {
        &mut self.constants
    }
}
```

### Function Compiler (function_compiler.rs)

```rust
use angelscript_core::{DataType, Span, TypeHash};
use angelscript_parser::ast::{Stmt, Expr};

use crate::bytecode::{BytecodeChunk, BytecodeEmitter, OpCode};
use crate::context::CompilationContext;
use crate::error::{CompileError, Result};
use crate::scope::LocalScope;
use crate::stmt::StmtCompiler;
use crate::return_checker::ReturnChecker;

/// Compiles a single function body.
pub struct FunctionCompiler<'ctx> {
    ctx: &'ctx CompilationContext<'ctx>,
    def: FunctionDef,
    owner: Option<TypeHash>,
    stmt_compiler: StmtCompiler<'ctx>,
    has_explicit_return: bool,
}

impl<'ctx> FunctionCompiler<'ctx> {
    pub fn new(
        ctx: &'ctx CompilationContext<'ctx>,
        def: FunctionDef,
        owner: Option<TypeHash>,
    ) -> Self {
        // Build parameter list for scope
        let params: Vec<_> = def.params.iter()
            .map(|p| (p.name.clone(), p.data_type, p.is_const))
            .collect();

        // Add implicit 'this' for methods
        let mut all_params = Vec::new();
        if owner.is_some() {
            all_params.push((
                "this".to_string(),
                DataType::handle(owner.unwrap()),
                def.traits.is_const,  // const method = const this
            ));
        }
        all_params.extend(params);

        let stmt_compiler = StmtCompiler::new(ctx, def.return_type, all_params);

        Self {
            ctx,
            def,
            owner,
            stmt_compiler,
            has_explicit_return: false,
        }
    }

    /// Create compiler for global initializer.
    pub fn new_for_global_init(ctx: &'ctx CompilationContext<'ctx>, name: String) -> Self {
        let def = FunctionDef::new_global_init(name);
        let stmt_compiler = StmtCompiler::new(ctx, DataType::void(), vec![]);

        Self {
            ctx,
            def,
            owner: None,
            stmt_compiler,
            has_explicit_return: false,
        }
    }

    /// Compile function body.
    pub fn compile_body(&mut self, body: &[Stmt]) -> Result<()> {
        for stmt in body {
            self.stmt_compiler.compile(stmt, Span::default())?;
        }

        // Track if we saw explicit return
        self.has_explicit_return = self.stmt_compiler.has_return();

        Ok(())
    }

    /// Compile constructor initializer list.
    pub fn compile_initializer_list(
        &mut self,
        init_list: &[MemberInit],
        class_hash: TypeHash,
    ) -> Result<()> {
        let class = self.ctx.get_type(class_hash)
            .and_then(|t| t.as_class())
            .ok_or_else(|| CompileError::Internal {
                message: "Class not found for initializer".to_string(),
            })?;

        // Initialize base class first (if specified)
        for init in init_list {
            if init.is_base_init() {
                self.compile_base_init(init, class)?;
            }
        }

        // Initialize members in declaration order
        for prop in &class.properties {
            let init = init_list.iter().find(|i| i.name == prop.name);

            if let Some(init) = init {
                // Explicit initialization
                self.compile_member_init_expr(&prop.name, &init.value, &prop.data_type)?;
            } else if let Some(default) = &prop.default_value {
                // Default value from declaration
                self.compile_member_init_expr(&prop.name, default, &prop.data_type)?;
            } else {
                // Default construct
                self.compile_member_default_init(&prop.name, &prop.data_type)?;
            }
        }

        Ok(())
    }

    /// Compile default member initialization.
    pub fn compile_default_member_init(&mut self, class_hash: TypeHash) -> Result<()> {
        let class = self.ctx.get_type(class_hash)
            .and_then(|t| t.as_class())?;

        // Initialize base first
        if let Some(base) = class.base_class {
            self.stmt_compiler.emitter().emit_load_this();
            self.stmt_compiler.emitter().emit_call_base_ctor(base);
        }

        // Default init each member
        for prop in &class.properties {
            if let Some(default) = &prop.default_value {
                self.compile_member_init_expr(&prop.name, default, &prop.data_type)?;
            } else {
                self.compile_member_default_init(&prop.name, &prop.data_type)?;
            }
        }

        Ok(())
    }

    fn compile_member_init_expr(
        &mut self,
        name: &str,
        value: &Expr,
        member_type: &DataType,
    ) -> Result<()> {
        // this.member = value
        self.stmt_compiler.emitter().emit_load_this();
        let mut ec = self.stmt_compiler.expr_compiler();
        ec.check(value, member_type, Span::default())?;
        self.stmt_compiler.emitter().emit_store_member(name);
        Ok(())
    }

    fn compile_member_default_init(&mut self, name: &str, member_type: &DataType) -> Result<()> {
        self.stmt_compiler.emitter().emit_load_this();

        if member_type.is_primitive() {
            self.stmt_compiler.emitter().emit_push_zero(member_type);
        } else if member_type.is_handle {
            self.stmt_compiler.emitter().emit(OpCode::PushNull);
        } else {
            // Default construct
            let class = self.ctx.get_type(member_type.type_hash)
                .and_then(|t| t.as_class())?;
            let default_ctor = class.behaviors.default_constructor
                .ok_or_else(|| CompileError::NoDefaultConstructor {
                    type_name: class.name.clone(),
                    span: Span::default(),
                })?;
            self.stmt_compiler.emitter().emit_call(default_ctor, 0);
        }

        self.stmt_compiler.emitter().emit_store_member(name);
        Ok(())
    }

    fn compile_base_init(&mut self, init: &MemberInit, class: &ClassEntry) -> Result<()> {
        let base = class.base_class
            .ok_or_else(|| CompileError::NoBaseClass {
                class_name: class.name.clone(),
                span: Span::default(),
            })?;

        self.stmt_compiler.emitter().emit_load_this();

        // Compile base constructor arguments
        for arg in &init.args {
            let mut ec = self.stmt_compiler.expr_compiler();
            ec.infer(arg, Span::default())?;
        }

        // Find matching base constructor
        let base_ctor = self.ctx.resolve_constructor(base, &init.args)?;
        self.stmt_compiler.emitter().emit_call(base_ctor, init.args.len() as u8 + 1);

        Ok(())
    }

    /// Compile member destruction (for destructor).
    pub fn compile_member_destruction(&mut self, class_hash: TypeHash) -> Result<()> {
        let class = self.ctx.get_type(class_hash)
            .and_then(|t| t.as_class())?;

        // Destroy members in reverse declaration order
        for prop in class.properties.iter().rev() {
            if prop.data_type.needs_destructor() {
                self.stmt_compiler.emitter().emit_load_this();
                self.stmt_compiler.emitter().emit_load_member(&prop.name);
                self.stmt_compiler.emitter().emit_destroy();
            }
        }

        // Call base destructor
        if let Some(base) = class.base_class {
            let base_dtor = TypeHash::from_destructor(base);
            self.stmt_compiler.emitter().emit_load_this();
            self.stmt_compiler.emitter().emit_call(base_dtor, 1);
        }

        Ok(())
    }

    /// Compile global variable initializer.
    pub fn compile_global_initializer(
        &mut self,
        init: &Expr,
        var_type: &DataType,
    ) -> Result<()> {
        let mut ec = self.stmt_compiler.expr_compiler();
        ec.check(init, var_type, Span::default())?;
        self.stmt_compiler.emitter().emit(OpCode::ReturnVoid);
        Ok(())
    }

    /// Verify all code paths return (for non-void functions).
    pub fn verify_returns(&self, span: Span) -> Result<()> {
        if self.def.return_type.is_void() {
            // Void functions don't need explicit return
            return Ok(());
        }

        let checker = ReturnChecker::new();
        if !checker.all_paths_return(self.stmt_compiler.bytecode()) {
            return Err(CompileError::NotAllPathsReturn {
                function: self.def.name.clone(),
                span,
            });
        }

        Ok(())
    }

    /// Get compiled bytecode.
    pub fn finish(mut self) -> BytecodeChunk {
        // Add implicit return for void functions
        if self.def.return_type.is_void() && !self.has_explicit_return {
            self.stmt_compiler.emitter().emit(OpCode::ReturnVoid);
        }

        self.stmt_compiler.finish()
    }
}
```

### Return Checker (return_checker.rs)

```rust
use crate::bytecode::{BytecodeChunk, OpCode};

/// Verifies all code paths return a value.
pub struct ReturnChecker;

impl ReturnChecker {
    pub fn new() -> Self {
        Self
    }

    /// Check if all code paths in the bytecode return.
    pub fn all_paths_return(&self, bytecode: &BytecodeChunk) -> bool {
        // Simple analysis: check if last instruction is Return
        // More sophisticated would do CFG analysis

        if bytecode.code.is_empty() {
            return false;
        }

        // Build basic blocks and check each exit
        let blocks = self.build_basic_blocks(bytecode);
        self.check_blocks_return(&blocks, bytecode)
    }

    fn build_basic_blocks(&self, bytecode: &BytecodeChunk) -> Vec<BasicBlock> {
        // Simplified: just check final instruction
        // Full implementation would build CFG
        vec![BasicBlock {
            start: 0,
            end: bytecode.code.len(),
            exits: vec![BlockExit::End],
        }]
    }

    fn check_blocks_return(&self, blocks: &[BasicBlock], bytecode: &BytecodeChunk) -> bool {
        for block in blocks {
            match &block.exits {
                exits if exits.is_empty() => {
                    // Fall through - check if ends with return
                    if !self.ends_with_return(bytecode, block.end) {
                        return false;
                    }
                }
                exits => {
                    for exit in exits {
                        match exit {
                            BlockExit::Return => continue,
                            BlockExit::Jump(target) => {
                                // Would need to check target block
                            }
                            BlockExit::End => {
                                if !self.ends_with_return(bytecode, block.end) {
                                    return false;
                                }
                            }
                        }
                    }
                }
            }
        }
        true
    }

    fn ends_with_return(&self, bytecode: &BytecodeChunk, offset: usize) -> bool {
        if offset == 0 {
            return false;
        }

        // Look at last opcode
        let last_op = bytecode.code.get(offset - 1);
        matches!(last_op, Some(OpCode::Return) | Some(OpCode::ReturnVoid))
    }
}

struct BasicBlock {
    start: usize,
    end: usize,
    exits: Vec<BlockExit>,
}

enum BlockExit {
    Return,
    Jump(usize),
    End,
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_simple_function() {
        // int add(int a, int b) { return a + b; }
    }

    #[test]
    fn compile_void_function() {
        // void greet() { print("hello"); }
    }

    #[test]
    fn compile_method() {
        // class Foo { int getValue() { return this.value; } }
    }

    #[test]
    fn compile_constructor() {
        // class Foo { int x; Foo(int x) : x(x) {} }
    }

    #[test]
    fn compile_destructor() {
        // class Foo { ~Foo() { cleanup(); } }
    }

    #[test]
    fn return_checker_all_paths() {
        // int abs(int x) { if (x < 0) return -x; else return x; }
    }

    #[test]
    fn return_checker_missing() {
        // int bad(int x) { if (x < 0) return -x; }
        // Should error: not all paths return
    }

    #[test]
    fn global_initializer() {
        // int global = computeValue();
    }
}
```

## Acceptance Criteria

- [ ] All script functions compiled to bytecode
- [ ] Method compilation with 'this' handling
- [ ] Constructor initializer lists work
- [ ] Default member initialization
- [ ] Destructor with member destruction
- [ ] Return path verification
- [ ] Global variable initializers compiled
- [ ] Bytecode stored in output correctly
- [ ] Namespace scoping preserved
- [ ] All tests pass

## Next Phase

Task 47: Integration & Testing (end-to-end compilation tests)
