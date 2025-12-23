//! Registration result types for Pass 1 output.
//!
//! This module defines the output of the registration pass, which collects all
//! unresolved type and function declarations from the AST. The completion pass
//! transforms these into resolved entries and populates the registry.

use angelscript_core::{
    CompilationError, QualifiedName, UnitId, UnresolvedClass, UnresolvedEnum, UnresolvedFuncdef,
    UnresolvedFunction, UnresolvedGlobal, UnresolvedInterface, UnresolvedMixin,
};

/// Output of Pass 1 (Registration).
///
/// Contains all type and function declarations collected from the AST,
/// with types still unresolved. Pass 2 (Completion) transforms this into
/// resolved entries and populates the registry.
#[derive(Debug)]
pub struct RegistrationResult {
    /// Unit ID this result is for.
    pub unit_id: UnitId,

    /// Unresolved class declarations.
    pub classes: Vec<UnresolvedClass>,

    /// Unresolved mixin declarations.
    pub mixins: Vec<UnresolvedMixin>,

    /// Unresolved interface declarations.
    pub interfaces: Vec<UnresolvedInterface>,

    /// Unresolved funcdef declarations.
    pub funcdefs: Vec<UnresolvedFuncdef>,

    /// Unresolved enum declarations.
    pub enums: Vec<UnresolvedEnum>,

    /// Unresolved global function declarations.
    pub functions: Vec<UnresolvedFunction>,

    /// Unresolved global variable declarations.
    pub globals: Vec<UnresolvedGlobal>,

    /// Errors encountered during registration.
    /// These are typically syntax-level issues or duplicate declarations.
    pub errors: Vec<CompilationError>,
}

