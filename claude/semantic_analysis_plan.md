# Semantic Analysis Implementation Plan

**Status:** Ready for Implementation
**Created:** 2025-11-24
**Phase:** Post-Parser, Pre-Codegen

---

## Overview & Philosophy

### Compilation Pipeline

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
│ Phase 3: SEMANTIC ANALYSIS (Next - 3 passes)                │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ Pass 1: Resolution & Symbol Collection                  │ │
│ │ • Collect all declarations                              │ │
│ │ • Build symbol tables with scope hierarchy              │ │
│ │ • Resolve names to declarations                         │ │
│ │ • Check for duplicates/undefined names                  │ │
│ └─────────────────────────────────────────────────────────┘ │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ Pass 2: Type Resolution                                 │ │
│ │ • Resolve TypeExpr → TypeId                             │ │
│ │ • Build type registry and hierarchy                     │ │
│ │ • Instantiate templates                                 │ │
│ │ • Validate types exist                                  │ │
│ └─────────────────────────────────────────────────────────┘ │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ Pass 3: Type Checking & Validation                      │ │
│ │ • Type check all expressions                            │ │
│ │ • Validate operations and control flow                  │ │
│ │ • Check class/interface contracts                       │ │
│ │ • Annotate AST with semantic info                       │ │
│ └─────────────────────────────────────────────────────────┘ │
│ Output: Validated, annotated AST ready for codegen          │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│ Phase 4: CODE GENERATION (Future)                           │
│ Input:  Validated AST + semantic data                       │
│ Output: Bytecode or executable code                         │
└─────────────────────────────────────────────────────────────┘
```

### Guiding Principles (from Crafting Interpreters)

1. **Separate semantic analysis from parsing** - Parser builds structure, semantic analysis validates meaning
2. **Use multiple focused passes** - Each pass has single responsibility (O(n) traversal)
3. **Stack-based scope tracking** - Push/pop scopes during traversal (matches execution model)
4. **Side tables for results** - Store semantic data separately, don't modify AST
5. **Declare/define two-phase** - Prevents self-reference bugs in variable initialization
6. **Fail fast on errors** - Report errors during analysis, not execution
7. **Resolve at compile-time** - Any work done now is work not done at runtime

### Why Multiple Passes?

**Forward References:**
```angelscript
void foo() {
    bar();  // bar used before defined
}
void bar() { }  // Must collect symbols first, then resolve
```

**Type Dependencies:**
```angelscript
class Base { }
class Derived : Base { }  // Base must be registered before checking inheritance

array<Player@> players;  // Need to resolve Player, then instantiate array<Player@>
```

**Complex Type System:**
- Templates with nested parameters: `dict<string, array<int>>`
- Scoped types: `Namespace::Type<T>`
- Class inheritance and interfaces
- Method overloading and overriding

---

## Pass 1: Resolution & Symbol Collection

### Goals

- Collect all declarations (functions, classes, variables, parameters)
- Build symbol table hierarchy (global → namespace → class → function → block)
- Resolve variable references to their declarations
- Check for duplicate declarations and undefined names
- Handle forward references correctly

### Input/Output

**Input:** `Script<'src, 'ast>` (raw AST from parser)

**Output:**
```rust
pub struct ResolutionData {
    /// All symbols organized by scope
    pub symbol_tables: HashMap<ScopeId, SymbolTable>,

    /// Maps AST nodes to their symbol definitions
    pub resolutions: HashMap<NodeId, SymbolId>,

    /// Scope hierarchy (parent relationships)
    pub scope_tree: ScopeTree,

    /// Errors found during resolution
    pub errors: Vec<SemanticError>,
}
```

### Data Structures

#### Symbol Representation

```rust
/// A declared identifier
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub declared_type: Option<TypeExpr>,  // Unresolved yet
    pub span: Span,
    pub is_defined: bool,  // Declare vs define separation
}

pub enum SymbolKind {
    Variable,
    Parameter,
    Function,
    Class,
    Interface,
    Enum,
    Namespace,
    Field,
}
```

#### Symbol Table (per-scope storage)

```rust
/// Storage for symbols in a single scope
pub struct SymbolTable {
    pub scope_id: ScopeId,
    pub scope_kind: ScopeKind,
    pub symbols: HashMap<String, SymbolId>,
    pub parent: Option<ScopeId>,
}

pub enum ScopeKind {
    Global,
    Namespace(String),
    Class(String),
    Function(String),
    Block,
}
```

#### Resolver (traversal state)

```rust
/// Performs name resolution pass
pub struct Resolver<'src, 'ast> {
    /// Stack of active scopes (push/pop during traversal)
    scope_stack: Vec<ScopeId>,

    /// All symbol tables indexed by scope
    symbol_tables: HashMap<ScopeId, SymbolTable>,

    /// Resolution results
    resolutions: HashMap<NodeId, SymbolId>,

    /// Context tracking
    current_function: Option<FunctionKind>,
    current_class: Option<ClassKind>,

    /// Error accumulation
    errors: Vec<SemanticError>,
}

enum FunctionKind {
    Function,
    Method,
    Constructor,
}

enum ClassKind {
    Class,
    Interface,
}
```

### Algorithm (Single O(n) Traversal)

```rust
impl<'src, 'ast> Resolver<'src, 'ast> {
    pub fn resolve(ast: &Script<'src, 'ast>) -> ResolutionData {
        let mut resolver = Self::new();
        resolver.visit_script(ast);
        resolver.into_result()
    }

    fn visit_script(&mut self, script: &Script) {
        // Global scope
        self.begin_scope(ScopeKind::Global);

        for decl in script.declarations {
            self.visit_declaration(decl);
        }

        self.end_scope();
    }

