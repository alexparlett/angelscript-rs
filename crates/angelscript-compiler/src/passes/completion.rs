//! Type Completion Pass - Copy inherited members from base classes.
//!
//! This pass runs after registration to finalize class structures by copying
//! public/protected methods and properties from base classes. This enables
//! O(1) lookups during compilation without needing to walk the inheritance
//! chain or check visibility repeatedly.
//!
//! ## Algorithm
//!
//! 1. Topologically sort classes by inheritance (base before derived)
//! 2. For each class in order:
//!    - Read public/protected members from immediate base class
//!    - Copy them to the derived class
//! 3. Because we process in topological order, each base is already complete
//!    when we process its derived classes
//!
//! ## Example
//!
//! ```text
//! class A { public void foo(); protected void bar(); private void baz(); }
//! class B : A { public void qux(); }
//!
//! After completion:
//! - A: foo(), bar(), baz() (unchanged)
//! - B: foo() (inherited), bar() (inherited), qux() (own)
//!   Note: baz() is NOT inherited (private)
//! ```

use rustc_hash::FxHashSet;

use angelscript_core::{CompilationError, Span, TypeHash, Visibility};
use angelscript_registry::SymbolRegistry;

/// Output of the type completion pass.
#[derive(Debug, Default)]
pub struct CompletionOutput {
    /// Number of classes completed.
    pub classes_completed: usize,
    /// Number of methods copied from base classes.
    pub methods_inherited: usize,
    /// Number of properties copied from base classes.
    pub properties_inherited: usize,
    /// Collected errors.
    pub errors: Vec<CompilationError>,
}

/// Inherited members to copy to a derived class.
#[derive(Debug, Default)]
struct InheritedMembers {
    /// Methods: (name, method_hash)
    methods: Vec<(String, TypeHash)>,
    /// Properties to copy
    properties: Vec<angelscript_core::PropertyEntry>,
}

/// Type Completion Pass - finalizes class structures with inherited members.
pub struct TypeCompletionPass<'reg> {
    registry: &'reg mut SymbolRegistry,
}

impl<'reg> TypeCompletionPass<'reg> {
    /// Create a new type completion pass.
    pub fn new(registry: &'reg mut SymbolRegistry) -> Self {
        Self { registry }
    }

    /// Run the type completion pass.
    pub fn run(mut self) -> CompletionOutput {
        let mut output = CompletionOutput::default();

        // Get all script class hashes
        let class_hashes: Vec<TypeHash> = self.registry.classes().map(|c| c.type_hash).collect();

        // Topologically sort classes (base before derived)
        let ordered = match self.topological_sort(&class_hashes) {
            Ok(ordered) => ordered,
            Err(e) => {
                output.errors.push(e);
                return output;
            }
        };

        // Process each class in order
        for class_hash in ordered {
            match self.complete_class(class_hash, &mut output) {
                Ok(()) => {
                    output.classes_completed += 1;
                }
                Err(e) => {
                    output.errors.push(e);
                }
            }
        }

        output
    }

    /// Complete a single class by copying inherited members.
    fn complete_class(
        &mut self,
        class_hash: TypeHash,
        output: &mut CompletionOutput,
    ) -> Result<(), CompilationError> {
        // Phase 1: Read what to inherit (immutable borrow)
        let inherited = {
            let class = self
                .registry
                .get(class_hash)
                .and_then(|e| e.as_class())
                .ok_or_else(|| CompilationError::Other {
                    message: format!("class not found: {:?}", class_hash),
                    span: Span::default(),
                })?;

            // If no base class, nothing to inherit
            let base_hash = match class.base_class {
                Some(h) => h,
                None => return Ok(()), // No inheritance, done
            };

            // Get base class (may be in global registry for FFI types)
            let base = self
                .registry
                .get(base_hash)
                .and_then(|e| e.as_class())
                .ok_or_else(|| CompilationError::UnknownType {
                    name: format!("base class {:?}", base_hash),
                    span: Span::default(),
                })?;

            // Collect inheritable members
            self.collect_inheritable_members(base)
        }; // immutable borrow ends here

        // Phase 2: Apply to derived class (mutable borrow)
        let class =
            self.registry
                .get_class_mut(class_hash)
                .ok_or_else(|| CompilationError::Other {
                    message: format!("class not found for mutation: {:?}", class_hash),
                    span: Span::default(),
                })?;

        // Copy methods
        for (name, method_hash) in inherited.methods {
            class.add_method(name, method_hash);
            output.methods_inherited += 1;
        }

        // Copy properties
        for property in inherited.properties {
            class.properties.push(property);
            output.properties_inherited += 1;
        }

        Ok(())
    }