impl RegistrationResult {
    /// Create a new empty registration result.
    pub fn new(unit_id: UnitId) -> Self {
        Self {
            unit_id,
            classes: Vec::new(),
            mixins: Vec::new(),
            interfaces: Vec::new(),
            funcdefs: Vec::new(),
            enums: Vec::new(),
            functions: Vec::new(),
            globals: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Add a class declaration.
    pub fn add_class(&mut self, class: UnresolvedClass) {
        self.classes.push(class);
    }

    /// Add a mixin declaration.
    pub fn add_mixin(&mut self, mixin: UnresolvedMixin) {
        self.mixins.push(mixin);
    }

    /// Add an interface declaration.
    pub fn add_interface(&mut self, interface: UnresolvedInterface) {
        self.interfaces.push(interface);
    }

    /// Add a funcdef declaration.
    pub fn add_funcdef(&mut self, funcdef: UnresolvedFuncdef) {
        self.funcdefs.push(funcdef);
    }

    /// Add an enum declaration.
    pub fn add_enum(&mut self, e: UnresolvedEnum) {
        self.enums.push(e);
    }

    /// Add a function declaration.
    pub fn add_function(&mut self, function: UnresolvedFunction) {
        self.functions.push(function);
    }

    /// Add a global variable declaration.
    pub fn add_global(&mut self, global: UnresolvedGlobal) {
        self.globals.push(global);
    }

    /// Add an error.
    pub fn add_error(&mut self, error: CompilationError) {
        self.errors.push(error);
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get total number of type declarations.
    pub fn type_count(&self) -> usize {
        self.classes.len()
            + self.mixins.len()
            + self.interfaces.len()
            + self.funcdefs.len()
            + self.enums.len()
    }

    /// Get total number of function declarations.
    pub fn function_count(&self) -> usize {
        self.functions.len() + self.classes.iter().map(|c| c.methods.len()).sum::<usize>()
    }

    /// Get total number of global variable declarations.
    pub fn global_count(&self) -> usize {
        self.globals.len()
    }

    /// Get all type names for duplicate detection during completion.
    pub fn all_type_names(&self) -> impl Iterator<Item = &QualifiedName> {
        self.classes
            .iter()
            .map(|c| &c.name)
            .chain(self.mixins.iter().map(|m| &m.class.name))
            .chain(self.interfaces.iter().map(|i| &i.name))
            .chain(self.funcdefs.iter().map(|f| &f.name))
            .chain(self.enums.iter().map(|e| &e.name))
    }

    /// Merge another registration result into this one.
    ///
    /// Used when combining results from multiple files in the same unit.
    pub fn merge(&mut self, other: RegistrationResult) {
        self.classes.extend(other.classes);
        self.mixins.extend(other.mixins);
        self.interfaces.extend(other.interfaces);
        self.funcdefs.extend(other.funcdefs);
        self.enums.extend(other.enums);
        self.functions.extend(other.functions);
        self.globals.extend(other.globals);
        self.errors.extend(other.errors);
    }
}

/// Statistics from the registration pass.
#[derive(Debug, Default, Clone, Copy)]
pub struct RegistrationStats {
    pub classes: usize,
    pub mixins: usize,
    pub interfaces: usize,
    pub funcdefs: usize,
    pub enums: usize,
    /// Number of global functions (does not include class methods).
    pub global_functions: usize,
    /// Number of methods across all classes.
    pub methods: usize,
    pub globals: usize,
    pub errors: usize,
}

impl From<&RegistrationResult> for RegistrationStats {
    fn from(result: &RegistrationResult) -> Self {
        Self {
            classes: result.classes.len(),
            mixins: result.mixins.len(),
            interfaces: result.interfaces.len(),
            funcdefs: result.funcdefs.len(),
            enums: result.enums.len(),
            global_functions: result.functions.len(),
            methods: result.classes.iter().map(|c| c.methods.len()).sum(),
            globals: result.globals.len(),
            errors: result.errors.len(),
        }
    }
}

impl std::fmt::Display for RegistrationStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Registration: {} classes, {} mixins, {} interfaces, {} funcdefs, \
             {} enums, {} functions, {} methods, {} globals, {} errors",
            self.classes,
            self.mixins,
            self.interfaces,
            self.funcdefs,
            self.enums,
            self.global_functions,
            self.methods,
            self.globals,
            self.errors
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{Span, UnresolvedSignature, UnresolvedType};

    #[test]
    fn empty_result() {
        let result = RegistrationResult::new(UnitId::new(0));
        assert_eq!(result.type_count(), 0);
        assert_eq!(result.function_count(), 0);
        assert!(!result.has_errors());
    }

    #[test]
    fn add_class() {
        let mut result = RegistrationResult::new(UnitId::new(0));
        let class = UnresolvedClass::new(
            QualifiedName::global("Player"),
            Span::default(),
            UnitId::new(0),
        );
        result.add_class(class);

        assert_eq!(result.type_count(), 1);
        assert_eq!(result.classes.len(), 1);
    }

    #[test]
    fn merge_results() {
        let mut result1 = RegistrationResult::new(UnitId::new(0));
        result1.add_class(UnresolvedClass::new(
            QualifiedName::global("Foo"),
            Span::default(),
            UnitId::new(0),
        ));

        let mut result2 = RegistrationResult::new(UnitId::new(0));
        result2.add_class(UnresolvedClass::new(
            QualifiedName::global("Bar"),
            Span::default(),
            UnitId::new(0),
        ));

        result1.merge(result2);
        assert_eq!(result1.classes.len(), 2);
    }

    #[test]
    fn all_type_names() {
        let mut result = RegistrationResult::new(UnitId::new(0));

        result.add_class(UnresolvedClass::new(
            QualifiedName::global("MyClass"),
            Span::default(),
            UnitId::new(0),
        ));

        result.add_interface(UnresolvedInterface::new(
            QualifiedName::global("IMyInterface"),
            Span::default(),
            UnitId::new(0),
        ));

        let names: Vec<_> = result.all_type_names().collect();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn stats() {
        let mut result = RegistrationResult::new(UnitId::new(0));
        result.add_class(UnresolvedClass::new(
            QualifiedName::global("A"),
            Span::default(),
            UnitId::new(0),
        ));
        result.add_function(UnresolvedFunction::new(
            QualifiedName::global("foo"),
            Span::default(),
            UnitId::new(0),
            UnresolvedSignature::new("foo", vec![], UnresolvedType::simple("void")),
        ));

        let stats = RegistrationStats::from(&result);
        assert_eq!(stats.classes, 1);
        assert_eq!(stats.global_functions, 1);
        assert_eq!(stats.methods, 0);
    }

    #[test]
    fn stats_display() {
        let stats = RegistrationStats {
            classes: 2,
            mixins: 1,
            interfaces: 3,
            funcdefs: 0,
            enums: 1,
            global_functions: 5,
            methods: 10,
            globals: 2,
            errors: 0,
        };

        let display = format!("{}", stats);
        assert!(display.contains("2 classes"));
        assert!(display.contains("1 mixins"));
        assert!(display.contains("3 interfaces"));
        assert!(display.contains("5 functions"));
        assert!(display.contains("10 methods"));
    }

    #[test]
    fn function_count_includes_methods() {
        let mut result = RegistrationResult::new(UnitId::new(0));

        // Add a class with methods
        let mut class = UnresolvedClass::new(
            QualifiedName::global("Player"),
            Span::default(),
            UnitId::new(0),
        );
        class.methods.push(angelscript_core::UnresolvedMethod::new(
            "update",
            UnresolvedSignature::new("update", vec![], UnresolvedType::simple("void")),
            Span::default(),
        ));
        class.methods.push(angelscript_core::UnresolvedMethod::new(
            "draw",
            UnresolvedSignature::new("draw", vec![], UnresolvedType::simple("void")),
            Span::default(),
        ));
        result.add_class(class);

        // Add a global function
        result.add_function(UnresolvedFunction::new(
            QualifiedName::global("main"),
            Span::default(),
            UnitId::new(0),
            UnresolvedSignature::new("main", vec![], UnresolvedType::simple("void")),
        ));

        // Should count 2 methods + 1 global function = 3
        assert_eq!(result.function_count(), 3);
    }

    #[test]
    fn stats_includes_methods() {
        let mut result = RegistrationResult::new(UnitId::new(0));

        // Add a class with methods
        let mut class = UnresolvedClass::new(
            QualifiedName::global("Player"),
            Span::default(),
            UnitId::new(0),
        );
        class.methods.push(angelscript_core::UnresolvedMethod::new(
            "update",
            UnresolvedSignature::new("update", vec![], UnresolvedType::simple("void")),
            Span::default(),
        ));
        class.methods.push(angelscript_core::UnresolvedMethod::new(
            "draw",
            UnresolvedSignature::new("draw", vec![], UnresolvedType::simple("void")),
            Span::default(),
        ));
        result.add_class(class);

        // Add a global function
        result.add_function(UnresolvedFunction::new(
            QualifiedName::global("main"),
            Span::default(),
            UnitId::new(0),
            UnresolvedSignature::new("main", vec![], UnresolvedType::simple("void")),
        ));

        let stats = RegistrationStats::from(&result);
        assert_eq!(stats.global_functions, 1);
        assert_eq!(stats.methods, 2);
        assert_eq!(stats.classes, 1);
    }

    #[test]
    fn error_handling() {
        let mut result = RegistrationResult::new(UnitId::new(0));
        assert!(!result.has_errors());

        result.add_error(CompilationError::DuplicateDefinition {
            name: "Foo".to_string(),
            span: Span::default(),
        });
        assert!(result.has_errors());
        assert_eq!(result.errors.len(), 1);
    }
}
