# Semantic Analysis Implementation Plan

**Status:** Ready for Implementation
**Created:** 2025-11-24 (Updated: 2025-11-25)
**Phase:** Post-Parser, Pre-Codegen

---

## Overview & Philosophy

### Compilation Pipeline (2-Pass Registry-Only Model)

```
Source Code
    ↓
┌─────────────────────────────────────────────────────────────┐
│ Phase 1: LEXER (✅ Complete)                                │
│ Input:  Raw source text                                     │
│ Output: Token stream with spans                             │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│ Phase 2: PARSER (✅ Complete)                               │
│ Input:  Token stream                                        │
│ Output: Abstract Syntax Tree (AST)                          │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│ Phase 3: SEMANTIC ANALYSIS (2 passes)                       │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ Pass 1: Registration (⏳ To Be Implemented)             │ │
│ │ • Register all global names in Registry                │ │
│ │ • Types: Classes, interfaces, enums, funcdefs          │ │
│ │ • Functions: Global and methods (names only)           │ │
│ │ • Global variables (names only)                        │ │
│ │ • Track namespace/class context dynamically            │ │
│ │ • NO local variable tracking                           │ │
│ │ • NO type resolution yet                               │ │
│ │ Output: Registry (empty shells with qualified names)   │ │
│ └─────────────────────────────────────────────────────────┘ │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ Pass 2: Compilation & Codegen (⏳ To Be Implemented)   │ │
│ │                                                          │ │
│ │ Sub-phase 2a: Type Compilation                          │ │
│ │ • Fill in type details (fields, methods, inheritance)   │ │
│ │ • Resolve TypeExpr → DataType                           │ │
│ │ • Instantiate templates with caching                    │ │
│ │ • Register complete function signatures                 │ │
│ │ • Build type hierarchy                                  │ │
│ │ Output: Registry (complete type information)            │ │
│ │                                                          │ │
│ │ Sub-phase 2b: Function Compilation (per-function)       │ │
│ │ • Type check expressions                                │ │
│ │ • Track local variables dynamically (LocalScope)        │ │
│ │ • Validate operations and control flow                  │ │
│ │ • Generate bytecode                                     │ │
│ │ Output: Module { bytecode, metadata }                   │ │
│ └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## Architecture Decision: 2-Pass Registry-Only Model

**Following AngelScript C++ Architecture:**

```cpp
// AngelScript C++ (simplified)
ParseScripts();              // Parse → AST
CompileClasses();            // Pass 1: Register + fill type details
CompileFunctions();          // Pass 2: Compile + codegen
```

**Our Rust Implementation:**

```
Pass 1: Registration
  - Walk AST tracking namespace/class context
  - Register all global names in Registry (types, functions, globals)
  - Output: Registry with qualified names (empty shells)

Pass 2: Compilation & Codegen
  Sub-phase 2a: Type Compilation
    - Fill type details from AST
    - Resolve all TypeExpr → DataType
    - Instantiate templates (with caching)

  Sub-phase 2b: Function Compilation (per-function)
    - Type check function bodies
    - Track local variables dynamically (no global SymbolTable)
    - Emit bytecode
```

### Key Architectural Principles

1. **Registry is the single source of truth for globals**
   - All types (classes, interfaces, enums, primitives)
   - All functions (global and methods) with qualified names
   - All global variables
   - Template instantiation cache

2. **No SymbolTable for globals**
   - Registry replaces SymbolTable for global names
   - Simpler, clearer separation of concerns
   - Matches AngelScript C++ architecture

3. **Local variables tracked dynamically**
   - Not stored in global tables
   - Tracked per-function during compilation (Pass 2b)
   - Uses LocalScope structure (stack-based, temporary)

4. **Qualified names for scoped items**
   - `Namespace::Class` for types
   - `Namespace::function` for functions
   - Built dynamically as we walk AST in Pass 1

5. **Two sub-phases in Pass 2**
   - First: Fill all type details (classes need complete info before function compilation)
   - Second: Compile function bodies (can now look up complete types)

---

## Why Multiple Passes?

### Forward References
```angelscript
void foo() {
    bar();  // bar used before defined
}
void bar() { }  // Must register names first
```

### Type Dependencies
```angelscript
class Base { }
class Derived : Base { }  // Base must be registered first