    fn visit_declaration(&mut self, decl: &Declaration) {
        match decl {
            Declaration::Function(func) => {
                // Declare function in current scope
                self.declare(func.name, SymbolKind::Function, func.span);
                self.define(func.name);  // Functions immediately usable

                // New scope for function body
                self.begin_scope(ScopeKind::Function(func.name));
                self.current_function = Some(FunctionKind::Function);

                // Parameters
                for param in func.params {
                    self.declare_and_define(param.name, SymbolKind::Parameter);
                }

                // Body
                self.visit_statement(func.body);

                self.end_scope();
                self.current_function = None;
            }

            Declaration::Class(class) => {
                // Declare class in current scope
                self.declare_and_define(class.name, SymbolKind::Class);

                // New scope for class body
                self.begin_scope(ScopeKind::Class(class.name));
                self.current_class = Some(ClassKind::Class);

                // Fields and methods
                for member in class.members {
                    match member {
                        ClassMember::Field(field) => {
                            self.declare_and_define(field.name, SymbolKind::Field);
                        }
                        ClassMember::Method(method) => {
                            self.visit_declaration(&Declaration::Function(method));
                        }
                    }
                }

                self.end_scope();
                self.current_class = None;
            }

            Declaration::Variable(var) => {
                // Two-phase: declare first, define after initializer
                self.declare(var.name, SymbolKind::Variable, var.span);

                if let Some(init) = var.initializer {
                    self.visit_expression(init);  // Resolve in initializer
                }

                self.define(var.name);  // Now usable
            }

            // ... other declarations
        }
    }

    fn visit_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Identifier(ident) => {
                // Resolve identifier to declaration
                if let Some(symbol_id) = self.lookup(ident.name) {
                    self.resolutions.insert(expr.node_id(), symbol_id);
                } else {
                    self.error(SemanticErrorKind::UndefinedVariable, ident.span);
                }
            }

            Expression::Binary(binary) => {
                self.visit_expression(binary.left);
                self.visit_expression(binary.right);
            }

            // ... other expressions
        }
    }

    // Scope management (stack-based)
    fn begin_scope(&mut self, kind: ScopeKind) {
        let scope_id = self.new_scope_id();
        let parent = self.scope_stack.last().copied();

        let table = SymbolTable {
            scope_id,
            scope_kind: kind,
            symbols: HashMap::new(),
            parent,
        };

        self.symbol_tables.insert(scope_id, table);
        self.scope_stack.push(scope_id);
    }

    fn end_scope(&mut self) {
        self.scope_stack.pop();
    }

    // Symbol management (declare/define pattern)
    fn declare(&mut self, name: &str, kind: SymbolKind, span: Span) {
        let scope = self.current_scope_mut();

        // Check for duplicate in current scope
        if scope.symbols.contains_key(name) {
            self.error(SemanticErrorKind::DuplicateDeclaration, span);
            return;
        }

        let symbol = Symbol {
            name: name.to_string(),
            kind,
            declared_type: None,
            span,
            is_defined: false,  // Not yet defined
        };

        let symbol_id = self.new_symbol_id(symbol);
        scope.symbols.insert(name.to_string(), symbol_id);
    }

    fn define(&mut self, name: &str) {
        if let Some(symbol_id) = self.current_scope().symbols.get(name) {
            self.symbols[*symbol_id].is_defined = true;
        }
    }

    fn declare_and_define(&mut self, name: &str, kind: SymbolKind) {
        // Convenience for symbols that are immediately usable
        self.declare(name, kind, Span::default());
        self.define(name);
    }

    // Lookup (searches scope chain)
    fn lookup(&self, name: &str) -> Option<SymbolId> {
        // Walk up scope stack
        for scope_id in self.scope_stack.iter().rev() {
            if let Some(symbol_id) = self.symbol_tables[scope_id].symbols.get(name) {
                // Check if defined (prevent self-reference in initializer)
                if self.symbols[*symbol_id].is_defined {
                    return Some(*symbol_id);
                } else {
                    self.error(SemanticErrorKind::UseBeforeDefinition, span);
                    return None;
                }
            }
        }
        None  // Not found in any scope
    }
}
```

### Semantic Checks Performed

1. **Duplicate declaration**: Same name declared twice in same scope
2. **Undefined variable**: Variable used but never declared
3. **Use before definition**: Variable referenced in its own initializer
4. **Return outside function**: `return` statement not in function body
5. **Break/continue outside loop**: Control flow statements in wrong context

### Implementation Tasks

1. Define `Symbol`, `SymbolKind`, `SymbolTable` types
2. Implement `Resolver` with scope stack
3. Implement visitor pattern for AST traversal
4. Add declare/define logic
5. Implement lookup with scope chain walking
6. Add error reporting
7. Handle all declaration types (function, class, variable, etc.)
8. Handle all expression types that reference names
9. Track context (current function, current class)
10. Write comprehensive tests

### Test Coverage

- Basic variable declaration and reference
- Forward function references
- Variable shadowing in nested scopes
- Duplicate declaration errors
- Undefined variable errors
- Use before definition (self-reference in initializer)
- Return outside function
- Break/continue outside loop
- Class member access
- Namespace resolution

**Estimated:** 30-40 tests

### Files to Create

- `src/semantic/resolver.rs` (~400-500 lines)
- `src/semantic/scope.rs` (~150-200 lines)
- `src/semantic/symbol_table.rs` (~200-250 lines)
- `src/semantic/error.rs` (~150-200 lines)

**Total:** ~900-1150 lines

---

## Pass 2: Type Resolution

### Goals

- Resolve all `TypeExpr` AST nodes to concrete `TypeId`
- Register built-in primitive types
- Collect user-defined types (classes, enums, interfaces)
- Build type hierarchy (inheritance, interface implementation)
- Instantiate template types
- Validate all types exist and are accessible

### Input/Output

**Input:**
- `Script<'src, 'ast>` (AST)
- `ResolutionData` (from Pass 1)

**Output:**
```rust
pub struct TypeResolutionData {
    /// Registry of all types
    pub type_registry: TypeRegistry,

    /// Maps AST TypeExpr nodes to resolved TypeId
    pub type_map: HashMap<NodeId, TypeId>,

    /// Class inheritance relationships
    pub inheritance: HashMap<TypeId, TypeId>,  // Derived → Base

    /// Interface implementations
    pub implements: HashMap<TypeId, Vec<TypeId>>,  // Class → Interfaces