    /// Collect public/protected members from a base class.
    fn collect_inheritable_members(&self, base: &angelscript_core::ClassEntry) -> InheritedMembers {
        let mut inherited = InheritedMembers::default();

        // Collect public/protected methods
        for (name, method_hashes) in &base.methods {
            for &method_hash in method_hashes {
                if let Some(func) = self.registry.get_function(method_hash) {
                    // Only inherit public and protected methods
                    if func.def.visibility != Visibility::Private {
                        inherited.methods.push((name.clone(), method_hash));
                    }
                }
            }
        }

        // Collect public/protected properties
        for property in &base.properties {
            if property.visibility != Visibility::Private {
                inherited.properties.push(property.clone());
            }
        }

        inherited
    }

    /// Topologically sort classes by inheritance (base before derived).
    ///
    /// Returns error if circular inheritance is detected.
    fn topological_sort(&self, classes: &[TypeHash]) -> Result<Vec<TypeHash>, CompilationError> {
        let mut visited = FxHashSet::default();
        let mut stack = Vec::new();
        let mut in_progress = FxHashSet::default();

        for &class_hash in classes {
            if !visited.contains(&class_hash) {
                self.visit(
                    class_hash,
                    classes,
                    &mut visited,
                    &mut in_progress,
                    &mut stack,
                )?;
            }
        }

        Ok(stack)
    }