array<Player@> players;  // Need Player registered, then instantiate array<Player@>
```

### Complex Type System
- Templates with nested parameters: `dict<string, array<int>>`
- Scoped types: `Namespace::Type<T>`
- Class inheritance and interfaces
- Method overloading and overriding

---

## Pass 1: Registration

### Goals

- Register ALL global names in Registry
- Types: Classes, interfaces, enums, funcdefs
- Functions: Global and methods (name + location, no signature yet)
- Global variables: Name + location (no type yet)
- Track namespace/class context as we walk AST
- Build qualified names (e.g., `Namespace::Class`, `Namespace::func`)
- NO local variable tracking (that's Pass 2b)
- NO type resolution (that's Pass 2a)

### Input/Output

**Input:** `Script<'src, 'ast>` (AST from parser)

**Output:**
```rust
pub struct RegistrationData {
    /// Registry with all global names (empty shells)
    pub registry: Registry,

    /// Errors found during registration
    pub errors: Vec<SemanticError>,
}
```

### Data Structures

#### Registry (Global Names Only)

```rust
/// Central storage for all global names (types, functions, variables)
pub struct Registry {
    // Types
    types: Vec<TypeDef>,
    type_by_name: FxHashMap<String, TypeId>,  // "Namespace::Class" → TypeId

    // Functions (with overloading support)
    functions: Vec<FunctionDef>,
    func_by_name: FxHashMap<String, Vec<FunctionId>>,  // "Namespace::foo" → [FunctionId, ...]

    // Template instantiation cache
    template_cache: FxHashMap<(TypeId, Vec<DataType>), TypeId>,

    // Fixed TypeIds for primitives
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
    // Built-in types
    pub string_type: TypeId,
    pub array_template: TypeId,
    pub dict_template: TypeId,
}
```

#### TypeDef (Type Definition - Initially Empty)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub u32);

// Fixed TypeIds for primitives
pub const VOID_TYPE: TypeId = TypeId(0);
pub const BOOL_TYPE: TypeId = TypeId(1);
pub const INT8_TYPE: TypeId = TypeId(2);
pub const INT16_TYPE: TypeId = TypeId(3);
pub const INT32_TYPE: TypeId = TypeId(4);  // "int" alias
pub const INT64_TYPE: TypeId = TypeId(5);
pub const UINT8_TYPE: TypeId = TypeId(6);
pub const UINT16_TYPE: TypeId = TypeId(7);
pub const UINT32_TYPE: TypeId = TypeId(8);  // "uint" alias
pub const UINT64_TYPE: TypeId = TypeId(9);
pub const FLOAT_TYPE: TypeId = TypeId(10);
pub const DOUBLE_TYPE: TypeId = TypeId(11);

// Built-in types
pub const STRING_TYPE: TypeId = TypeId(16);
pub const ARRAY_TEMPLATE: TypeId = TypeId(17);
pub const DICT_TEMPLATE: TypeId = TypeId(18);

const FIRST_USER_TYPE_ID: u32 = 32;

pub enum TypeDef {
    Primitive {
        kind: PrimitiveType,
    },

    Class {
        name: String,
        qualified_name: String,  // "Namespace::Class"
        fields: Vec<FieldDef>,   // Empty in Pass 1, filled in Pass 2a
        methods: Vec<FunctionId>,
        base_class: Option<TypeId>,
        interfaces: Vec<TypeId>,
    },

    Interface {
        name: String,
        qualified_name: String,
        methods: Vec<MethodSignature>,
    },

    Enum {
        name: String,
        qualified_name: String,
        values: Vec<(String, i64)>,
    },

    Funcdef {
        name: String,
        qualified_name: String,
        params: Vec<DataType>,      // Empty in Pass 1
        return_type: DataType,      // Empty in Pass 1
    },

    Template {
        name: String,
        param_count: usize,
    },

    TemplateInstance {
        template: TypeId,
        sub_types: Vec<DataType>,
    },
}
```

#### Registrar (Pass 1 Traversal State)

```rust
/// Performs Pass 1: Registration of global names
pub struct Registrar<'src, 'ast> {
    /// The registry we're building
    registry: Registry,

    /// Current namespace path (e.g., ["NamespaceA", "NamespaceB"])
    namespace_path: Vec<String>,

    /// Current class (if inside a class)
    current_class: Option<TypeId>,

    /// Errors found
    errors: Vec<SemanticError>,
}
```

### Algorithm (Single O(n) Traversal)