    /// Errors found during resolution
    pub errors: Vec<SemanticError>,
}
```

### Data Structures

#### Type System Core

```rust
/// Unique identifier for a type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(u32);

/// Complete type definition
pub enum TypeDef {
    Primitive {
        kind: PrimitiveType,  // void, bool, int, float, etc.
        size: usize,
    },

    Class {
        name: String,
        fields: Vec<FieldDef>,
        methods: Vec<MethodDef>,
        base_class: Option<TypeId>,
        interfaces: Vec<TypeId>,
    },

    Interface {
        name: String,
        methods: Vec<MethodSignature>,
    },

    Enum {
        name: String,
        values: Vec<(String, i64)>,
    },

    Funcdef {
        name: String,
        params: Vec<TypeId>,
        return_type: TypeId,
    },

    Array {
        element_type: TypeId,
        dimensions: u32,
    },

    Template {
        name: String,
        params: Vec<String>,  // Template parameter names
    },

    TemplateInstance {
        template: TypeId,
        args: Vec<TypeId>,
    },
}

pub struct FieldDef {
    pub name: String,
    pub type_id: TypeId,
    pub access: AccessLevel,
}

pub struct MethodDef {
    pub name: String,
    pub params: Vec<TypeId>,
    pub return_type: TypeId,
    pub access: AccessLevel,
    pub is_virtual: bool,
}

pub enum AccessLevel {
    Private,
    Protected,
    Public,
}
```

#### Type Registry

```rust
/// Central storage for all types
pub struct TypeRegistry {
    types: Vec<TypeDef>,  // TypeId is index into this
    by_name: HashMap<String, TypeId>,

    // Built-in type IDs (for convenience)
    pub void_type: TypeId,
    pub bool_type: TypeId,
    pub int_type: TypeId,
    pub float_type: TypeId,
    pub double_type: TypeId,
    // ... other primitives
}

impl TypeRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            types: Vec::new(),
            by_name: HashMap::new(),
            void_type: TypeId(0),
            bool_type: TypeId(0),
            // ...
        };

        // Pre-populate with built-in types
        registry.register_primitives();
        registry
    }

    fn register_primitives(&mut self) {
        self.void_type = self.register(TypeDef::Primitive {
            kind: PrimitiveType::Void,
            size: 0,
        });
        self.by_name.insert("void".to_string(), self.void_type);

        // ... register all primitive types
    }

    pub fn register(&mut self, def: TypeDef) -> TypeId {
        let id = TypeId(self.types.len() as u32);
        self.types.push(def);
        id
    }

    pub fn lookup(&self, name: &str) -> Option<TypeId> {
        self.by_name.get(name).copied()
    }

    pub fn get(&self, id: TypeId) -> &TypeDef {
        &self.types[id.0 as usize]
    }
}
```

#### Type Resolver

```rust
/// Resolves TypeExpr AST nodes to TypeId
pub struct TypeResolver<'ast> {
    registry: TypeRegistry,
    type_map: HashMap<NodeId, TypeId>,
    resolution_data: &'ast ResolutionData,  // From Pass 1
    errors: Vec<SemanticError>,
}

impl<'ast> TypeResolver<'ast> {
    pub fn resolve(
        ast: &Script,
        resolution: &ResolutionData,
    ) -> TypeResolutionData {
        let mut resolver = Self::new(resolution);

        // Two-phase: collect types, then resolve references
        resolver.collect_types(ast);
        resolver.resolve_type_references(ast);

        resolver.into_result()
    }

    fn collect_types(&mut self, ast: &Script) {
        // Register all user-defined types
        for decl in ast.declarations {
            match decl {
                Declaration::Class(class) => {
                    let type_id = self.registry.register(TypeDef::Class {
                        name: class.name.to_string(),
                        fields: Vec::new(),  // Fill in later
                        methods: Vec::new(),
                        base_class: None,
                        interfaces: Vec::new(),
                    });
                    self.registry.by_name.insert(class.name.to_string(), type_id);
                }

                Declaration::Enum(enum_decl) => {
                    // Register enum
                }

                // ... other type declarations
            }
        }
    }

    fn resolve_type_references(&mut self, ast: &Script) {
        // Now resolve all TypeExpr references
        for decl in ast.declarations {
            self.visit_declaration(decl);
        }
    }

    fn resolve_type_expr(&mut self, type_expr: &TypeExpr) -> Result<TypeId, SemanticError> {
        // Resolve base type name
        let base_name = type_expr.base.name();

        let mut type_id = if let Some(id) = self.registry.lookup(base_name) {
            id
        } else {
            return Err(SemanticError::undefined_type(base_name, type_expr.span));
        };

        // Handle template arguments
        if !type_expr.template_args.is_empty() {
            let arg_types: Vec<TypeId> = type_expr.template_args
                .iter()
                .map(|arg| self.resolve_type_expr(arg))
                .collect::<Result<_, _>>()?;

            // Instantiate template
            type_id = self.instantiate_template(type_id, arg_types)?;
        }

        // Handle suffixes (arrays, handles)
        for suffix in type_expr.suffixes {
            match suffix {
                TypeSuffix::Array => {
                    type_id = self.make_array_type(type_id);
                }
                TypeSuffix::Handle { .. } => {
                    // Handles don't create new types, just modify semantics
                }
            }
        }

        Ok(type_id)
    }