    /// DFS visit for topological sort with cycle detection.
    fn visit(
        &self,
        class_hash: TypeHash,
        all_classes: &[TypeHash],
        visited: &mut FxHashSet<TypeHash>,
        in_progress: &mut FxHashSet<TypeHash>,
        stack: &mut Vec<TypeHash>,
    ) -> Result<(), CompilationError> {
        // Cycle detection
        if in_progress.contains(&class_hash) {
            let class = self
                .registry
                .get(class_hash)
                .and_then(|e| e.as_class())
                .ok_or_else(|| CompilationError::Other {
                    message: "class not found during sort".to_string(),
                    span: Span::default(),
                })?;
            return Err(CompilationError::CircularInheritance {
                name: class.name.clone(),
                span: Span::default(),
            });
        }

        if visited.contains(&class_hash) {
            return Ok(());
        }

        in_progress.insert(class_hash);

        // Visit base class first (if it's a script class)
        let class = self
            .registry
            .get(class_hash)
            .and_then(|e| e.as_class())
            .ok_or_else(|| CompilationError::Other {
                message: "class not found".to_string(),
                span: Span::default(),
            })?;

        if let Some(base_hash) = class.base_class {
            // Only visit if base is also a script class (in our list)
            if all_classes.contains(&base_hash) {
                self.visit(base_hash, all_classes, visited, in_progress, stack)?;
            }
            // If base is in global registry (FFI), it's already complete
        }

        in_progress.remove(&class_hash);
        visited.insert(class_hash);
        stack.push(class_hash);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{
        ClassEntry, DataType, FunctionDef, FunctionEntry, FunctionSource, FunctionTraits,
        PropertyEntry, TypeSource, UnitId, primitives,
    };

    fn create_test_registry() -> SymbolRegistry {
        SymbolRegistry::with_primitives()
    }

    fn register_script_function(
        registry: &mut SymbolRegistry,
        def: FunctionDef,
    ) -> Result<(), angelscript_core::RegistrationError> {
        registry.register_function(FunctionEntry::script(
            def,
            UnitId::new(0),
            FunctionSource::Script {
                span: Span::new(0, 0, 10),
            },
        ))
    }

    #[test]
    fn complete_simple_inheritance() {
        let mut registry = create_test_registry();

        // class Base { public void foo(); }
        let base = ClassEntry::script(
            "Base",
            vec![],
            "Base",
            TypeSource::script(UnitId::new(0), Span::new(0, 0, 10)),
        );
        let base_hash = base.type_hash;

        let foo_def = FunctionDef::new(
            TypeHash::from_function("Base::foo", &[]),
            "foo".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(base_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let foo_hash = foo_def.func_hash;

        let base = base.with_method("foo", foo_hash);
        registry.register_type(base.into()).unwrap();
        register_script_function(&mut registry, foo_def).unwrap();

        // class Derived : Base { }
        let derived = ClassEntry::script(
            "Derived",
            vec![],
            "Derived",
            TypeSource::script(UnitId::new(0), Span::new(0, 11, 20)),
        )
        .with_base(base_hash);
        let derived_hash = derived.type_hash;
        registry.register_type(derived.into()).unwrap();

        // Run completion pass
        let pass = TypeCompletionPass::new(&mut registry);
        let output = pass.run();

        assert_eq!(output.errors.len(), 0, "Expected no errors");
        assert_eq!(output.classes_completed, 2);
        assert_eq!(output.methods_inherited, 1);

        // Verify derived has foo()
        let derived = registry.get(derived_hash).unwrap().as_class().unwrap();
        assert_eq!(derived.find_methods("foo"), &[foo_hash]);
    }

    #[test]
    fn complete_respects_visibility() {
        let mut registry = create_test_registry();

        // class Base {
        //     public void pub_method();
        //     protected void prot_method();
        //     private void priv_method();
        // }
        let base = ClassEntry::script(
            "Base",
            vec![],
            "Base",
            TypeSource::script(UnitId::new(0), Span::new(0, 0, 10)),
        );
        let base_hash = base.type_hash;

        let pub_def = FunctionDef::new(
            TypeHash::from_function("Base::pub_method", &[]),
            "pub_method".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(base_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let pub_hash = pub_def.func_hash;

        let prot_def = FunctionDef::new(
            TypeHash::from_function("Base::prot_method", &[]),
            "prot_method".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(base_hash),
            FunctionTraits::default(),
            false,
            Visibility::Protected,
        );
        let prot_hash = prot_def.func_hash;

        let priv_def = FunctionDef::new(
            TypeHash::from_function("Base::priv_method", &[]),
            "priv_method".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(base_hash),
            FunctionTraits::default(),
            false,
            Visibility::Private,
        );
        let priv_hash = priv_def.func_hash;

        let base = base
            .with_method("pub_method", pub_hash)
            .with_method("prot_method", prot_hash)
            .with_method("priv_method", priv_hash);
        registry.register_type(base.into()).unwrap();
        register_script_function(&mut registry, pub_def).unwrap();
        register_script_function(&mut registry, prot_def).unwrap();
        register_script_function(&mut registry, priv_def).unwrap();

        // class Derived : Base { }
        let derived = ClassEntry::script(
            "Derived",
            vec![],
            "Derived",
            TypeSource::script(UnitId::new(0), Span::new(0, 11, 20)),
        )
        .with_base(base_hash);
        let derived_hash = derived.type_hash;
        registry.register_type(derived.into()).unwrap();

        // Run completion pass
        let pass = TypeCompletionPass::new(&mut registry);
        let output = pass.run();

        assert_eq!(output.errors.len(), 0);
        assert_eq!(output.methods_inherited, 2); // Only public and protected

        // Verify derived has pub_method and prot_method, NOT priv_method
        let derived = registry.get(derived_hash).unwrap().as_class().unwrap();
        assert!(derived.find_methods("pub_method").contains(&pub_hash));
        assert!(derived.find_methods("prot_method").contains(&prot_hash));
        assert!(derived.find_methods("priv_method").is_empty());
    }

    #[test]
    fn complete_chain() {
        let mut registry = create_test_registry();

        // class A { public void a(); }
        let a = ClassEntry::script(
            "A",
            vec![],
            "A",
            TypeSource::script(UnitId::new(0), Span::new(0, 0, 10)),
        );
        let a_hash = a.type_hash;

        let a_def = FunctionDef::new(
            TypeHash::from_function("A::a", &[]),
            "a".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(a_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let a_method_hash = a_def.func_hash;

        let a = a.with_method("a", a_method_hash);
        registry.register_type(a.into()).unwrap();
        register_script_function(&mut registry, a_def).unwrap();

        // class B : A { public void b(); }
        let b = ClassEntry::script(
            "B",
            vec![],
            "B",
            TypeSource::script(UnitId::new(0), Span::new(0, 11, 20)),
        )
        .with_base(a_hash);
        let b_hash = b.type_hash;

        let b_def = FunctionDef::new(
            TypeHash::from_function("B::b", &[]),
            "b".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(b_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let b_method_hash = b_def.func_hash;

        let b = b.with_method("b", b_method_hash);
        registry.register_type(b.into()).unwrap();
        register_script_function(&mut registry, b_def).unwrap();

        // class C : B { public void c(); }
        let c = ClassEntry::script(
            "C",
            vec![],
            "C",
            TypeSource::script(UnitId::new(0), Span::new(0, 21, 30)),
        )
        .with_base(b_hash);
        let c_hash = c.type_hash;

        let c_def = FunctionDef::new(
            TypeHash::from_function("C::c", &[]),
            "c".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(c_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let c_method_hash = c_def.func_hash;

        let c = c.with_method("c", c_method_hash);
        registry.register_type(c.into()).unwrap();
        register_script_function(&mut registry, c_def).unwrap();

        // Run completion pass
        let pass = TypeCompletionPass::new(&mut registry);
        let output = pass.run();

        assert_eq!(output.errors.len(), 0);
        assert_eq!(output.classes_completed, 3);
        // A inherits 0, B inherits 1 (a), C inherits 2 (a, b)
        assert_eq!(output.methods_inherited, 3);

        // Verify C has a(), b(), c()
        let c = registry.get(c_hash).unwrap().as_class().unwrap();
        assert!(c.find_methods("a").contains(&a_method_hash));
        assert!(c.find_methods("b").contains(&b_method_hash));
        assert!(c.find_methods("c").contains(&c_method_hash));
    }

    #[test]
    fn complete_detects_cycle() {
        let mut registry = create_test_registry();

        // class A : B { }
        let a = ClassEntry::script(
            "A",
            vec![],
            "A",
            TypeSource::script(UnitId::new(0), Span::new(0, 0, 10)),
        );
        let a_hash = a.type_hash;

        // class B : A { }  (creates cycle)
        let b = ClassEntry::script(
            "B",
            vec![],
            "B",
            TypeSource::script(UnitId::new(0), Span::new(0, 11, 20)),
        );
        let b_hash = b.type_hash;

        let a = a.with_base(b_hash);
        let b = b.with_base(a_hash);

        registry.register_type(a.into()).unwrap();
        registry.register_type(b.into()).unwrap();

        // Run completion pass
        let pass = TypeCompletionPass::new(&mut registry);
        let output = pass.run();

        // Should detect circular inheritance
        assert_eq!(output.errors.len(), 1);
        match &output.errors[0] {
            CompilationError::CircularInheritance { name, .. } => {
                // Should be one of A or B
                assert!(name == "A" || name == "B");
            }
            other => panic!("Expected CircularInheritance error, got: {:?}", other),
        }
    }

    #[test]
    fn complete_properties() {
        let mut registry = create_test_registry();

        // class Base {
        //     public int pub_prop;
        //     protected int prot_prop;
        //     private int priv_prop;
        // }
        let base = ClassEntry::script(
            "Base",
            vec![],
            "Base",
            TypeSource::script(UnitId::new(0), Span::new(0, 0, 10)),
        );
        let base_hash = base.type_hash;

        let pub_prop = PropertyEntry::new(
            "pub_prop",
            DataType::simple(primitives::INT32),
            Visibility::Public,
            Some(TypeHash::from_name("get_pub_prop")),
            None,
        );
        let prot_prop = PropertyEntry::new(
            "prot_prop",
            DataType::simple(primitives::INT32),
            Visibility::Protected,
            Some(TypeHash::from_name("get_prot_prop")),
            None,
        );
        let priv_prop = PropertyEntry::new(
            "priv_prop",
            DataType::simple(primitives::INT32),
            Visibility::Private,
            Some(TypeHash::from_name("get_priv_prop")),
            None,
        );

        let base = base
            .with_property(pub_prop)
            .with_property(prot_prop)
            .with_property(priv_prop);
        registry.register_type(base.into()).unwrap();

        // class Derived : Base { }
        let derived = ClassEntry::script(
            "Derived",
            vec![],
            "Derived",
            TypeSource::script(UnitId::new(0), Span::new(0, 11, 20)),
        )
        .with_base(base_hash);
        let derived_hash = derived.type_hash;
        registry.register_type(derived.into()).unwrap();

        // Run completion pass
        let pass = TypeCompletionPass::new(&mut registry);
        let output = pass.run();

        assert_eq!(output.errors.len(), 0);
        assert_eq!(output.properties_inherited, 2); // Only public and protected

        // Verify derived has pub_prop and prot_prop, NOT priv_prop
        let derived = registry.get(derived_hash).unwrap().as_class().unwrap();
        assert!(derived.find_property("pub_prop").is_some());
        assert!(derived.find_property("prot_prop").is_some());
        assert!(derived.find_property("priv_prop").is_none());
    }
}
