//! Type conversion system.
//!
//! This module provides the type conversion checking system that determines if one type
//! can be converted to another, and at what cost. This is essential for:
//!
//! - Type checking (can argument X be passed to parameter Y?)
//! - Overload resolution (which function is the best match?)
//! - Implicit vs explicit conversions
//!
//! ## Conversion Priority
//!
//! Conversions are checked in this order:
//! 1. Identity (exact match)
//! 2. Const relaxation (non-const to const)
//! 3. Primitive conversions (int widening, float, etc.)
//! 4. Handle conversions (null to handle, handle to const)
//! 5. Class hierarchy (derived to base, class to interface)
//! 6. User-defined (opImplConv, opCast, constructors)

use angelscript_core::{DataType, TypeHash};

use crate::context::CompilationContext;

mod handle;
mod primitive;
mod user_defined;

pub use handle::find_handle_conversion;
pub use primitive::{find_primitive_conversion, is_primitive_numeric};
pub use user_defined::find_user_conversion;

/// A type conversion with its cost for overload resolution.
///
/// When resolving overloaded functions, the compiler needs to track
/// what conversions are required for each argument and their relative
/// costs to choose the best match.
#[derive(Debug, Clone, PartialEq)]
pub struct Conversion {
    /// The kind of conversion being performed.
    pub kind: ConversionKind,
    /// The cost of this conversion (lower is better).
    pub cost: u32,
    /// Whether this conversion can be applied implicitly.
    pub is_implicit: bool,
}

/// The kind of conversion being performed.
#[derive(Debug, Clone, PartialEq)]
pub enum ConversionKind {
    /// No conversion needed (exact match).
    Identity,

    /// Primitive type conversion (int -> float, etc.).
    Primitive {
        /// Source type hash.
        from: TypeHash,
        /// Target type hash.
        to: TypeHash,
    },

    /// Null literal to handle type.
    NullToHandle,

    /// Handle to const handle.
    HandleToConst,

    /// Derived class to base class.
    DerivedToBase {
        /// The base class type hash.
        base: TypeHash,
    },

    /// Class to interface it implements.
    ClassToInterface {
        /// The interface type hash.
        interface: TypeHash,
    },

    /// Implicit conversion via constructor.
    ConstructorConversion {
        /// The constructor function hash.
        constructor: TypeHash,
    },

    /// Implicit conversion via opImplConv method.
    ImplicitConvMethod {
        /// The conversion method hash.
        method: TypeHash,
    },

    /// Explicit cast via opCast method.
    ExplicitCastMethod {
        /// The cast method hash.
        method: TypeHash,
    },

    /// Value type to handle (@value).
    ValueToHandle,

    /// Enum to underlying integer type.
    EnumToInt,

    /// Integer to enum type.
    IntToEnum {
        /// The enum type hash.
        enum_type: TypeHash,
    },

    /// Reference cast - derived handle to base handle.
    /// This includes both class hierarchy (derived to base) and interface casts.
    ReferenceCast {
        /// The target type hash (base class or interface).
        target: TypeHash,
    },

    /// Variable argument type (?).
    /// Used when a function accepts any type via '?' parameter.
    VarArg,
}

impl Conversion {
    // Cost constants follow AngelScript's overload resolution priority order.
    // Lower cost = better match. The ordering determines which overload wins.
    //
    // AngelScript priority (best to worst):
    // 1. no conversion needed
    // 2. conversion to const
    // 3. enum to integer of same size
    // 4. enum to integer of different size
    // 5. size of primitive type increases (widening)
    // 6. size of primitive type decreases (narrowing)
    // 7. signed to unsigned integer
    // 8. unsigned to signed integer
    // 9. integer to float
    // 10. float to integer
    // 11. reference cast
    // 12. object to primitive (user-defined)
    // 13. conversion to object (user-defined)
    // 14. variable argument type