    fn instantiate_template(&mut self, template_id: TypeId, args: Vec<TypeId>) -> Result<TypeId, SemanticError> {
        // Check if this instantiation already exists
        let cache_key = (template_id, args.clone());
        if let Some(existing) = self.template_cache.get(&cache_key) {
            return Ok(*existing);
        }

        // Create new template instance
        let instance = TypeDef::TemplateInstance {
            template: template_id,
            args: args.clone(),
        };

        let instance_id = self.registry.register(instance);
        self.template_cache.insert(cache_key, instance_id);

        Ok(instance_id)
    }
}
```

### Algorithm (Single O(n) Traversal + Pre-processing)

1. **Pre-populate**: Register built-in primitive types
2. **First sub-pass**: Collect all user-defined type names (classes, enums, interfaces)
3. **Second sub-pass**: Resolve all TypeExpr references in declarations
4. **Handle templates**: Instantiate as needed, cache instances
5. **Build hierarchy**: Record inheritance and interface relationships

### Implementation Tasks

1. Define `TypeId`, `TypeDef`, `TypeRegistry` types
2. Implement primitive type registration
3. Implement type collection sub-pass
4. Implement type resolution for simple types
5. Add scoped type resolution (Namespace::Type)
6. Implement template instantiation with caching
7. Add array type creation
8. Build inheritance hierarchy
9. Track interface implementations
10. Write comprehensive tests

### Test Coverage

- Primitive type resolution
- User-defined class/enum types
- Scoped types (Namespace::Type)
- Template instantiation (array<int>, dict<string, T>)
- Nested templates (array<dict<string, int>>)
- Inheritance relationships
- Interface implementation
- Undefined type errors
- Invalid template arguments

**Estimated:** 40-50 tests

### Files to Create

- `src/semantic/type_def.rs` (~300-400 lines)
- `src/semantic/type_registry.rs` (~200-300 lines)
- `src/semantic/type_resolver.rs` (~400-500 lines)

**Total:** ~900-1200 lines

---

## Pass 3: Type Checking & Validation

### Goals

- Type check every expression in the AST
- Validate operators are applied to compatible types
- Verify function calls match signatures
- Check assignments are type-compatible
- Validate control flow (break/continue/return)
- Check class contracts (inheritance, interfaces, overrides)
- Verify access modifiers
- Annotate AST with resolved type information

### Input/Output

**Input:**
- `Script<'src, 'ast>` (AST)
- `ResolutionData` (from Pass 1)
- `TypeResolutionData` (from Pass 2)

**Output:**
```rust
pub struct TypeCheckData {
    /// Type of each expression node
    pub expr_types: HashMap<NodeId, TypeId>,

    /// Validation results
    pub validated: bool,

    /// All errors found
    pub errors: Vec<SemanticError>,
}

/// Final combined result
pub struct AnalyzedScript<'src, 'ast> {
    pub ast: Script<'src, 'ast>,
    pub resolution: ResolutionData,
    pub type_resolution: TypeResolutionData,
    pub type_check: TypeCheckData,
}
```

### Data Structures

```rust
/// Type checking traversal state
pub struct TypeChecker<'ast> {
    type_registry: &'ast TypeRegistry,
    resolution: &'ast ResolutionData,
    type_resolution: &'ast TypeResolutionData,

    /// Results: expression types
    expr_types: HashMap<NodeId, TypeId>,

    /// Context tracking
    current_function_return: Option<TypeId>,
    in_loop: bool,

    /// Errors
    errors: Vec<SemanticError>,
}
```

### Algorithm (Single O(n) Traversal)

```rust
impl<'ast> TypeChecker<'ast> {
    pub fn check(
        ast: &Script,
        resolution: &ResolutionData,
        type_resolution: &TypeResolutionData,
    ) -> TypeCheckData {
        let mut checker = Self::new(resolution, type_resolution);
        checker.visit_script(ast);
        checker.into_result()
    }

    /// Type check an expression, return its type
    fn check_expr(&mut self, expr: &Expression) -> TypeId {
        let type_id = match expr {
            Expression::Literal(lit) => self.check_literal(lit),
            Expression::Identifier(ident) => self.check_identifier(ident),
            Expression::Binary(binary) => self.check_binary(binary),
            Expression::Unary(unary) => self.check_unary(unary),
            Expression::Call(call) => self.check_call(call),
            Expression::MemberAccess(access) => self.check_member_access(access),
            Expression::Index(index) => self.check_index(index),
            Expression::Cast(cast) => self.check_cast(cast),
            // ... other expressions
        };

        // Record type for this expression
        self.expr_types.insert(expr.node_id(), type_id);
        type_id
    }

    fn check_literal(&self, lit: &Literal) -> TypeId {
        match lit {
            Literal::Int(_) => self.type_registry.int_type,
            Literal::Float(_) => self.type_registry.float_type,
            Literal::Bool(_) => self.type_registry.bool_type,
            Literal::String(_) => self.type_registry.string_type,
            Literal::Null => self.type_registry.null_type,
        }
    }

    fn check_identifier(&self, ident: &Identifier) -> TypeId {
        // Look up resolved symbol from Pass 1
        let symbol_id = self.resolution.resolutions[&ident.node_id()];
        let symbol = &self.resolution.symbols[symbol_id];

        // Get type from Pass 2
        if let Some(type_expr) = &symbol.declared_type {
            self.type_resolution.type_map[&type_expr.node_id()]
        } else {
            self.error(SemanticErrorKind::MissingType);
            self.type_registry.error_type
        }
    }

    fn check_binary(&mut self, binary: &BinaryExpr) -> TypeId {
        let left_type = self.check_expr(binary.left);
        let right_type = self.check_expr(binary.right);

        // Get result type from operator rules
        match self.check_binary_op(binary.op, left_type, right_type) {
            Ok(result_type) => result_type,
            Err(e) => {
                self.errors.push(e);
                self.type_registry.error_type
            }
        }
    }

    fn check_binary_op(&self, op: BinaryOp, left: TypeId, right: TypeId) -> Result<TypeId, SemanticError> {
        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                // Numeric operators
                if self.is_numeric(left) && self.is_numeric(right) {
                    // Promotion rules: if either is float/double, result is float/double
                    if left == self.type_registry.double_type || right == self.type_registry.double_type {
                        Ok(self.type_registry.double_type)
                    } else if left == self.type_registry.float_type || right == self.type_registry.float_type {
                        Ok(self.type_registry.float_type)
                    } else {
                        Ok(self.type_registry.int_type)
                    }
                } else {
                    Err(SemanticError::type_mismatch("numeric types", left, right))
                }
            }