```rust
impl<'src, 'ast> Registrar<'src, 'ast> {
    pub fn register(script: &Script<'src, 'ast>) -> RegistrationData {
        let mut registrar = Self::new();
        registrar.visit_script(script);

        RegistrationData {
            registry: registrar.registry,
            errors: registrar.errors,
        }
    }

    fn visit_script(&mut self, script: &Script) {
        for item in script.items() {
            self.visit_item(item);
        }
    }

    fn visit_item(&mut self, item: &Item) {
        match item {
            Item::Namespace(ns) => {
                // Track namespace context
                self.namespace_path.push(ns.name.to_string());

                for item in ns.items {
                    self.visit_item(item);
                }

                self.namespace_path.pop();
            }

            Item::Class(class) => {
                // Build qualified name
                let qualified_name = self.build_qualified_name(class.name);

                // Register type (empty shell)
                let type_id = self.registry.register_type(TypeDef::Class {
                    name: class.name.to_string(),
                    qualified_name,
                    fields: Vec::new(),  // Filled in Pass 2a
                    methods: Vec::new(),
                    base_class: None,
                    interfaces: Vec::new(),
                });

                // Enter class context
                self.current_class = Some(type_id);

                // Register methods (names only)
                for member in class.members {
                    if let ClassMember::Method(method) = member {
                        let qualified_method_name = self.build_qualified_name(method.name);
                        self.registry.register_function_name(qualified_method_name, type_id);
                    }
                }

                self.current_class = None;
            }

            Item::Function(func) => {
                let qualified_name = self.build_qualified_name(func.name);
                self.registry.register_function_name(qualified_name, None);
            }

            Item::GlobalVar(var) => {
                let qualified_name = self.build_qualified_name(var.name);
                self.registry.register_global_var_name(qualified_name);
            }

            Item::Interface(iface) => {
                let qualified_name = self.build_qualified_name(iface.name);
                self.registry.register_type(TypeDef::Interface {
                    name: iface.name.to_string(),
                    qualified_name,
                    methods: Vec::new(),  // Filled in Pass 2a
                });
            }

            Item::Enum(enum_decl) => {
                let qualified_name = self.build_qualified_name(enum_decl.name);
                let values = enum_decl.values.iter()
                    .map(|v| (v.name.to_string(), v.value))
                    .collect();

                self.registry.register_type(TypeDef::Enum {
                    name: enum_decl.name.to_string(),
                    qualified_name,
                    values,
                });
            }
        }
    }

    fn build_qualified_name(&self, name: &str) -> String {
        if self.namespace_path.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.namespace_path.join("::"), name)
        }
    }
}
```

### What Pass 1 Does NOT Do

