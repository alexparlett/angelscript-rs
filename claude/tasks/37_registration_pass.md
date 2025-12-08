# Task 37: Registration Pass (Pass 1)

## Overview

Implement Pass 1 of the two-pass compiler: walk the AST and register all type and function declarations. This pass collects signatures without compiling function bodies.

## Goals

1. Register all class/interface/enum declarations
2. Register all function signatures (global and methods)
3. Handle namespace declarations
4. Resolve base classes and interfaces
5. Auto-generate default constructors/destructors where needed

## Dependencies

- Task 32: Compilation Context
- Task 33: Type Resolution

## Files to Create

```
crates/angelscript-compiler/src/
├── passes/
│   ├── mod.rs
│   └── registration.rs    # Pass 1 implementation
└── lib.rs
```

## Detailed Implementation

### 1. Registration Pass (passes/registration.rs)

```rust
use angelscript_core::{DataType, Span, TypeHash, UnitId, Visibility};
use angelscript_parser::ast::{
    ClassDecl, EnumDecl, FunctionDecl, InterfaceDecl, NamespaceDecl,
    Script, TypeExpr, VarDecl,
};

use crate::context::CompilationContext;
use crate::error::{CompileError, Result};
use crate::script_defs::{ScriptFunctionDef, ScriptParam, ScriptTypeDef, ScriptTypeKind};
use crate::type_resolver::TypeResolver;

/// Output of the registration pass.
pub struct RegistrationOutput {
    /// Number of types registered
    pub types_registered: usize,
    /// Number of functions registered
    pub functions_registered: usize,
    /// Collected errors (compilation can continue with some errors)
    pub errors: Vec<CompileError>,
}

/// Pass 1: Register all types and function signatures.
pub struct RegistrationPass<'a, 'reg, 'ast> {
    ctx: &'a mut CompilationContext<'reg>,
    ast: &'ast Script<'ast>,
    resolver: TypeResolver<'a, 'reg>,
    types_registered: usize,
    functions_registered: usize,
}

impl<'a, 'reg, 'ast> RegistrationPass<'a, 'reg, 'ast> {
    pub fn new(ctx: &'a mut CompilationContext<'reg>, ast: &'ast Script<'ast>) -> Self {
        let resolver = TypeResolver::new(ctx);
        Self {
            ctx,
            ast,
            resolver,
            types_registered: 0,
            functions_registered: 0,
        }
    }

    /// Run the registration pass.
    pub fn run(mut self) -> RegistrationOutput {
        // Process all top-level declarations
        for decl in &self.ast.declarations {
            self.visit_declaration(decl);
        }

        // Auto-generate missing default members
        self.generate_defaults();

        RegistrationOutput {
            types_registered: self.types_registered,
            functions_registered: self.functions_registered,
            errors: self.ctx.take_errors(),
        }
    }

    fn visit_declaration(&mut self, decl: &'ast Declaration<'ast>) {
        match decl {
            Declaration::Namespace(ns) => self.visit_namespace(ns),
            Declaration::Class(class) => self.visit_class(class),
            Declaration::Interface(iface) => self.visit_interface(iface),
            Declaration::Enum(e) => self.visit_enum(e),
            Declaration::Function(func) => self.visit_function(func, None),
            Declaration::GlobalVar(var) => self.visit_global_var(var),
            Declaration::Typedef(td) => self.visit_typedef(td),
            Declaration::Funcdef(fd) => self.visit_funcdef(fd),
            Declaration::Import(_) => { /* Handle imports */ }
            Declaration::Using(u) => self.visit_using(u),
        }
    }

    // ==========================================================================
    // Namespace
    // ==========================================================================

    fn visit_namespace(&mut self, ns: &'ast NamespaceDecl<'ast>) {
        self.ctx.enter_namespace(&ns.name);

        for decl in &ns.declarations {
            self.visit_declaration(decl);
        }

        self.ctx.exit_namespace();
    }

    fn visit_using(&mut self, u: &'ast UsingDecl) {
        let ns: Vec<String> = u.path.iter().map(|s| s.to_string()).collect();
        self.ctx.add_import(ns);
    }

    // ==========================================================================
    // Class
    // ==========================================================================

    fn visit_class(&mut self, class: &'ast ClassDecl<'ast>) {
        let name = class.name.to_string();
        let qualified_name = self.qualified_name(&name);
        let type_hash = TypeHash::from_name(&qualified_name);

        // Resolve base class
        let base_class = class.base.as_ref().and_then(|base_expr| {
            match self.resolver.resolve(base_expr, class.span) {
                Ok(dt) => Some(dt.type_hash),
                Err(e) => {
                    self.ctx.error(e);
                    None
                }
            }
        });

        // Resolve interfaces
        let interfaces: Vec<TypeHash> = class.interfaces
            .iter()
            .filter_map(|iface_expr| {
                match self.resolver.resolve(iface_expr, class.span) {
                    Ok(dt) => Some(dt.type_hash),
                    Err(e) => {
                        self.ctx.error(e);
                        None
                    }
                }
            })
            .collect();

        // Create type definition
        let def = ScriptTypeDef {
            name: name.clone(),
            qualified_name: qualified_name.clone(),
            type_hash,
            kind: ScriptTypeKind::Class {
                base_class,
                interfaces,
                is_final: class.is_final,
                is_abstract: class.is_abstract,
            },
            span: class.span,
        };

        self.ctx.register_script_type(def);
        self.types_registered += 1;

        // Register class members
        for member in &class.members {
            match member {
                ClassMember::Method(method) => {
                    self.visit_function(method, Some(type_hash));
                }
                ClassMember::Property(prop) => {
                    self.visit_property(prop, type_hash);
                }
                ClassMember::Constructor(ctor) => {
                    self.visit_constructor(ctor, type_hash);
                }
                ClassMember::Destructor(dtor) => {
                    self.visit_destructor(dtor, type_hash);
                }
            }
        }
    }

    // ==========================================================================
    // Interface
    // ==========================================================================

    fn visit_interface(&mut self, iface: &'ast InterfaceDecl<'ast>) {
        let name = iface.name.to_string();
        let qualified_name = self.qualified_name(&name);
        let type_hash = TypeHash::from_name(&qualified_name);

        let methods: Vec<TypeHash> = iface.methods
            .iter()
            .map(|method| {
                let func_hash = self.register_interface_method(method, type_hash);
                func_hash
            })
            .collect();

        let def = ScriptTypeDef {
            name,
            qualified_name,
            type_hash,
            kind: ScriptTypeKind::Interface { methods },
            span: iface.span,
        };

        self.ctx.register_script_type(def);
        self.types_registered += 1;
    }

    fn register_interface_method(&mut self, method: &'ast FunctionDecl<'ast>, iface_hash: TypeHash) -> TypeHash {
        // Interface methods have no body
        self.visit_function(method, Some(iface_hash))
    }

    // ==========================================================================
    // Enum
    // ==========================================================================

    fn visit_enum(&mut self, e: &'ast EnumDecl<'ast>) {
        let name = e.name.to_string();
        let qualified_name = self.qualified_name(&name);
        let type_hash = TypeHash::from_name(&qualified_name);

        // Resolve underlying type (default int)
        let underlying = e.underlying_type
            .as_ref()
            .and_then(|t| self.resolver.resolve(t, e.span).ok())
            .map(|dt| dt.type_hash)
            .unwrap_or(angelscript_core::primitives::INT32);

        // Collect enum values
        let mut values = Vec::new();
        let mut next_value: i64 = 0;

        for variant in &e.variants {
            let value = variant.value.unwrap_or(next_value);
            values.push((variant.name.to_string(), value));
            next_value = value + 1;
        }

        let def = ScriptTypeDef {
            name,
            qualified_name,
            type_hash,
            kind: ScriptTypeKind::Enum { underlying, values },
            span: e.span,
        };

        self.ctx.register_script_type(def);
        self.types_registered += 1;
    }

    // ==========================================================================
    // Function
    // ==========================================================================

    fn visit_function(&mut self, func: &'ast FunctionDecl<'ast>, object_type: Option<TypeHash>) -> TypeHash {
        let name = func.name.to_string();
        let qualified_name = if let Some(obj) = object_type {
            format!("{}::{}", self.get_type_name(obj), name)
        } else {
            self.qualified_name(&name)
        };

        // Resolve parameters
        let params: Vec<ScriptParam> = func.params
            .iter()
            .map(|p| self.resolve_param(p))
            .collect();

        // Resolve return type
        let return_type = func.return_type
            .as_ref()
            .and_then(|t| self.resolver.resolve(t, func.span).ok())
            .unwrap_or_else(DataType::void);

        // Compute function hash
        let param_types: Vec<TypeHash> = params.iter().map(|p| p.data_type.type_hash).collect();
        let func_hash = if let Some(obj) = object_type {
            TypeHash::from_method(obj, &name, &param_types, func.is_const)
        } else {
            TypeHash::from_function(&qualified_name, &param_types)
        };

        let def = ScriptFunctionDef {
            name,
            qualified_name,
            func_hash,
            params,
            return_type,
            object_type,
            is_const: func.is_const,
            visibility: func.visibility,
            body_ast: func.body.as_ref().map(|_| func.body_index),  // Index for Pass 2
            span: func.span,
        };

        self.ctx.register_script_function(def);
        self.functions_registered += 1;

        func_hash
    }

    fn resolve_param(&mut self, param: &'ast ParamDecl<'ast>) -> ScriptParam {
        let data_type = self.resolver.resolve(&param.type_expr, param.span)
            .unwrap_or_else(|e| {
                self.ctx.error(e);
                DataType::void()
            });

        ScriptParam {
            name: param.name.to_string(),
            data_type,
            has_default: param.default.is_some(),
            default_ast: param.default.as_ref().map(|_| param.default_index),
        }
    }

    fn visit_constructor(&mut self, ctor: &'ast ConstructorDecl<'ast>, class_hash: TypeHash) {
        // Constructor is a special function with no return type
        let params: Vec<ScriptParam> = ctor.params
            .iter()
            .map(|p| self.resolve_param(p))
            .collect();

        let param_types: Vec<TypeHash> = params.iter().map(|p| p.data_type.type_hash).collect();
        let func_hash = TypeHash::from_constructor(class_hash, &param_types);

        let def = ScriptFunctionDef {
            name: "$ctor".to_string(),
            qualified_name: format!("{}::$ctor", self.get_type_name(class_hash)),
            func_hash,
            params,
            return_type: DataType::void(),
            object_type: Some(class_hash),
            is_const: false,
            visibility: ctor.visibility,
            body_ast: Some(ctor.body_index),
            span: ctor.span,
        };

        self.ctx.register_script_function(def);
        self.functions_registered += 1;

        // TODO: Register in class behaviors.constructors
    }

    fn visit_destructor(&mut self, dtor: &'ast DestructorDecl<'ast>, class_hash: TypeHash) {
        let func_hash = TypeHash::from_destructor(class_hash);

        let def = ScriptFunctionDef {
            name: "$dtor".to_string(),
            qualified_name: format!("{}::$dtor", self.get_type_name(class_hash)),
            func_hash,
            params: vec![],
            return_type: DataType::void(),
            object_type: Some(class_hash),
            is_const: false,
            visibility: Visibility::Public,
            body_ast: Some(dtor.body_index),
            span: dtor.span,
        };

        self.ctx.register_script_function(def);
        self.functions_registered += 1;
    }

    // ==========================================================================
    // Other Declarations
    // ==========================================================================

    fn visit_global_var(&mut self, var: &'ast VarDecl<'ast>) {
        // Global variables are registered but not compiled yet
        // Will be handled in Pass 2
    }

    fn visit_property(&mut self, prop: &'ast PropertyDecl<'ast>, class_hash: TypeHash) {
        // Register getter if present
        if let Some(getter) = &prop.getter {
            self.visit_function(getter, Some(class_hash));
        }
        // Register setter if present
        if let Some(setter) = &prop.setter {
            self.visit_function(setter, Some(class_hash));
        }
    }

    fn visit_funcdef(&mut self, fd: &'ast FuncdefDecl<'ast>) {
        let name = fd.name.to_string();
        let qualified_name = self.qualified_name(&name);
        let type_hash = TypeHash::from_name(&qualified_name);

        let params: Vec<DataType> = fd.params
            .iter()
            .filter_map(|p| self.resolver.resolve(&p.type_expr, fd.span).ok())
            .collect();

        let return_type = fd.return_type
            .as_ref()
            .and_then(|t| self.resolver.resolve(t, fd.span).ok())
            .unwrap_or_else(DataType::void);

        let def = ScriptTypeDef {
            name,
            qualified_name,
            type_hash,
            kind: ScriptTypeKind::Funcdef { params, return_type },
            span: fd.span,
        };

        self.ctx.register_script_type(def);
        self.types_registered += 1;
    }

    fn visit_typedef(&mut self, _td: &'ast TypedefDecl<'ast>) {
        // Typedef creates an alias - handled in type resolution
    }

    // ==========================================================================
    // Auto-Generation
    // ==========================================================================

    fn generate_defaults(&mut self) {
        // Auto-generate default constructor for classes without any constructor
        // Auto-generate destructor if needed
        // Auto-generate opAssign if not deleted

        // This requires iterating over registered script types
        // Implementation details depend on how we track "has constructor"
    }

    // ==========================================================================
    // Helpers
    // ==========================================================================

    fn qualified_name(&self, name: &str) -> String {
        let ns = self.ctx.current_namespace();
        if ns.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", ns, name)
        }
    }

    fn get_type_name(&self, hash: TypeHash) -> String {
        self.ctx.get_type(hash)
            .map(|t| t.name().to_string())
            .unwrap_or_else(|| format!("{:?}", hash))
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_simple_class() {
        let source = "class Player {}";
        let ast = parse(source);
        let mut registry = TypeRegistry::new();
        let mut ctx = CompilationContext::new(&mut registry, UnitId::new(0));

        let pass = RegistrationPass::new(&mut ctx, &ast);
        let output = pass.run();

        assert_eq!(output.types_registered, 1);
        assert!(ctx.resolve_type("Player").is_some());
    }

    #[test]
    fn register_class_with_methods() {
        let source = r#"
            class Player {
                void update() {}
                int getHealth() const { return 0; }
            }
        "#;
        // Should register Player type and 2 methods
    }

    #[test]
    fn register_namespace() {
        let source = r#"
            namespace game {
                class Entity {}
            }
        "#;
        // Should register game::Entity
    }

    #[test]
    fn register_inheritance() {
        let source = r#"
            class Entity {}
            class Player : Entity {}
        "#;
        // Player should have base_class = Entity hash
    }
}
```

## Acceptance Criteria

- [ ] Classes registered with correct hash and qualified name
- [ ] Methods registered with object_type set
- [ ] Interfaces registered with method list
- [ ] Enums registered with values
- [ ] Namespace handling works correctly
- [ ] Base class resolution works
- [ ] Interface implementation recorded
- [ ] Constructors/destructors registered
- [ ] Funcdefs registered
- [ ] All tests pass

## Next Phase

Task 38: Local Scope - variable tracking and scope management