            BinaryOp::Less | BinaryOp::Greater | BinaryOp::LessEq | BinaryOp::GreaterEq => {
                // Comparison operators
                if self.is_numeric(left) && self.is_numeric(right) {
                    Ok(self.type_registry.bool_type)
                } else {
                    Err(SemanticError::type_mismatch("comparable types", left, right))
                }
            }

            BinaryOp::Equal | BinaryOp::NotEqual => {
                // Equality works on any type
                Ok(self.type_registry.bool_type)
            }

            BinaryOp::And | BinaryOp::Or => {
                // Logical operators require bool
                if left == self.type_registry.bool_type && right == self.type_registry.bool_type {
                    Ok(self.type_registry.bool_type)
                } else {
                    Err(SemanticError::type_mismatch("bool", left, right))
                }
            }

            // ... other operators
        }
    }

    fn check_call(&mut self, call: &CallExpr) -> TypeId {
        // Check callee
        let callee_type = self.check_expr(call.callee);

        // Get function signature
        let func_def = match self.type_registry.get(callee_type) {
            TypeDef::Funcdef { params, return_type, .. } => (params, return_type),
            _ => {
                self.error(SemanticErrorKind::NotCallable);
                return self.type_registry.error_type;
            }
        };

        // Check argument count
        if call.arguments.len() != func_def.0.len() {
            self.error(SemanticErrorKind::ArgumentCountMismatch);
            return func_def.1;
        }

        // Check argument types
        for (arg, param_type) in call.arguments.iter().zip(func_def.0) {
            let arg_type = self.check_expr(arg);
            if !self.is_assignable(arg_type, *param_type) {
                self.error(SemanticErrorKind::ArgumentTypeMismatch);
            }
        }

        func_def.1
    }

    fn check_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Variable(var) => {
                // Check initializer type matches declared type
                if let Some(init) = var.initializer {
                    let init_type = self.check_expr(init);
                    let declared_type = self.type_resolution.type_map[&var.type_expr.node_id()];

                    if !self.is_assignable(init_type, declared_type) {
                        self.error(SemanticErrorKind::TypeMismatch);
                    }
                }
            }

            Statement::Expression(expr) => {
                self.check_expr(expr);
            }

            Statement::If(if_stmt) => {
                // Condition must be bool
                let cond_type = self.check_expr(if_stmt.condition);
                if cond_type != self.type_registry.bool_type {
                    self.error(SemanticErrorKind::ConditionNotBool);
                }

                self.check_statement(if_stmt.then_branch);
                if let Some(else_branch) = if_stmt.else_branch {
                    self.check_statement(else_branch);
                }
            }

            Statement::While(while_stmt) => {
                let cond_type = self.check_expr(while_stmt.condition);
                if cond_type != self.type_registry.bool_type {
                    self.error(SemanticErrorKind::ConditionNotBool);
                }

                let prev_in_loop = self.in_loop;
                self.in_loop = true;
                self.check_statement(while_stmt.body);
                self.in_loop = prev_in_loop;
            }

            Statement::Return(ret) => {
                // Check return type matches function
                let return_type = if let Some(expr) = ret.value {
                    self.check_expr(expr)
                } else {
                    self.type_registry.void_type
                };

                if let Some(expected) = self.current_function_return {
                    if !self.is_assignable(return_type, expected) {
                        self.error(SemanticErrorKind::ReturnTypeMismatch);
                    }
                } else {
                    self.error(SemanticErrorKind::ReturnOutsideFunction);
                }
            }

            Statement::Break | Statement::Continue => {
                if !self.in_loop {
                    self.error(SemanticErrorKind::BreakContinueOutsideLoop);
                }
            }

            // ... other statements
        }
    }

    /// Check if `from` can be assigned to `to`
    fn is_assignable(&self, from: TypeId, to: TypeId) -> bool {
        // Exact match
        if from == to {
            return true;
        }

        // Implicit conversions
        // int → float, int → double
        if to == self.type_registry.float_type || to == self.type_registry.double_type {
            if self.is_integer(from) {
                return true;
            }
        }

        // Derived class → base class
        if self.is_derived_from(from, to) {
            return true;
        }

        false
    }
}
```

### Semantic Validations

1. **Type compatibility** - Assignments, operators, function calls
2. **Control flow** - Break/continue in loops, return in functions
3. **Class inheritance** - No circular inheritance, proper overrides
4. **Interface implementation** - All methods implemented with correct signatures
5. **Access modifiers** - Private/protected/public respected
6. **Const correctness** - Const objects not modified
7. **Null safety** - Handles checked before dereference

### Implementation Tasks

1. Implement `TypeChecker` with context tracking
2. Add expression type checking (all expression types)
3. Define operator type rules
4. Implement function call validation
5. Add statement checking (assignments, control flow)
6. Implement class validation (inheritance, interfaces)
7. Add access control checking
8. Implement method override validation
9. Handle implicit conversions
10. Write comprehensive tests

### Test Coverage

- Literal type inference
- Binary operator type checking (all combinations)
- Unary operator type checking
- Function call argument matching
- Type mismatch errors
- Implicit conversions (int → float)
- Class inheritance type compatibility
- Interface implementation validation
- Method override signature matching
- Access control violations
- Control flow validation (break/continue/return)
- Const correctness

**Estimated:** 80-100 tests

### Files to Create

- `src/semantic/type_checker.rs` (~600-800 lines)
- `src/semantic/validation.rs` (~300-400 lines)
- `src/semantic/type_compat.rs` (~200-300 lines)

**Total:** ~1100-1500 lines

---

## Integration & Public API

### Coordinator Module

```rust
// src/semantic/mod.rs

pub use self::error::{SemanticError, SemanticErrorKind};
pub use self::resolver::ResolutionData;
pub use self::type_registry::{TypeRegistry, TypeId, TypeDef};
pub use self::type_checker::TypeCheckData;

mod resolver;
mod scope;
mod symbol_table;
mod type_def;
mod type_registry;
mod type_resolver;
mod type_checker;
mod validation;
mod type_compat;
mod error;