- ❌ Does NOT track local variables (that's Pass 2b)
- ❌ Does NOT resolve type expressions (that's Pass 2a)
- ❌ Does NOT validate inheritance (that's Pass 2a)
- ❌ Does NOT register function signatures (that's Pass 2a)
- ❌ Does NOT type check anything (that's Pass 2b)

### Implementation Tasks for Pass 1

1. Create `Registry` structure with fixed primitive TypeIds
2. Implement `TypeDef` enum
3. Implement `Registrar` visitor
4. Add namespace path tracking
5. Register all global types (classes, interfaces, enums)
6. Register all global function names
7. Register all global variable names
8. Handle error cases (duplicate names)
9. Write tests (20-30 tests)

---

## Pass 2: Compilation & Codegen

Pass 2 has two distinct sub-phases that must run in order:

### Sub-phase 2a: Type Compilation

**Goals:**
- Fill in all type details (fields, methods, inheritance)
- Resolve all `TypeExpr` AST nodes → `DataType`
- Instantiate template types (with caching)
- Register complete function signatures
- Build type hierarchy (inheritance, interfaces)

**Input:**
- `Script<'src, 'ast>` (AST)
- `Registry` (from Pass 1, with names only)

**Output:**
```rust
pub struct TypeCompilationData {
    /// Registry with complete type information
    pub registry: Registry,

    /// Maps AST TypeExpr spans to resolved DataType
    pub type_map: FxHashMap<Span, DataType>,

    /// Inheritance relationships (Derived → Base)
    pub inheritance: FxHashMap<TypeId, TypeId>,

    /// Interface implementations (Class → [Interfaces])
    pub implements: FxHashMap<TypeId, Vec<TypeId>>,

    /// Errors found
    pub errors: Vec<SemanticError>,
}
```

### Data Structures for Pass 2a

#### DataType (Complete Type with Modifiers)

```rust
/// A complete type including modifiers (const, handle, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DataType {
    pub type_id: TypeId,
    pub is_const: bool,
    pub is_handle: bool,
    pub is_handle_to_const: bool,
}

impl DataType {
    pub fn simple(type_id: TypeId) -> Self {
        Self {
            type_id,
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
        }
    }

    pub fn with_const(type_id: TypeId) -> Self {
        Self {
            type_id,
            is_const: true,
            is_handle: false,
            is_handle_to_const: false,
        }
    }

    pub fn with_handle(type_id: TypeId, is_const: bool) -> Self {
        Self {
            type_id,
            is_const: false,
            is_handle: true,
            is_handle_to_const: is_const,
        }
    }
}
```

#### TypeCompiler (Pass 2a State)

```rust
/// Performs Pass 2a: Type Compilation
pub struct TypeCompiler<'src, 'ast> {
    /// Registry (mutable, filling in details)
    registry: Registry,

    /// Maps AST spans to resolved types
    type_map: FxHashMap<Span, DataType>,

    /// Current namespace path
    namespace_path: Vec<String>,

    /// Inheritance tracking
    inheritance: FxHashMap<TypeId, TypeId>,
    implements: FxHashMap<TypeId, Vec<TypeId>>,

    /// Errors
    errors: Vec<SemanticError>,
}
```

### Algorithm for Pass 2a (Single O(n) Traversal)

```rust
impl<'src, 'ast> TypeCompiler<'src, 'ast> {
    pub fn compile(
        script: &Script<'src, 'ast>,
        registry: Registry,
    ) -> TypeCompilationData {
        let mut compiler = Self::new(registry);

        // Walk AST and fill in type details
        compiler.visit_script(script);

        TypeCompilationData {
            registry: compiler.registry,
            type_map: compiler.type_map,
            inheritance: compiler.inheritance,
            implements: compiler.implements,
            errors: compiler.errors,
        }
    }

    fn visit_item(&mut self, item: &Item) {
        match item {
            Item::Class(class) => {
                let qualified_name = self.build_qualified_name(class.name);
                let type_id = self.registry.lookup_type(&qualified_name).unwrap();

                // Resolve base class
                let base_class = if let Some(base) = &class.base_class {
                    Some(self.resolve_type_expr(base)?.type_id)
                } else {
                    None
                };

                // Resolve interfaces
                let interfaces = class.interfaces.iter()
                    .map(|i| self.resolve_type_expr(i))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .map(|dt| dt.type_id)
                    .collect();

                // Fill in fields
                let fields = class.members.iter()
                    .filter_map(|m| match m {
                        ClassMember::Field(f) => Some(FieldDef {
                            name: f.name.to_string(),
                            data_type: self.resolve_type_expr(&f.type_expr)?,
                            visibility: f.visibility,
                        }),
                        _ => None,
                    })
                    .collect();

                // Update type definition
                self.registry.fill_class_details(type_id, fields, base_class, interfaces);

                // Register method signatures
                for member in class.members {
                    if let ClassMember::Method(method) = member {
                        self.register_function_signature(method, Some(type_id));
                    }
                }
            }

            Item::Function(func) => {
                self.register_function_signature(func, None);
            }

            // ... other items
        }
    }

    fn resolve_type_expr(&mut self, expr: &TypeExpr) -> Result<DataType, SemanticError> {
        // Resolve base type name
        let qualified_name = self.build_qualified_name(expr.base.name());
        let type_id = self.registry.lookup_type(&qualified_name)
            .ok_or_else(|| SemanticError::undefined_type(expr.base.name(), expr.span))?;

        // Handle template arguments
        let type_id = if !expr.template_args.is_empty() {
            let arg_types = expr.template_args.iter()
                .map(|arg| self.resolve_type_expr(arg))
                .collect::<Result<Vec<_>, _>>()?;

            self.registry.instantiate_template(type_id, arg_types)?
        } else {
            type_id
        };

        // Build DataType with modifiers
        let mut data_type = DataType::simple(type_id);

        // Apply modifiers from suffixes
        for suffix in &expr.suffixes {
            match suffix {
                TypeSuffix::Handle { is_const } => {
                    data_type.is_handle = true;
                    data_type.is_handle_to_const = *is_const;
                }
            }
        }

        // Apply const modifier
        if expr.is_const {
            data_type.is_const = true;
        }

        // Store in type map
        self.type_map.insert(expr.span, data_type.clone());

        Ok(data_type)
    }

    fn register_function_signature(
        &mut self,
        func: &FunctionDecl,
        object_type: Option<TypeId>,
    ) {
        let qualified_name = self.build_qualified_name(func.name);

        let params = func.params.iter()
            .map(|p| self.resolve_type_expr(&p.type_expr))
            .collect::<Result<Vec<_>, _>>()?;

        let return_type = self.resolve_type_expr(&func.return_type)?;

        self.registry.register_function_signature(
            qualified_name,
            params,
            return_type,
            object_type,
        );
    }
}
```

### Implementation Tasks for Pass 2a

1. Create `DataType` structure
2. Implement `TypeCompiler` visitor
3. Implement `resolve_type_expr` (TypeExpr → DataType)
4. Fill in class details (fields, methods, inheritance)
5. Register complete function signatures
6. Implement template instantiation with caching
7. Build type hierarchy
8. Handle errors (undefined types, circular inheritance)
9. Write tests (40-50 tests)

---

### Sub-phase 2b: Function Compilation

**Goals:**
- Type check all function bodies
- Track local variables dynamically (per-function)
- Validate all expressions and statements
- Generate bytecode

**Input:**
- `Script<'src, 'ast>` (AST)
- `Registry` (from Pass 2a, complete)

**Output:**
```rust
pub struct Module {
    /// Compiled bytecode for all functions
    pub bytecode: Vec<u8>,

    /// Metadata (function locations, debug info, etc.)
    pub metadata: ModuleMetadata,

    /// Errors found
    pub errors: Vec<SemanticError>,
}
```

### Data Structures for Pass 2b

#### LocalScope (Per-Function Local Variable Tracking)

```rust
/// Tracks local variables for a single function compilation
pub struct LocalScope {
    /// Variables in current function (name → info)
    variables: FxHashMap<String, LocalVar>,

    /// Current scope depth (for nested blocks)
    scope_depth: u32,
}

pub struct LocalVar {
    pub name: String,
    pub data_type: DataType,
    pub scope_depth: u32,
    pub stack_offset: u32,
    pub is_mutable: bool,
}

impl LocalScope {
    pub fn new() -> Self {
        Self {
            variables: FxHashMap::default(),
            scope_depth: 0,
        }
    }

    pub fn enter_scope(&mut self) {
        self.scope_depth += 1;
    }

    pub fn exit_scope(&mut self) {
        // Remove variables from this scope
        self.variables.retain(|_, v| v.scope_depth < self.scope_depth);
        self.scope_depth -= 1;
    }

    pub fn declare_variable(&mut self, name: String, data_type: DataType, offset: u32) {
        self.variables.insert(name.clone(), LocalVar {
            name,
            data_type,
            scope_depth: self.scope_depth,
            stack_offset: offset,
            is_mutable: true,
        });
    }

    pub fn lookup(&self, name: &str) -> Option<&LocalVar> {
        self.variables.get(name)
    }
}
```

#### FunctionCompiler (Per-Function Compilation State)

```rust
/// Compiles a single function (type checking + codegen)
pub struct FunctionCompiler<'src, 'ast> {
    /// Global registry (read-only)
    registry: &'ast Registry,

    /// Local variables for THIS function only
    local_scope: LocalScope,

    /// Bytecode emitter
    bytecode: BytecodeEmitter,

    /// Current function's return type
    return_type: DataType,

    /// Loop depth (for break/continue validation)
    loop_depth: u32,

    /// Errors
    errors: Vec<SemanticError>,
}

impl<'src, 'ast> FunctionCompiler<'src, 'ast> {
    pub fn compile_function(
        func: &FunctionDecl<'src, 'ast>,
        registry: &Registry,
    ) -> CompiledFunction {
        let mut compiler = Self::new(registry, func.return_type);

        // Declare parameters as local variables
        for (i, param) in func.params.iter().enumerate() {
            compiler.local_scope.declare_variable(
                param.name.to_string(),
                param.data_type.clone(),
                i as u32,
            );
        }

        // Compile function body
        compiler.visit_block(func.body);

        CompiledFunction {
            bytecode: compiler.bytecode.finish(),
            errors: compiler.errors,
        }
    }

    fn visit_block(&mut self, block: &Block) {
        self.local_scope.enter_scope();

        for stmt in block.statements {
            self.visit_statement(stmt);
        }

        self.local_scope.exit_scope();
    }

    fn visit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VarDecl(var) => {
                // Resolve type
                let data_type = self.resolve_type_expr(&var.type_expr)?;

                // Check initializer
                if let Some(init) = &var.initializer {
                    let init_type = self.check_expr(init)?;
                    if !self.is_assignable(init_type, data_type) {
                        self.error(SemanticErrorKind::TypeMismatch, var.span);
                    }
                }

                // Declare local variable
                let offset = self.bytecode.next_stack_offset();
                self.local_scope.declare_variable(
                    var.name.to_string(),
                    data_type,
                    offset,
                );
            }

            Statement::Expression(expr) => {
                self.check_expr(expr)?;
            }

            Statement::If(if_stmt) => {
                let cond_type = self.check_expr(&if_stmt.condition)?;
                if cond_type.type_id != self.registry.bool_type {
                    self.error(SemanticErrorKind::TypeMismatch, if_stmt.condition.span());
                }

                self.visit_block(&if_stmt.then_branch);
                if let Some(else_branch) = &if_stmt.else_branch {
                    self.visit_block(else_branch);
                }
            }

            Statement::While(while_stmt) => {
                self.loop_depth += 1;
                self.visit_block(&while_stmt.body);
                self.loop_depth -= 1;
            }

            Statement::Return(ret) => {
                let return_type = if let Some(expr) = &ret.value {
                    self.check_expr(expr)?
                } else {
                    DataType::simple(self.registry.void_type)
                };

                if !self.is_assignable(return_type, self.return_type) {
                    self.error(SemanticErrorKind::TypeMismatch, ret.span);
                }
            }

            Statement::Break | Statement::Continue => {
                if self.loop_depth == 0 {
                    self.error(SemanticErrorKind::BreakOutsideLoop, stmt.span());
                }
            }
        }
    }

    fn check_expr(&mut self, expr: &Expression) -> Result<DataType, SemanticError> {
        match expr {
            Expression::Identifier(ident) => {
                // Look up in local scope first
                if let Some(local_var) = self.local_scope.lookup(ident.name) {
                    return Ok(local_var.data_type.clone());
                }

                // Then look up in global scope (Registry)
                if let Some(global_var) = self.registry.lookup_global_var(ident.name) {
                    return Ok(global_var.data_type.clone());
                }

                Err(SemanticError::undefined_variable(ident.name, ident.span))
            }

            Expression::Binary(binary) => {
                let left_type = self.check_expr(&binary.left)?;
                let right_type = self.check_expr(&binary.right)?;
                self.check_binary_op(binary.op, left_type, right_type, binary.span)
            }

            Expression::Call(call) => {
                // ... function call type checking
            }

            // ... other expressions
        }
    }
}
```

### Implementation Tasks for Pass 2b

1. Create `LocalScope` structure
2. Implement `FunctionCompiler`
3. Implement expression type checking (all expression types)
4. Implement statement validation
5. Add bytecode emission
6. Handle control flow validation (break/continue/return)
7. Implement operator type rules
8. Write tests (60-80 tests)

---

## File Structure

```
src/semantic/
├── mod.rs                    # Public API, exports
│
├── error.rs                  # Error types (✅ Exists)
│
├── registry.rs               # NEW: Registry (global types/functions)
├── data_type.rs              # NEW: DataType with modifiers
├── type_def.rs               # NEW: TypeDef enum, TypeId constants
│
├── registrar.rs              # NEW: Pass 1 implementation
├── type_compiler.rs          # NEW: Pass 2a implementation
├── function_compiler.rs      # NEW: Pass 2b implementation
├── local_scope.rs            # NEW: LocalScope for function compilation
│
├── bytecode.rs               # NEW: Bytecode emitter
│
├── resolver.rs               # EXISTING: To be simplified/removed later
├── scope.rs                  # EXISTING: May repurpose for LocalScope
└── symbol_table.rs           # EXISTING: To be removed later

docs/
└── semantic_analysis.md      # Documentation

tests/
├── registration_tests.rs     # Pass 1 tests
├── type_compilation_tests.rs # Pass 2a tests
└── codegen_tests.rs          # Pass 2b tests
```

**Total Estimates:**
- Pass 1 (Registration): ~300-400 lines
- Pass 2a (Type Compilation): ~700-900 lines
- Pass 2b (Function Compilation): ~800-1000 lines
- Support structures: ~500-700 lines
- **Total new code: ~2300-3000 lines**
- **Tests: ~100-150 test functions**

---

## Testing Strategy

### Pass 1 Tests (20-30 tests)

```rust
#[test]
fn register_simple_class() {
    let source = "class Player { }";
    let data = Registrar::register(parse(source));
    assert!(data.registry.lookup_type("Player").is_some());
}

#[test]
fn register_namespaced_class() {
    let source = "namespace Game { class Player { } }";
    let data = Registrar::register(parse(source));
    assert!(data.registry.lookup_type("Game::Player").is_some());
}

#[test]
fn register_function_names() {
    let source = "void foo() { }";
    let data = Registrar::register(parse(source));
    assert!(data.registry.lookup_function("foo").is_some());
}
```

### Pass 2a Tests (40-50 tests)

```rust
#[test]
fn resolve_primitive_types() {
    let source = "class Player { int health; }";
    let data = TypeCompiler::compile(parse(source), registry);
    let player_type = data.registry.lookup_type("Player").unwrap();
    // Check that health field has type int
}

#[test]
fn instantiate_template() {
    let source = "array<int> numbers;";
    let data = TypeCompiler::compile(parse(source), registry);
    // Check that array<int> was instantiated
}

#[test]
fn error_undefined_type() {
    let source = "Undefined x;";
    let data = TypeCompiler::compile(parse(source), registry);
    assert!(!data.errors.is_empty());
}
```

### Pass 2b Tests (60-80 tests)

```rust
#[test]
fn compile_simple_function() {
    let source = "void foo() { int x = 5; }";
    let module = compile(source);
    assert!(module.errors.is_empty());
}

#[test]
fn error_type_mismatch() {
    let source = "void foo() { int x = \"hello\"; }";
    let module = compile(source);
    assert!(!module.errors.is_empty());
}

#[test]
fn local_variable_shadowing() {
    let source = r#"
        void foo() {
            int x = 5;
            {
                int x = 10;  // Shadows outer x
            }
        }
    "#;
    let module = compile(source);
    assert!(module.errors.is_empty());
}
```

---

## Performance Constraints

### Target Performance

**Pass 1 (Registration):** < 0.5 ms for 5000 lines
**Pass 2a (Type Compilation):** < 0.7 ms for 5000 lines
**Pass 2b (Function Compilation):** < 0.8 ms for 5000 lines
**Total:** < 2.0 ms for full compilation (5000 lines)

### Performance Strategies

1. **Use FxHashMap** from `rustc_hash` (faster than std HashMap)
2. **Pre-allocate:** `Vec::with_capacity` based on AST size
3. **Cache template instantiations:** Same args → Same TypeId
4. **Use TypeId (u32) for comparisons:** Not String
5. **Inline hot functions:** `#[inline]` on lookup methods

### Benchmarks

Add to `benches/semantic_benchmarks.rs`:

```rust
group.bench_function("pass1_registration_5000_lines", |b| {
    let arena = Bump::new();
    let (script, _) = parse_lenient(stress_test, &arena);
    b.iter(|| Registrar::register(&script));
});

group.bench_function("pass2a_type_compilation_5000_lines", |b| {
    let arena = Bump::new();
    let (script, _) = parse_lenient(stress_test, &arena);
    let registration = Registrar::register(&script);
    b.iter(|| TypeCompiler::compile(&script, registration.registry.clone()));
});
```

---

## Implementation Timeline

### Phase 1: Foundation (~3-4 days) ✅ COMPLETE
- [x] Create `Registry`, `TypeDef`, `DataType` structures
- [x] Implement fixed TypeIds for primitives
- [x] Write foundation tests

### Phase 2: Pass 1 - Registration (~3-4 days) ✅ COMPLETE
- [x] Implement `Registrar` visitor
- [x] Add namespace/class context tracking
- [x] Register all global names
- [x] Write registration tests (24 tests, all passing)

### Phase 3: Pass 2a - Type Compilation (~5-6 days) ✅ COMPLETE
- [x] Implement `TypeCompiler` visitor
- [x] Implement `resolve_type_expr`
- [x] Fill in type details
- [x] Implement template instantiation (uses Registry cache)
- [x] Write type compilation tests (7 tests, all passing)

### Phase 4: Pass 2b - Function Compilation (~7-9 days) ✅ BASIC IMPLEMENTATION COMPLETE
- [x] Implement `LocalScope` ✅
- [x] Implement `FunctionCompiler` ✅
- [x] Add expression type checking (11/14 expressions) ✅
- [x] Add bytecode emission ✅
- [x] Write codegen tests (basic coverage) ✅
- [ ] **CRITICAL MISSING FEATURES** (see below)

### Phase 5: Critical Type System Features (~6-8 weeks)
**Current Status:** Basic implementation covers ~30-40% of production AngelScript code. The following features are CRITICAL for realistic code compilation:

#### Week 1-2: Type Conversions & Object Construction (CRITICAL)
- [ ] Implement implicit type conversions (int → float, derived → base)
- [ ] Implement handle conversions (T@ → const T@)
- [ ] Add constructor call detection and compilation
- [ ] Implement initializer list support ({1, 2, 3})
- [ ] Update all type checking sites to attempt conversions
- [ ] Add comprehensive conversion tests

#### Week 3-4: Reference Semantics & Handles (CRITICAL)
- [ ] Extend DataType with reference modifiers (&in, &out, &inout)
- [ ] Implement reference parameter validation
- [ ] Implement handle (@) reference counting semantics
- [ ] Add handle null checking
- [ ] Implement auto-handle (@+) support
- [ ] Add reference/handle tests

#### Week 5-6: Operator Overloading (HIGH PRIORITY)
- [ ] Implement operator overload method lookup (opAdd, opMul, etc.)
- [ ] Integrate with binary/unary operation checking
- [ ] Support both member and global operator overloads
- [ ] Implement comparison operators (opEquals, opCmp)
- [ ] Add operator overloading tests

#### Week 7-8: Advanced Features (MEDIUM PRIORITY)
- [ ] Implement property accessor detection (get_/set_ methods)
- [ ] Add default argument support
- [ ] Implement lambda expressions with capture
- [ ] Add comprehensive integration tests

### Phase 6: Integration & Polish (~2-3 weeks)
- [ ] Integration tests with real AngelScript samples
- [ ] Performance benchmarks
- [ ] Documentation
- [ ] Simplify/remove old Pass 1 code

**Total Estimated Time:**
- Phase 4 (Basic): ✅ Complete (21-27 days)
- Phase 5 (Critical Features): 6-8 weeks remaining
- Phase 6 (Polish): 2-3 weeks
- **Total Remaining: 8-11 weeks**

---

## Success Criteria

### Feature Completeness

**Completed (Basic Implementation):**
- [x] Registry implemented with fixed primitive TypeIds ✅
- [x] Pass 1 registers all global names ✅
- [x] Pass 2a fills in all type details ✅
- [x] Pass 2a resolves all TypeExpr → DataType ✅
- [x] Pass 2a instantiates templates with caching ✅
- [x] Pass 2b basic expression type checking (11/14 expressions) ✅
- [x] Pass 2b all statement types (13/13) ✅
- [x] Pass 2b tracks local variables dynamically ✅
- [x] Pass 2b basic bytecode emission ✅
- [x] Error messages with source location ✅

**Critical Missing Features (Blocks Realistic Code):**
- [ ] Type conversions (implicit casts, handle conversions)
- [ ] Constructor/destructor calls
- [ ] Reference parameter semantics (&in, &out, &inout)
- [ ] Handle type (@) semantics and reference counting
- [ ] Operator overloading resolution
- [ ] Property accessors (get/set)
- [ ] Default arguments
- [ ] Lambda expressions with capture
- [ ] Complete initializer list support

### Test Coverage

- [x] 24 registration tests ✅
- [x] 30 data_type tests ✅
- [x] 27 type_def tests ✅
- [x] 53 registry tests ✅
- [x] 7 type_compiler tests ✅
- [x] **Total: 141 tests passing** ✅
- [ ] 60-80 function compilation tests
- [ ] All tests passing
- [ ] ~120-160 total test functions

### Documentation

- [ ] Module documentation (rustdoc)
- [ ] Public API documented with examples
- [ ] Architecture decisions logged

### Quality Metrics

- [ ] No compiler warnings
- [ ] All clippy lints passing
- [ ] Clear error messages with spans
- [ ] **Performance: < 2ms total for 5000 lines**
- [ ] Memory efficient (pre-allocation, caching)

---

## Migration from Current Implementation

The current codebase has a Pass 1 implementation using `Resolver` and `SymbolTable`. Here's how we'll transition:

### Current State
- ✅ `resolver.rs` - Implements Pass 1 with SymbolTable
- ✅ `symbol_table.rs` - Stores all symbols (global and local)
- ✅ `scope.rs` - Scope stack management
- ✅ `error.rs` - Error types

### Migration Strategy

**Phase 1: Implement New System**
1. Implement Registry, Registrar, TypeCompiler, FunctionCompiler
2. Keep old system running (don't break existing code)
3. Write tests for new system

**Phase 2: Switch Over**
1. Update main compilation pipeline to use new system
2. Mark old system as deprecated

**Phase 3: Cleanup**
1. Remove `symbol_table.rs` (replaced by Registry + LocalScope)
2. Simplify or remove `resolver.rs` (replaced by Registrar)
3. Repurpose `scope.rs` for LocalScope if useful

---

## References

- [Crafting Interpreters - Resolving and Binding](https://craftinginterpreters.com/resolving-and-binding.html)
- [Crafting Interpreters - A Map of the Territory](https://craftinginterpreters.com/a-map-of-the-territory.html)
- [AngelScript Documentation](https://www.angelcode.com/angelscript/sdk/docs/manual/)
- AngelScript C++ source: `as_builder.cpp`, `as_compiler.cpp`
- Project docs: `docs/architecture.md`

---

**Ready to begin implementation!**
