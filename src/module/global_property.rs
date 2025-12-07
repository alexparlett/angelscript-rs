//! Global property definitions for FFI registration.
//!
//! Global properties allow scripts to read and write app-owned data.
//! The app owns the data; scripts access it via reference.

use std::any::Any;

use angelscript_parser::ast::{Ident, TypeExpr};

/// A global property definition for FFI registration.
///
/// Stores metadata about a global property that will be applied
/// to the Registry during `apply_to_registry()`.
///
/// The value is stored as a type-erased `&mut dyn Any` reference.
/// The `TypeExpr` provides the parsed AngelScript type information.
///
/// # Lifetimes
///
/// - `'ast`: The arena lifetime for parsed AST nodes (Ident, TypeExpr)
/// - `'app`: The application lifetime for the value reference
pub struct GlobalPropertyDef<'ast, 'app> {
    /// Property name (parsed identifier)
    pub name: Ident<'ast>,
    /// Type expression (parsed from declaration)
    pub ty: TypeExpr<'ast>,
    /// The actual value reference (type-erased)
    pub value: &'app mut dyn Any,
}

impl<'ast, 'app> GlobalPropertyDef<'ast, 'app> {
    /// Create a new global property definition.
    pub fn new<T: 'static>(
        name: Ident<'ast>,
        ty: TypeExpr<'ast>,
        value: &'app mut T,
    ) -> Self {
        Self {
            name,
            ty,
            value,
        }
    }

    /// Check if this property is const (read-only from script).
    pub fn is_const(&self) -> bool {
        self.ty.is_const
    }

    /// Try to downcast to a concrete type (immutable).
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.value.downcast_ref::<T>()
    }

    /// Try to downcast to a concrete type (mutable).
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.value.downcast_mut::<T>()
    }

    /// Get the TypeId of the stored value.
    pub fn type_id(&self) -> std::any::TypeId {
        (*self.value).type_id()
    }
}

// Manual Debug implementation since dyn Any doesn't implement Debug
impl std::fmt::Debug for GlobalPropertyDef<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobalPropertyDef")
            .field("name", &self.name)
            .field("ty", &self.ty)
            .field("value_type_id", &(*self.value).type_id())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_parser::ast::{PrimitiveType, TypeBase};
    use angelscript_parser::lexer::Span;

    fn make_ident(name: &str) -> Ident<'_> {
        Ident::new(name, Span::new(1, 1, name.len() as u32))
    }

    fn make_int_type() -> TypeExpr<'static> {
        TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 1, 3))
    }

    fn make_double_type() -> TypeExpr<'static> {
        TypeExpr::primitive(PrimitiveType::Double, Span::new(1, 1, 6))
    }

    fn make_const_double_type() -> TypeExpr<'static> {
        let mut ty = TypeExpr::primitive(PrimitiveType::Double, Span::new(1, 1, 6));
        ty.is_const = true;
        ty
    }

    fn make_named_type(name: &str) -> TypeExpr<'_> {
        TypeExpr::named(Ident::new(name, Span::new(1, 1, name.len() as u32)))
    }

    #[test]
    fn global_property_def_new() {
        let mut value = 42i32;
        let def = GlobalPropertyDef::new(make_ident("score"), make_int_type(), &mut value);

        assert_eq!(def.name.name, "score");
        assert!(matches!(def.ty.base, TypeBase::Primitive(PrimitiveType::Int)));
        assert!(!def.is_const());
    }

    #[test]
    fn global_property_def_const() {
        let mut value = 3.14f64;
        let def = GlobalPropertyDef::new(make_ident("PI"), make_const_double_type(), &mut value);

        assert_eq!(def.name.name, "PI");
        assert!(def.is_const());
    }

    #[test]
    fn global_property_def_downcast_ref() {
        let mut value = 42i32;
        let def = GlobalPropertyDef::new(make_ident("score"), make_int_type(), &mut value);

        assert_eq!(def.downcast_ref::<i32>(), Some(&42));
        assert_eq!(def.downcast_ref::<i64>(), None);
    }

    #[test]
    fn global_property_def_downcast_mut() {
        let mut value = 42i32;
        let mut def = GlobalPropertyDef::new(make_ident("score"), make_int_type(), &mut value);

        if let Some(v) = def.downcast_mut::<i32>() {
            *v = 100;
        }

        assert_eq!(def.downcast_ref::<i32>(), Some(&100));
    }

    #[test]
    fn global_property_def_type_id() {
        let mut value = 42i32;
        let def = GlobalPropertyDef::new(make_ident("score"), make_int_type(), &mut value);

        assert_eq!(def.type_id(), std::any::TypeId::of::<i32>());
    }

    #[test]
    fn global_property_def_debug() {
        let mut value = 42i32;
        let def = GlobalPropertyDef::new(make_ident("score"), make_int_type(), &mut value);
        let debug = format!("{:?}", def);

        assert!(debug.contains("GlobalPropertyDef"));
        assert!(debug.contains("score"));
    }

    #[test]
    fn global_property_def_string_type() {
        let mut value = String::from("hello");
        let def = GlobalPropertyDef::new(make_ident("greeting"), make_named_type("string"), &mut value);

        assert_eq!(def.downcast_ref::<String>(), Some(&String::from("hello")));
    }

    #[test]
    fn global_property_def_custom_type() {
        struct MyType {
            value: i32,
        }

        let mut val = MyType { value: 42 };
        let def = GlobalPropertyDef::new(make_ident("obj"), make_named_type("MyType"), &mut val);

        assert_eq!(def.type_id(), std::any::TypeId::of::<MyType>());
        assert!(def.downcast_ref::<MyType>().is_some());
    }
}