/// Analyzed script ready for code generation
pub struct AnalyzedScript<'src, 'ast> {
    pub ast: Script<'src, 'ast>,
    pub resolution: ResolutionData,
    pub type_resolution: TypeResolutionData,
    pub type_check: TypeCheckData,
}

impl<'src, 'ast> AnalyzedScript<'src, 'ast> {
    /// Check if analysis succeeded (no errors)
    pub fn is_valid(&self) -> bool {
        self.resolution.errors.is_empty()
            && self.type_resolution.errors.is_empty()
            && self.type_check.errors.is_empty()
    }

    /// Get all errors from all passes
    pub fn all_errors(&self) -> Vec<&SemanticError> {
        self.resolution.errors.iter()
            .chain(&self.type_resolution.errors)
            .chain(&self.type_check.errors)
            .collect()
    }
}

/// Perform full semantic analysis
pub fn analyze<'src, 'ast>(
    ast: Script<'src, 'ast>
) -> AnalyzedScript<'src, 'ast> {
    // Pass 1: Resolution & Symbol Collection
    let resolution = resolver::resolve(&ast);

    // Pass 2: Type Resolution
    let type_resolution = type_resolver::resolve(&ast, &resolution);

    // Pass 3: Type Checking & Validation
    let type_check = type_checker::check(&ast, &resolution, &type_resolution);

    AnalyzedScript {
        ast,
        resolution,
        type_resolution,
        type_check,
    }
}
```

### Public API (src/lib.rs)

```rust
// Update public API
pub use semantic::{
    analyze,
    AnalyzedScript,
    SemanticError,
    SemanticErrorKind,
    TypeId,
    TypeDef,
    TypeRegistry,
};

/// Convenience: parse and analyze in one call
pub fn parse_and_analyze(source: &str) -> Result<AnalyzedScript, Vec<Error>> {
    // Parse
    let (ast, parse_errors) = parse_lenient(source);
    if !parse_errors.is_empty() {
        return Err(parse_errors.into_iter().map(Error::Parse).collect());
    }

    // Analyze
    let analyzed = analyze(ast);
    if !analyzed.is_valid() {
        return Err(analyzed.all_errors().into_iter()
            .map(|e| Error::Semantic(e.clone()))
            .collect());
    }

    Ok(analyzed)
}

pub enum Error {
    Parse(ParseError),
    Semantic(SemanticError),
}
```

---

## File Structure

```
src/semantic/
├── mod.rs                    # Public API, coordinator (~150-200 lines)
│
├── error.rs                  # Error types (~150-200 lines)
│
├── resolver.rs               # Pass 1: Resolution (~400-500 lines)
├── scope.rs                  # Scope management (~150-200 lines)
├── symbol_table.rs           # Symbol storage (~200-250 lines)
│
├── type_def.rs               # Type definitions (~300-400 lines)
├── type_registry.rs          # Type storage (~200-300 lines)
├── type_resolver.rs          # Pass 2: Type resolution (~400-500 lines)
│
├── type_checker.rs           # Pass 3: Type checking (~600-800 lines)
├── validation.rs             # Semantic validation (~300-400 lines)
└── type_compat.rs            # Type compatibility (~200-300 lines)

docs/
└── semantic_analysis.md      # Documentation (~100-150 lines)

tests/
└── semantic_tests.rs         # Integration tests (~1000-1500 lines)
```

**Total Estimates:**
- Code: ~3,050-4,150 lines
- Tests: ~1,150-1,640 individual tests (150-190 test functions)
- Documentation: ~100-150 lines

---

## Testing Strategy

### Unit Tests (per module)

Each module has focused tests:

```rust
// resolver.rs tests
#[cfg(test)]
mod tests {
    #[test] fn resolve_local_variable() { }
    #[test] fn resolve_forward_reference() { }
    #[test] fn error_duplicate_declaration() { }
    #[test] fn error_undefined_variable() { }
    #[test] fn scope_shadowing() { }
    // ... ~30-40 tests
}

// type_resolver.rs tests
#[cfg(test)]
mod tests {
    #[test] fn resolve_primitive_type() { }
    #[test] fn resolve_class_type() { }
    #[test] fn resolve_scoped_type() { }
    #[test] fn instantiate_template() { }
    #[test] fn error_undefined_type() { }
    // ... ~40-50 tests
}

// type_checker.rs tests
#[cfg(test)]
mod tests {
    #[test] fn check_binary_add_int() { }
    #[test] fn check_function_call() { }
    #[test] fn error_type_mismatch() { }
    #[test] fn implicit_int_to_float() { }
    // ... ~80-100 tests
}
```

### Integration Tests (tests/semantic_tests.rs)

Full pipeline tests:

```rust
#[test]
fn test_full_analysis_valid_script() {
    let source = r#"
        class Player {
            int health;
            void takeDamage(int amount) {
                health -= amount;
            }
        }

        void main() {
            Player p;
            p.takeDamage(10);
        }
    "#;

    let analyzed = parse_and_analyze(source).unwrap();
    assert!(analyzed.is_valid());
}

#[test]
fn test_type_error_detected() {
    let source = r#"
        void main() {
            int x = "hello";  // Type error
        }
    "#;

    let result = parse_and_analyze(source);
    assert!(result.is_err());

    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| matches!(e, Error::Semantic(_))));
}