    /// Cost for exact match (identity conversion).
    pub const COST_EXACT: u32 = 0;
    /// Cost for adding const qualifier.
    pub const COST_CONST_ADDITION: u32 = 1;
    /// Cost for enum to integer of same size.
    pub const COST_ENUM_SAME_SIZE: u32 = 2;
    /// Cost for enum to integer of different size.
    pub const COST_ENUM_DIFF_SIZE: u32 = 3;
    /// Cost for primitive widening (int8 -> int32, float -> double).
    pub const COST_PRIMITIVE_WIDENING: u32 = 4;
    /// Cost for primitive narrowing (int32 -> int8, double -> float).
    pub const COST_PRIMITIVE_NARROWING: u32 = 5;
    /// Cost for signed to unsigned integer conversion.
    pub const COST_SIGNED_TO_UNSIGNED: u32 = 6;
    /// Cost for unsigned to signed integer conversion.
    pub const COST_UNSIGNED_TO_SIGNED: u32 = 7;
    /// Cost for integer to float conversion.
    pub const COST_INT_TO_FLOAT: u32 = 8;
    /// Cost for float to integer conversion.
    pub const COST_FLOAT_TO_INT: u32 = 9;
    /// Cost for reference cast (derived handle to base handle).
    pub const COST_REFERENCE_CAST: u32 = 10;
    /// Cost for user-defined object to primitive conversion.
    pub const COST_OBJECT_TO_PRIMITIVE: u32 = 11;
    /// Cost for user-defined conversion to object.
    pub const COST_TO_OBJECT: u32 = 12;
    /// Cost for variable argument type (?).
    pub const COST_VAR_ARG: u32 = 13;
    /// Cost marker for explicit-only conversions (not usable implicitly).
    pub const COST_EXPLICIT_ONLY: u32 = 100;

    // Legacy aliases for backwards compatibility with existing code
    /// Cost for enum to/from integer conversion (legacy, use COST_ENUM_SAME_SIZE or COST_ENUM_DIFF_SIZE).
    pub const COST_ENUM_CONVERSION: u32 = Self::COST_ENUM_SAME_SIZE;
    /// Cost for derived-to-base conversion.
    pub const COST_DERIVED_TO_BASE: u32 = Self::COST_REFERENCE_CAST;
    /// Cost for class-to-interface conversion.
    pub const COST_CLASS_TO_INTERFACE: u32 = Self::COST_REFERENCE_CAST;
    /// Cost for user-defined implicit conversion.
    pub const COST_USER_IMPLICIT: u32 = Self::COST_TO_OBJECT;

    /// Create an identity conversion (no conversion needed).
    pub(crate) fn identity() -> Self {
        Self {
            kind: ConversionKind::Identity,
            cost: Self::COST_EXACT,
            is_implicit: true,
        }
    }

    /// Check if this conversion can be used implicitly.
    pub fn is_implicit(&self) -> bool {
        self.is_implicit
    }

    /// Check if this is an exact match (no conversion).
    pub fn is_exact(&self) -> bool {
        matches!(self.kind, ConversionKind::Identity)
    }
}

