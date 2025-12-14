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

/// Members to copy from a mixin to an including class.
#[derive(Debug, Default)]
struct MixinMembers {
    /// Methods: (name, method_hash) - these OVERRIDE base class methods
    methods: Vec<(String, TypeHash)>,
    /// Properties to copy (only if not already inherited from base)
    properties: Vec<angelscript_core::PropertyEntry>,
    /// Interfaces the mixin declares (added to including class)
    interfaces: Vec<TypeHash>,
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
                Ok(completed) => {
                    if completed {
                        output.classes_completed += 1;
                    }
                }
                Err(e) => {
                    output.errors.push(e);
                }
            }
        }

        output
    }

    /// Complete a single class by copying inherited members and applying mixins.
    ///
    /// Returns `Ok(true)` if the class was completed, `Ok(false)` if skipped (e.g., mixins).
    fn complete_class(
        &mut self,
        class_hash: TypeHash,
        output: &mut CompletionOutput,
    ) -> Result<bool, CompilationError> {
        // Phase 1: Read what to inherit from base class (immutable borrow)
        let (inherited, mixin_hashes) = {
            let class = self
                .registry
                .get(class_hash)
                .and_then(|e| e.as_class())
                .ok_or_else(|| CompilationError::Other {
                    message: format!("class not found: {:?}", class_hash),
                    span: Span::default(),
                })?;

            // Skip mixin classes themselves - they don't get completed
            // (their members are copied to including classes instead)
            if class.is_mixin {
                return Ok(false);
            }

            let mixin_hashes = class.mixins.clone();

            let inherited = if let Some(base_hash) = class.base_class {
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
            } else {
                InheritedMembers::default()
            };

            (inherited, mixin_hashes)
        }; // immutable borrow ends here

        // Phase 2: Apply base class inheritance to derived class (mutable borrow)
        {
            let class =
                self.registry
                    .get_class_mut(class_hash)
                    .ok_or_else(|| CompilationError::Other {
                        message: format!("class not found for mutation: {:?}", class_hash),
                        span: Span::default(),
                    })?;

            // Copy methods from base class
            for (name, method_hash) in inherited.methods {
                class.add_method(name, method_hash);
                output.methods_inherited += 1;
            }

            // Copy properties from base class
            for property in inherited.properties {
                class.properties.push(property);
                output.properties_inherited += 1;
            }
        }

        // Phase 3: Apply mixin members (after base class so mixins can override)
        for mixin_hash in mixin_hashes {
            self.apply_mixin(class_hash, mixin_hash, output)?;
        }

        Ok(true)
    }

    /// Apply mixin members to an including class.
    ///
    /// Mixin semantics:
    /// - Methods: Copied to including class; OVERRIDE methods from base class
    /// - Properties: Copied only if NOT already inherited from base class
    /// - Interfaces: Added to including class
    fn apply_mixin(
        &mut self,
        class_hash: TypeHash,
        mixin_hash: TypeHash,
        output: &mut CompletionOutput,
    ) -> Result<(), CompilationError> {
        // Phase 1: Collect mixin members (immutable borrows)
        let mixin_members = {
            let mixin = self
                .registry
                .get(mixin_hash)
                .and_then(|e| e.as_class())
                .ok_or_else(|| CompilationError::UnknownType {
                    name: format!("mixin {:?}", mixin_hash),
                    span: Span::default(),
                })?;

            let class = self
                .registry
                .get(class_hash)
                .and_then(|e| e.as_class())
                .ok_or_else(|| CompilationError::Other {
                    message: format!("class not found: {:?}", class_hash),
                    span: Span::default(),
                })?;

            self.collect_mixin_members(mixin, class)
        }; // immutable borrows end here

        // Phase 2: Apply mixin members to including class (mutable borrow)
        let class =
            self.registry
                .get_class_mut(class_hash)
                .ok_or_else(|| CompilationError::Other {
                    message: format!("class not found for mutation: {:?}", class_hash),
                    span: Span::default(),
                })?;

        // Copy methods from mixin (these override base class methods)
        for (name, method_hash) in mixin_members.methods {
            // Mixin methods override base class methods with the same name
            // We add them even if a method with this name exists from base class
            // (the mixin version takes precedence)
            class.add_method(name, method_hash);
            output.methods_inherited += 1;
        }

        // Copy properties from mixin (only if not already present)
        for property in mixin_members.properties {
            class.properties.push(property);
            output.properties_inherited += 1;
        }

        // Add mixin's interfaces to including class
        for interface_hash in mixin_members.interfaces {
            if !class.interfaces.contains(&interface_hash) {
                class.interfaces.push(interface_hash);
            }
        }

        Ok(())
    }

    /// Collect members from a mixin that should be copied to the including class.
    fn collect_mixin_members(
        &self,
        mixin: &angelscript_core::ClassEntry,
        including_class: &angelscript_core::ClassEntry,
    ) -> MixinMembers {
        let mut members = MixinMembers::default();

        // Copy ALL methods from mixin (public/protected/private)
        // Mixin methods override inherited methods from base classes
        for (name, method_hashes) in &mixin.methods {
            for &method_hash in method_hashes {
                // Skip if method is explicitly declared in including class itself
                // (not inherited, but declared)
                // For now, we copy all methods since we don't track origin
                // The including class's own methods would have been registered after
                // so they would override these
                members.methods.push((name.clone(), method_hash));
            }
        }

        // Copy properties from mixin UNLESS already present in including class
        // (either declared or inherited from base class)
        for property in &mixin.properties {
            if including_class.find_property(&property.name).is_none() {
                members.properties.push(property.clone());
            }
        }

        // Collect mixin's interfaces
        members.interfaces = mixin.interfaces.clone();

        members
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

    // ==========================================================================
    // Mixin inclusion tests (Task 41e)
    // ==========================================================================

    #[test]
    fn complete_mixin_inclusion() {
        let mut registry = create_test_registry();

        // mixin class RenderMixin { void render(); }
        let mixin = ClassEntry::script_mixin(
            "RenderMixin",
            vec![],
            "RenderMixin",
            TypeSource::script(UnitId::new(0), Span::new(0, 0, 10)),
        );
        let mixin_hash = mixin.type_hash;

        let render_def = FunctionDef::new(
            TypeHash::from_function("RenderMixin::render", &[]),
            "render".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(mixin_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let render_hash = render_def.func_hash;

        let mixin = mixin.with_method("render", render_hash);
        registry.register_type(mixin.into()).unwrap();
        register_script_function(&mut registry, render_def).unwrap();

        // class Sprite : RenderMixin { void update(); }
        let sprite = ClassEntry::script(
            "Sprite",
            vec![],
            "Sprite",
            TypeSource::script(UnitId::new(0), Span::new(0, 11, 20)),
        )
        .with_mixin(mixin_hash);
        let sprite_hash = sprite.type_hash;

        let update_def = FunctionDef::new(
            TypeHash::from_function("Sprite::update", &[]),
            "update".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(sprite_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let update_hash = update_def.func_hash;

        let sprite = sprite.with_method("update", update_hash);
        registry.register_type(sprite.into()).unwrap();
        register_script_function(&mut registry, update_def).unwrap();

        // Run completion pass
        let pass = TypeCompletionPass::new(&mut registry);
        let output = pass.run();

        assert_eq!(output.errors.len(), 0, "Expected no errors");
        // Mixin is skipped, Sprite is completed with 1 method from mixin
        assert_eq!(output.classes_completed, 1);
        assert_eq!(output.methods_inherited, 1);

        // Verify Sprite has render() method from mixin
        let sprite = registry.get(sprite_hash).unwrap().as_class().unwrap();
        assert!(sprite.find_methods("render").contains(&render_hash));
        assert!(sprite.find_methods("update").contains(&update_hash));
    }

    #[test]
    fn complete_mixin_does_not_copy_existing_property() {
        let mut registry = create_test_registry();

        // mixin class PropMixin { int value; }
        let mixin = ClassEntry::script_mixin(
            "PropMixin",
            vec![],
            "PropMixin",
            TypeSource::script(UnitId::new(0), Span::new(0, 0, 10)),
        );
        let mixin_hash = mixin.type_hash;

        let mixin_prop = PropertyEntry::new(
            "value",
            DataType::simple(primitives::INT32),
            Visibility::Public,
            Some(TypeHash::from_name("get_value_mixin")),
            None,
        );
        let mixin = mixin.with_property(mixin_prop);
        registry.register_type(mixin.into()).unwrap();

        // class MyClass : PropMixin { int value; } (already has the property)
        let my_class = ClassEntry::script(
            "MyClass",
            vec![],
            "MyClass",
            TypeSource::script(UnitId::new(0), Span::new(0, 11, 20)),
        )
        .with_mixin(mixin_hash);
        let my_class_hash = my_class.type_hash;

        let class_prop = PropertyEntry::new(
            "value",
            DataType::simple(primitives::INT32),
            Visibility::Public,
            Some(TypeHash::from_name("get_value_class")),
            None,
        );
        let my_class = my_class.with_property(class_prop);
        registry.register_type(my_class.into()).unwrap();

        // Run completion pass
        let pass = TypeCompletionPass::new(&mut registry);
        let output = pass.run();

        assert_eq!(output.errors.len(), 0, "Expected no errors");
        // Property not copied because it already exists
        assert_eq!(output.properties_inherited, 0);

        // Verify MyClass still has just one "value" property (its own)
        let my_class = registry.get(my_class_hash).unwrap().as_class().unwrap();
        let props: Vec<_> = my_class
            .properties
            .iter()
            .filter(|p| p.name == "value")
            .collect();
        assert_eq!(props.len(), 1);
        // Should be the class's property, not the mixin's
        assert_eq!(
            props[0].getter,
            Some(TypeHash::from_name("get_value_class"))
        );
    }

    #[test]
    fn complete_mixin_adds_interfaces() {
        let mut registry = create_test_registry();

        let iface_hash = TypeHash::from_name("IDrawable");
        let iface = angelscript_core::InterfaceEntry::new(
            "IDrawable",
            vec![],
            "IDrawable",
            iface_hash,
            TypeSource::script(UnitId::new(0), Span::new(0, 0, 10)),
        );
        registry.register_type(iface.into()).unwrap();

        // mixin class RenderMixin : IDrawable { }
        let mixin = ClassEntry::script_mixin(
            "RenderMixin",
            vec![],
            "RenderMixin",
            TypeSource::script(UnitId::new(0), Span::new(0, 11, 20)),
        )
        .with_interface(iface_hash);
        let mixin_hash = mixin.type_hash;
        registry.register_type(mixin.into()).unwrap();

        // class Sprite : RenderMixin { }
        let sprite = ClassEntry::script(
            "Sprite",
            vec![],
            "Sprite",
            TypeSource::script(UnitId::new(0), Span::new(0, 21, 30)),
        )
        .with_mixin(mixin_hash);
        let sprite_hash = sprite.type_hash;
        registry.register_type(sprite.into()).unwrap();

        // Run completion pass
        let pass = TypeCompletionPass::new(&mut registry);
        let output = pass.run();

        assert_eq!(output.errors.len(), 0, "Expected no errors");

        // Verify Sprite now implements IDrawable (from mixin)
        let sprite = registry.get(sprite_hash).unwrap().as_class().unwrap();
        assert!(sprite.interfaces.contains(&iface_hash));
    }

    #[test]
    fn complete_mixin_skips_mixin_classes() {
        let mut registry = create_test_registry();

        // mixin class Helper { void help(); }
        let mixin = ClassEntry::script_mixin(
            "Helper",
            vec![],
            "Helper",
            TypeSource::script(UnitId::new(0), Span::new(0, 0, 10)),
        );
        let mixin_hash = mixin.type_hash;
        registry.register_type(mixin.into()).unwrap();

        // Run completion pass
        let pass = TypeCompletionPass::new(&mut registry);
        let output = pass.run();

        assert_eq!(output.errors.len(), 0, "Expected no errors");
        // Mixin itself should be skipped (classes_completed = 0 for just a mixin)
        assert_eq!(output.classes_completed, 0);

        // Verify the mixin is still in the registry and unchanged
        let mixin = registry.get(mixin_hash).unwrap().as_class().unwrap();
        assert!(mixin.is_mixin);
    }

    #[test]
    fn complete_mixin_with_base_class() {
        let mut registry = create_test_registry();

        // class Base { void base_method(); }
        let base = ClassEntry::script(
            "Base",
            vec![],
            "Base",
            TypeSource::script(UnitId::new(0), Span::new(0, 0, 10)),
        );
        let base_hash = base.type_hash;

        let base_method_def = FunctionDef::new(
            TypeHash::from_function("Base::base_method", &[]),
            "base_method".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(base_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let base_method_hash = base_method_def.func_hash;

        let base = base.with_method("base_method", base_method_hash);
        registry.register_type(base.into()).unwrap();
        register_script_function(&mut registry, base_method_def).unwrap();

        // mixin class Helper { void mixin_method(); }
        let mixin = ClassEntry::script_mixin(
            "Helper",
            vec![],
            "Helper",
            TypeSource::script(UnitId::new(0), Span::new(0, 11, 20)),
        );
        let mixin_hash = mixin.type_hash;

        let mixin_method_def = FunctionDef::new(
            TypeHash::from_function("Helper::mixin_method", &[]),
            "mixin_method".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(mixin_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let mixin_method_hash = mixin_method_def.func_hash;

        let mixin = mixin.with_method("mixin_method", mixin_method_hash);
        registry.register_type(mixin.into()).unwrap();
        register_script_function(&mut registry, mixin_method_def).unwrap();

        // class Derived : Base, Helper { void own_method(); }
        let derived = ClassEntry::script(
            "Derived",
            vec![],
            "Derived",
            TypeSource::script(UnitId::new(0), Span::new(0, 21, 30)),
        )
        .with_base(base_hash)
        .with_mixin(mixin_hash);
        let derived_hash = derived.type_hash;

        let own_method_def = FunctionDef::new(
            TypeHash::from_function("Derived::own_method", &[]),
            "own_method".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(derived_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let own_method_hash = own_method_def.func_hash;

        let derived = derived.with_method("own_method", own_method_hash);
        registry.register_type(derived.into()).unwrap();
        register_script_function(&mut registry, own_method_def).unwrap();

        // Run completion pass
        let pass = TypeCompletionPass::new(&mut registry);
        let output = pass.run();

        assert_eq!(output.errors.len(), 0, "Expected no errors");
        // Base completed (0 inherited), Derived completed (1 from base + 1 from mixin)
        assert_eq!(output.methods_inherited, 2);

        // Verify Derived has all three methods
        let derived = registry.get(derived_hash).unwrap().as_class().unwrap();
        assert!(
            derived
                .find_methods("base_method")
                .contains(&base_method_hash)
        );
        assert!(
            derived
                .find_methods("mixin_method")
                .contains(&mixin_method_hash)
        );
        assert!(
            derived
                .find_methods("own_method")
                .contains(&own_method_hash)
        );
    }

    #[test]
    fn complete_mixin_method_overrides_base_method() {
        let mut registry = create_test_registry();

        // class Base { void shared_method(); }
        let base = ClassEntry::script(
            "Base",
            vec![],
            "Base",
            TypeSource::script(UnitId::new(0), Span::new(0, 0, 10)),
        );
        let base_hash = base.type_hash;

        let base_method_def = FunctionDef::new(
            TypeHash::from_function("Base::shared_method", &[]),
            "shared_method".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(base_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let base_method_hash = base_method_def.func_hash;

        let base = base.with_method("shared_method", base_method_hash);
        registry.register_type(base.into()).unwrap();
        register_script_function(&mut registry, base_method_def).unwrap();

        // mixin class Helper { void shared_method(); }  <-- same name as base!
        let mixin = ClassEntry::script_mixin(
            "Helper",
            vec![],
            "Helper",
            TypeSource::script(UnitId::new(0), Span::new(0, 11, 20)),
        );
        let mixin_hash = mixin.type_hash;

        let mixin_method_def = FunctionDef::new(
            TypeHash::from_function("Helper::shared_method", &[]),
            "shared_method".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(mixin_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let mixin_method_hash = mixin_method_def.func_hash;

        let mixin = mixin.with_method("shared_method", mixin_method_hash);
        registry.register_type(mixin.into()).unwrap();
        register_script_function(&mut registry, mixin_method_def).unwrap();

        // class Derived : Base, Helper { }
        let derived = ClassEntry::script(
            "Derived",
            vec![],
            "Derived",
            TypeSource::script(UnitId::new(0), Span::new(0, 21, 30)),
        )
        .with_base(base_hash)
        .with_mixin(mixin_hash);
        let derived_hash = derived.type_hash;
        registry.register_type(derived.into()).unwrap();

        // Run completion pass
        let pass = TypeCompletionPass::new(&mut registry);
        let output = pass.run();

        assert_eq!(output.errors.len(), 0, "Expected no errors");

        // Verify Derived has both method hashes under "shared_method"
        // (mixin method added after base, so both exist as potential overloads)
        let derived = registry.get(derived_hash).unwrap().as_class().unwrap();
        let shared_methods = derived.find_methods("shared_method");

        // Both base and mixin versions are stored (mixin added last takes precedence in dispatch)
        assert!(
            shared_methods.contains(&base_method_hash),
            "Should have base method"
        );
        assert!(
            shared_methods.contains(&mixin_method_hash),
            "Should have mixin method"
        );

        // Mixin method should be added last (this is the override semantic -
        // during method resolution, the mixin version takes precedence)
        assert_eq!(shared_methods.len(), 2);
        assert_eq!(
            shared_methods[1], mixin_method_hash,
            "Mixin method should be last (higher precedence)"
        );
    }
}