// ... ~40-50 integration tests
```

### Error Message Quality Tests

```rust
#[test]
fn test_error_message_undefined_variable() {
    let source = "void main() { x = 5; }";
    let errors = parse_and_analyze(source).unwrap_err();

    let msg = errors[0].to_string();
    assert!(msg.contains("undefined"));
    assert!(msg.contains("'x'"));
}
```

---

## Implementation Timeline

### Phase 1: Foundation (Week 1) - 5-7 days
- [ ] Create `src/semantic/` module structure
- [ ] Implement error types
- [ ] Implement scope management
- [ ] Implement symbol table
- [ ] Write foundation tests
- **Deliverable:** Basic infrastructure ready

### Phase 2: Pass 1 - Resolution (Week 2) - 5-7 days
- [ ] Implement resolver visitor
- [ ] Add declare/define logic
- [ ] Handle all declaration types
- [ ] Implement name lookup
- [ ] Write resolution tests
- **Deliverable:** Symbol collection working

### Phase 3: Pass 2 - Type Resolution (Week 3) - 5-7 days
- [ ] Implement type registry
- [ ] Add primitive type registration
- [ ] Implement type resolver
- [ ] Handle templates and scoped types
- [ ] Write type resolution tests
- **Deliverable:** Type system working

### Phase 4: Pass 3 - Type Checking (Week 4-5) - 8-10 days
- [ ] Implement expression type checker
- [ ] Add operator type rules
- [ ] Implement statement validation
- [ ] Add class/interface validation
- [ ] Write type checking tests
- **Deliverable:** Full type checking working

### Phase 5: Integration & Polish (Week 5-6) - 4-5 days
- [ ] Implement coordinator
- [ ] Update public API
- [ ] Write integration tests
- [ ] Write documentation
- [ ] Performance testing
- **Deliverable:** Production-ready semantic analysis

**Total Estimated Time:** 27-36 days (5.5-7 weeks)

---

## Success Criteria

### Feature Completeness

- [x] ✅ Lexer (complete)
- [x] ✅ Parser (complete)
- [ ] Symbol collection for all declaration types
- [ ] Name resolution with forward references
- [ ] Type resolution with templates
- [ ] Type checking for all expressions
- [ ] Semantic validation (control flow, classes, etc.)
- [ ] Error messages with source location
- [ ] Public API for analysis

### Test Coverage

- [ ] 30-40 resolution tests
- [ ] 40-50 type resolution tests
- [ ] 80-100 type checking tests
- [ ] 40-50 integration tests
- [ ] All tests passing
- [ ] ~150-190 total test functions

### Documentation

- [ ] Module documentation (rustdoc)
- [ ] `docs/semantic_analysis.md` written
- [ ] Public API documented with examples
- [ ] Design decisions logged

### Quality Metrics

- [ ] No compiler warnings
- [ ] All clippy lints passing
- [ ] Clear error messages with spans
- [ ] O(n) performance per pass
- [ ] Memory efficient (side tables, not AST modification)

---

## Performance Constraints & Benchmarks

### Current Parser Performance Baseline

From existing benchmarks (`benches/parser_benchmarks.rs`):

| File Size | Lines | Time | Throughput |
|-----------|-------|------|------------|
| Tiny | 5 lines | ~10-20 µs | - |
| Small | 60 lines | ~100-150 µs | - |
| Medium | 130 lines | ~200-300 µs | - |
| Large | 266 lines | ~400-600 µs | - |
| XLarge | 500 lines | ~600-900 µs | - |
| XXLarge | 1000 lines | ~1.0-1.3 ms | - |
| **Stress** | **5000 lines** | **~1.0 ms** | **~5M lines/sec** |

**Key insight:** Parser achieves **sub-1ms for 5000 lines** - this is our performance target to match.

### Semantic Analysis Performance Targets

Since we have 3 passes, each should aim for similar or better performance than parsing:

#### **Target: 3-pass analysis completes in < 3ms for 5000 lines**

**Per-pass budget:**
- Pass 1 (Resolution): < 1.0 ms for 5000 lines
- Pass 2 (Type Resolution): < 1.0 ms for 5000 lines
- Pass 3 (Type Checking): < 1.0 ms for 5000 lines
- **Total: < 3.0 ms for full analysis**

#### **Stretch Goal: < 2ms total (match or beat parser)**

**Reasoning:**
- Parser does complex work: lexing + precedence climbing + error recovery
- Semantic analysis is mostly table lookups and tree traversal
- With proper data structures, should be faster than parsing
- Each pass is O(n) - linear in AST size

### Performance Design Principles

#### 1. **Use Efficient Data Structures**

```rust
// ✅ Good: Direct indexing
symbols: Vec<Symbol>,           // TypeId/SymbolId are indices
type_registry: Vec<TypeDef>,    // O(1) lookup

// ✅ Good: FxHashMap for small keys
use rustc_hash::FxHashMap;
by_name: FxHashMap<String, TypeId>,  // Faster than std HashMap

// ❌ Avoid: Linear searches
symbols: Vec<Symbol>,
symbols.iter().find(|s| s.name == name)  // O(n) - too slow
```

#### 2. **Minimize Allocations**

```rust
// ✅ Good: Pre-allocate based on AST size
let estimated_symbols = ast.declarations.len() * 4;  // Heuristic
let mut symbols = Vec::with_capacity(estimated_symbols);

// ✅ Good: Reuse allocated capacity
scope_stack.clear();  // Don't reallocate

// ❌ Avoid: Repeated small allocations
for item in items {
    let mut temp = Vec::new();  // Allocates every iteration
    // ...
}
```

#### 3. **Cache Lookups**

```rust
// ✅ Good: Cache resolved types
type_map: HashMap<NodeId, TypeId>,  // Store once, reuse

// ✅ Good: Cache template instantiations
template_cache: HashMap<(TypeId, Vec<TypeId>), TypeId>,

// ❌ Avoid: Re-resolving same type multiple times
fn check_expr(&mut self, expr: &Expr) -> TypeId {
    self.resolve_type(expr.type_expr)  // If called repeatedly, cache it
}
```

#### 4. **Optimize Hot Paths**

```rust
// ✅ Add inline hints for frequently called functions
#[inline]
fn lookup_symbol(&self, name: &str) -> Option<SymbolId> { }

#[inline]
fn is_assignable(&self, from: TypeId, to: TypeId) -> bool { }

// ✅ Use early returns to avoid unnecessary work
fn check_binary_op(&self, op: BinaryOp, left: TypeId, right: TypeId) -> Result<TypeId> {
    // Fast path: same types
    if left == right {
        return Ok(left);
    }

    // Slow path: compatibility checks
    // ...
}
```

#### 5. **Avoid String Operations in Hot Paths**

```rust
// ✅ Good: Use string references
symbols: HashMap<&'src str, SymbolId>,  // No allocations