/// Check if source type can convert to target type.
///
/// Returns `Some(Conversion)` if a conversion exists, with cost and implicit flag.
/// Returns `None` if no conversion is possible.
pub fn find_conversion(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<Conversion> {
    use angelscript_core::primitives;

    // 1. Identity check (exact match including modifiers)
    if source == target {
        return Some(Conversion::identity());
    }

    // 2. Same base type - check modifier conversions
    if source.type_hash == target.type_hash {
        // Const relaxation: non-const to const is free
        if !source.is_const && target.is_const && !source.is_handle && !target.is_handle {
            return Some(Conversion {
                kind: ConversionKind::Identity,
                cost: Conversion::COST_CONST_ADDITION,
                is_implicit: true,
            });
        }
    }

    // 3. Enum conversions
    if let Some(conv) = find_enum_conversion(source, target, ctx) {
        return Some(conv);
    }

    // 4. Primitive conversions
    if let Some(conv) = primitive::find_primitive_conversion(source, target) {
        return Some(conv);
    }

    // 5. Handle conversions
    if let Some(conv) = handle::find_handle_conversion(source, target) {
        return Some(conv);
    }

    // 6. Class hierarchy conversions
    if let Some(conv) = find_hierarchy_conversion(source, target, ctx) {
        return Some(conv);
    }

    // 7. User-defined conversions (constructor, opConv, opCast)
    if let Some(conv) = user_defined::find_user_conversion(source, target, ctx) {
        return Some(conv);
    }

    // 8. Variable argument type (?) - accepts any type
    // This is the lowest priority implicit conversion
    if target.type_hash == primitives::VARIABLE_PARAM {
        return Some(Conversion {
            kind: ConversionKind::VarArg,
            cost: Conversion::COST_VAR_ARG,
            is_implicit: true,
        });
    }

    None
}

/// Check if implicit conversion is possible.
pub fn can_implicitly_convert(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> bool {
    find_conversion(source, target, ctx)
        .map(|c| c.is_implicit)
        .unwrap_or(false)
}

/// Find class hierarchy conversion (derived to base, class to interface).
///
/// For handle types, this is a "reference cast" with cost COST_REFERENCE_CAST.
/// For value types, this uses DerivedToBase/ClassToInterface with same cost.
fn find_hierarchy_conversion(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<Conversion> {
    let source_class = ctx.get_type(source.type_hash)?.as_class()?;
    let is_handle_conversion = source.is_handle && target.is_handle;

    // Derived to base class
    if is_derived_from(source.type_hash, target.type_hash, ctx) {
        let kind = if is_handle_conversion {
            ConversionKind::ReferenceCast {
                target: target.type_hash,
            }
        } else {
            ConversionKind::DerivedToBase {
                base: target.type_hash,
            }
        };
        return Some(Conversion {
            kind,
            cost: Conversion::COST_REFERENCE_CAST,
            is_implicit: true,
        });
    }

    // Class to interface
    if source_class.interfaces.contains(&target.type_hash) {
        // Verify target is actually an interface
        if ctx
            .get_type(target.type_hash)
            .and_then(|t| t.as_interface())
            .is_some()
        {
            let kind = if is_handle_conversion {
                ConversionKind::ReferenceCast {
                    target: target.type_hash,
                }
            } else {
                ConversionKind::ClassToInterface {
                    interface: target.type_hash,
                }
            };
            return Some(Conversion {
                kind,
                cost: Conversion::COST_REFERENCE_CAST,
                is_implicit: true,
            });
        }
    }

    None
}

/// Check if source is derived from target (walks inheritance chain).
fn is_derived_from(source: TypeHash, target: TypeHash, ctx: &CompilationContext<'_>) -> bool {
    let mut current = source;
    while let Some(class) = ctx.get_type(current).and_then(|t| t.as_class()) {
        if let Some(base) = class.base_class {
            if base == target {
                return true;
            }
            current = base;
        } else {
            break;
        }
    }
    false
}

/// Find enum conversion (enum to int, int to enum).
///
/// AngelScript enums default to int (32-bit). Same-size conversions have
/// lower cost than different-size conversions.
fn find_enum_conversion(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<Conversion> {
    use angelscript_core::primitives;
    use primitive::is_primitive_numeric;

    // Enum to integer
    if ctx
        .get_type(source.type_hash)
        .and_then(|t| t.as_enum())
        .is_some()
        && is_primitive_numeric(target.type_hash)
    {
        // Enums default to int32, so int32 is "same size"
        let cost = if target.type_hash == primitives::INT32 {
            Conversion::COST_ENUM_SAME_SIZE
        } else {
            Conversion::COST_ENUM_DIFF_SIZE
        };
        return Some(Conversion {
            kind: ConversionKind::EnumToInt,
            cost,
            is_implicit: true,
        });
    }

    // Integer to enum
    if is_primitive_numeric(source.type_hash)
        && ctx
            .get_type(target.type_hash)
            .and_then(|t| t.as_enum())
            .is_some()
    {
        // Enums default to int32, so int32 is "same size"
        let cost = if source.type_hash == primitives::INT32 {
            Conversion::COST_ENUM_SAME_SIZE
        } else {
            Conversion::COST_ENUM_DIFF_SIZE
        };
        return Some(Conversion {
            kind: ConversionKind::IntToEnum {
                enum_type: target.type_hash,
            },
            cost,
            is_implicit: true,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{ClassEntry, InterfaceEntry, TypeKind, primitives};
    use angelscript_registry::SymbolRegistry;

    #[test]
    fn identity_conversion() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let dt = DataType::simple(primitives::INT32);
        let conv = find_conversion(&dt, &dt, &ctx);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_exact());
        assert!(conv.is_implicit());
        assert_eq!(conv.cost, Conversion::COST_EXACT);
    }

    #[test]
    fn const_relaxation() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let from = DataType::simple(primitives::INT32);
        let to = DataType::simple(primitives::INT32).as_const();
        let conv = find_conversion(&from, &to, &ctx);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit());
        assert_eq!(conv.cost, Conversion::COST_CONST_ADDITION);
    }

    #[test]
    fn integer_widening() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let from = DataType::simple(primitives::INT32);
        let to = DataType::simple(primitives::INT64);
        let conv = find_conversion(&from, &to, &ctx);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit());
        assert_eq!(conv.cost, Conversion::COST_PRIMITIVE_WIDENING);
    }

    #[test]
    fn integer_narrowing() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let from = DataType::simple(primitives::INT64);
        let to = DataType::simple(primitives::INT32);
        let conv = find_conversion(&from, &to, &ctx);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit()); // AngelScript allows implicit narrowing
        assert_eq!(conv.cost, Conversion::COST_PRIMITIVE_NARROWING);
    }

    #[test]
    fn null_to_handle() {
        let mut registry = SymbolRegistry::with_primitives();

        let player_hash = TypeHash::from_name("Player");
        let player_class = ClassEntry::ffi("Player", TypeKind::reference());
        registry.register_type(player_class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);

        let from = DataType::null_literal();
        let to = DataType::simple(player_hash).as_handle();
        let conv = find_conversion(&from, &to, &ctx);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit());
        assert!(matches!(conv.kind, ConversionKind::NullToHandle));
    }

    #[test]
    fn handle_to_const_handle() {
        let mut registry = SymbolRegistry::with_primitives();

        let player_hash = TypeHash::from_name("Player");
        let player_class = ClassEntry::ffi("Player", TypeKind::reference());
        registry.register_type(player_class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);

        let from = DataType::simple(player_hash).as_handle();
        let to = DataType::simple(player_hash).as_handle_to_const();
        let conv = find_conversion(&from, &to, &ctx);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit());
        assert!(matches!(conv.kind, ConversionKind::HandleToConst));
    }

    #[test]
    fn derived_to_base() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create base class
        let base_hash = TypeHash::from_name("Entity");
        let base_class = ClassEntry::ffi("Entity", TypeKind::reference());
        registry.register_type(base_class.into()).unwrap();

        // Create derived class
        let derived_hash = TypeHash::from_name("Player");
        let derived_class = ClassEntry::ffi("Player", TypeKind::reference()).with_base(base_hash);
        registry.register_type(derived_class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);

        let from = DataType::simple(derived_hash);
        let to = DataType::simple(base_hash);
        let conv = find_conversion(&from, &to, &ctx);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit());
        assert_eq!(conv.cost, Conversion::COST_DERIVED_TO_BASE);
        assert!(matches!(
            conv.kind,
            ConversionKind::DerivedToBase { base } if base == base_hash
        ));
    }

    #[test]
    fn class_to_interface() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create interface
        let interface_hash = TypeHash::from_name("IDrawable");
        let interface = InterfaceEntry::ffi("IDrawable");
        registry.register_type(interface.into()).unwrap();

        // Create class implementing interface
        let class_hash = TypeHash::from_name("Sprite");
        let class = ClassEntry::ffi("Sprite", TypeKind::reference()).with_interface(interface_hash);
        registry.register_type(class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);

        let from = DataType::simple(class_hash);
        let to = DataType::simple(interface_hash);
        let conv = find_conversion(&from, &to, &ctx);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit());
        assert_eq!(conv.cost, Conversion::COST_CLASS_TO_INTERFACE);
        assert!(matches!(
            conv.kind,
            ConversionKind::ClassToInterface { interface } if interface == interface_hash
        ));
    }

    #[test]
    fn no_conversion_unrelated_types() {
        let mut registry = SymbolRegistry::with_primitives();

        let player_hash = TypeHash::from_name("Player");
        let enemy_hash = TypeHash::from_name("Enemy");

        let player_class = ClassEntry::ffi("Player", TypeKind::reference());
        let enemy_class = ClassEntry::ffi("Enemy", TypeKind::reference());
        registry.register_type(player_class.into()).unwrap();
        registry.register_type(enemy_class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);

        let from = DataType::simple(player_hash);
        let to = DataType::simple(enemy_hash);
        let conv = find_conversion(&from, &to, &ctx);

        assert!(conv.is_none());
    }

    #[test]
    fn can_implicitly_convert_works() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let int32 = DataType::simple(primitives::INT32);
        let int64 = DataType::simple(primitives::INT64);

        assert!(can_implicitly_convert(&int32, &int64, &ctx));
        assert!(can_implicitly_convert(&int32, &int32, &ctx)); // Identity
    }

    #[test]
    fn multi_level_inheritance() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create hierarchy: Entity -> Character -> Player
        let entity_hash = TypeHash::from_name("Entity");
        let entity = ClassEntry::ffi("Entity", TypeKind::reference());
        registry.register_type(entity.into()).unwrap();

        let character_hash = TypeHash::from_name("Character");
        let character = ClassEntry::ffi("Character", TypeKind::reference()).with_base(entity_hash);
        registry.register_type(character.into()).unwrap();

        let player_hash = TypeHash::from_name("Player");
        let player = ClassEntry::ffi("Player", TypeKind::reference()).with_base(character_hash);
        registry.register_type(player.into()).unwrap();

        let ctx = CompilationContext::new(&registry);

        // Player -> Entity (two levels up)
        let from = DataType::simple(player_hash);
        let to = DataType::simple(entity_hash);
        let conv = find_conversion(&from, &to, &ctx);

        assert!(conv.is_some());
        assert!(conv.unwrap().is_implicit());
    }

    #[test]
    fn enum_to_int() {
        use angelscript_core::{EnumEntry, TypeSource};

        let mut registry = SymbolRegistry::with_primitives();

        let status_enum = EnumEntry::new(
            "Status",
            vec![],
            "Status",
            TypeHash::from_name("Status"),
            TypeSource::ffi_untyped(),
        );
        registry.register_type(status_enum.into()).unwrap();

        let ctx = CompilationContext::new(&registry);

        let from = DataType::simple(TypeHash::from_name("Status"));
        let to = DataType::simple(primitives::INT32);
        let conv = find_conversion(&from, &to, &ctx);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit());
        assert_eq!(conv.cost, Conversion::COST_ENUM_CONVERSION);
        assert!(matches!(conv.kind, ConversionKind::EnumToInt));
    }

    #[test]
    fn int_to_enum() {
        use angelscript_core::{EnumEntry, TypeSource};

        let mut registry = SymbolRegistry::with_primitives();

        let status_hash = TypeHash::from_name("Status");
        let status_enum = EnumEntry::new(
            "Status",
            vec![],
            "Status",
            status_hash,
            TypeSource::ffi_untyped(),
        );
        registry.register_type(status_enum.into()).unwrap();

        let ctx = CompilationContext::new(&registry);

        let from = DataType::simple(primitives::INT32);
        let to = DataType::simple(status_hash);
        let conv = find_conversion(&from, &to, &ctx);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit());
        assert_eq!(conv.cost, Conversion::COST_ENUM_CONVERSION);
        assert!(matches!(
            conv.kind,
            ConversionKind::IntToEnum { enum_type } if enum_type == status_hash
        ));
    }
}