// ✅ Good: Intern strings if needed
string_interner: StringInterner,

// ❌ Avoid: String cloning in lookups
symbols: HashMap<String, SymbolId>,
if symbols.contains_key(&name.to_string()) { }  // Allocates!
```

### Benchmark Suite for Semantic Analysis

Add new benchmarks to `benches/semantic_benchmarks.rs`:

```rust
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use angelscript::{parse_lenient, analyze};

fn semantic_analysis_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("semantic_analysis");

    // Tiny: 5 lines - baseline overhead
    let hello_world = include_str!("../test_scripts/hello_world.as");
    group.bench_function("tiny_5_lines", |b| {
        let (ast, _) = parse_lenient(hello_world);
        b.iter(|| analyze(&ast));
    });

    // Small: 60 lines
    let functions = include_str!("../test_scripts/functions.as");
    group.bench_function("small_60_lines", |b| {
        let (ast, _) = parse_lenient(functions);
        b.iter(|| analyze(&ast));
    });

    // Stress: 5000 lines - MUST BE < 3ms
    let stress = include_str!("../test_scripts/performance/xxlarge_5000.as");
    group.bench_function("stress_5000_lines", |b| {
        let (ast, _) = parse_lenient(stress);
        b.iter(|| analyze(&ast));
    });

    group.finish();
}

// Per-pass benchmarks
fn pass_breakdown_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("pass_breakdown");

    let stress = include_str!("../test_scripts/performance/xxlarge_5000.as");
    let (ast, _) = parse_lenient(stress);

    group.bench_function("pass1_resolution", |b| {
        b.iter(|| resolver::resolve(&ast));
    });

    group.bench_function("pass2_type_resolution", |b| {
        let resolution = resolver::resolve(&ast);
        b.iter(|| type_resolver::resolve(&ast, &resolution));
    });

    group.bench_function("pass3_type_checking", |b| {
        let resolution = resolver::resolve(&ast);
        let type_resolution = type_resolver::resolve(&ast, &resolution);
        b.iter(|| type_checker::check(&ast, &resolution, &type_resolution));
    });

    group.finish();
}

criterion_group!(benches, semantic_analysis_benchmarks, pass_breakdown_benchmarks);
criterion_main!(benches);
```

### Performance Testing Strategy

1. **Benchmark after each module**
   - Add benchmarks as you implement each pass
   - Catch performance regressions early

2. **Profile hot spots**
   ```bash
   # Build with debug info for profiling
   RUSTFLAGS='-C force-frame-pointers=yes' cargo bench --bench semantic_benchmarks

   # Profile with samply or instruments
   cargo bench --bench semantic_benchmarks -- --profile-time=10
   ```

3. **Memory profiling**
   ```bash
   # Check for excessive allocations
   cargo bench --bench semantic_benchmarks -- --profile-time=10 --print-allocations
   ```

4. **Regression testing**
   - Run benchmarks before/after changes
   - Ensure no pass exceeds 1ms budget
   - Track total analysis time stays < 3ms

### Optimization Checklist

Before declaring a pass "complete", verify:

- [ ] Uses `FxHashMap` for small-key maps (not `std::HashMap`)
- [ ] Pre-allocates collections based on AST size estimates
- [ ] Uses string slices (`&str`) not owned strings where possible
- [ ] Hot functions marked with `#[inline]`
- [ ] No unnecessary allocations in inner loops
- [ ] Lookups are O(1) with proper indexing
- [ ] Template instantiation is cached
- [ ] Scope stack reuses allocated capacity
- [ ] Benchmarks show < 1ms per pass for 5000 lines

### Expected Performance Profile

**Well-optimized semantic analysis should be FASTER than parsing:**

| Phase | 5000 lines | Reasoning |
|-------|-----------|-----------|
| Lexer + Parser | ~1.0 ms | Character scanning, token creation, tree building |
| Pass 1: Resolution | ~0.5 ms | Simple tree walk, hash table inserts |
| Pass 2: Type Resolution | ~0.4 ms | Type lookups, template instantiation (cached) |
| Pass 3: Type Checking | ~0.6 ms | Tree walk, type compatibility checks |
| **Total Analysis** | **~1.5 ms** | Should beat parser with good implementation |

**Why semantic analysis should be faster:**
1. No character-level processing (parser already did that)
2. Mostly hash table lookups and integer comparisons
3. Tree already built - just traversing
4. Type IDs are u32 indices - very fast comparisons
5. Modern CPUs love simple loops over contiguous data

### Failure Criteria

If any pass exceeds these thresholds, investigate and optimize:

- ⚠️ Warning: Pass takes > 1.0 ms for 5000 lines
- 🚨 Critical: Pass takes > 2.0 ms for 5000 lines
- 🚨 Critical: Total analysis > 5.0 ms for 5000 lines

### Performance Success Criteria

- ✅ Pass 1 completes in < 1.0 ms for 5000 lines
- ✅ Pass 2 completes in < 1.0 ms for 5000 lines
- ✅ Pass 3 completes in < 1.0 ms for 5000 lines
- ✅ **Total analysis completes in < 3.0 ms for 5000 lines**
- 🎯 **Stretch: Total analysis < 2.0 ms (faster than parser)**

---

## Next Steps After Semantic Analysis

Once semantic analysis is complete, you'll have a validated AST ready for:

1. **Bytecode Generation** - Compile to VM instructions
2. **Tree-Walk Interpreter** - Direct execution of validated AST
3. **Optimization Passes** - Dead code elimination, constant folding
4. **JIT Compilation** - Convert to native code

The semantic analysis phase is the foundation for all execution strategies.

---

## References

- [Crafting Interpreters - Resolving and Binding](https://craftinginterpreters.com/resolving-and-binding.html)
- [Crafting Interpreters - A Map of the Territory](https://craftinginterpreters.com/a-map-of-the-territory.html)
- [AngelScript Documentation](https://www.angelcode.com/angelscript/sdk/docs/manual/)
- Project docs: `docs/architecture.md`, `docs/parser.md`

---

**Ready to begin implementation!**
