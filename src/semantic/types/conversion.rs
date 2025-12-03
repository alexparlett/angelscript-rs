//! Type conversion system for AngelScript.
//!
//! This module implements type conversion rules including:
//! - Primitive type conversions (int ↔ float, widening/narrowing)
//! - Handle conversions (T@ → const T@, Derived@ → Base@)
//! - User-defined conversions (constructors, opConv, opImplConv, opCast, opImplCast)
//!
//! The semantic analyzer determines IF a conversion is valid and WHICH kind.
//! The code generator (FunctionCompiler) selects the specific bytecode instruction.

use super::{
    data_type::DataType,
    registry::Registry,
    type_def::{
        FunctionId, OperatorBehavior, TypeDef, TypeId, DOUBLE_TYPE, FLOAT_TYPE, INT16_TYPE,
        INT32_TYPE, INT64_TYPE, INT8_TYPE, NULL_TYPE, UINT16_TYPE, UINT32_TYPE, UINT64_TYPE,
        UINT8_TYPE,
    },
};

/// The kind of type conversion.
///
/// Describes HOW a type converts without specifying the bytecode instruction.
/// The code generator uses this to select the appropriate instruction.
#[derive(Debug, Clone, PartialEq)]
pub enum ConversionKind {
    /// No conversion needed (types are identical)
    Identity,

    /// Primitive type conversion
    ///
    /// The code generator selects the specific instruction based on (from_type, to_type).
    /// Examples: int32→float32, uint64→double, float→int, etc.
    Primitive {
        from_type: TypeId,
        to_type: TypeId,
    },

    /// Null literal to handle conversion (null → T@)
    NullToHandle,

    /// Handle to const handle conversion (T@ → const T@)
    HandleToConst,

    /// Derived class handle to base class handle (Derived@ → Base@)
    DerivedToBase,

    /// Class handle to interface handle (Class@ → Interface@)
    ClassToInterface,

    /// User-defined implicit conversion via constructor
    ConstructorConversion {
        constructor_id: FunctionId,
    },

    /// User-defined implicit conversion via opImplConv method
    ImplicitConversionMethod {
        method_id: FunctionId,
    },

    /// User-defined explicit conversion via opCast method
    ExplicitCastMethod {
        method_id: FunctionId,
    },

    /// User-defined explicit conversion via opImplCast method
    ImplicitCastMethod {
        method_id: FunctionId,
    },

    /// Value type to handle conversion (T → T@)
    /// This occurs when initializing a handle with a value, e.g., Node@ n = Node(args)
    ValueToHandle,
}

/// Represents a valid type conversion.
///
/// The semantic analyzer uses this to determine:
/// - IF a conversion exists
/// - WHAT the cost is (for overload resolution)
/// - WHETHER it can happen implicitly
/// - WHICH kind of conversion it is (for code generation)
#[derive(Debug, Clone, PartialEq)]
pub struct Conversion {
    /// Kind of conversion (determines which bytecode instruction to use)
    pub kind: ConversionKind,

    /// Cost of this conversion (for overload resolution)
    ///
    /// Lower cost = better match:
    /// - 0: Exact match
    /// - 1: Primitive implicit widening
    /// - 2: Handle to const
    /// - 3: Derived to base
    /// - 5: Class to interface
    /// - 10: User-defined implicit
    /// - 100: Explicit only (narrowing, user-defined explicit)
    pub cost: u32,

    /// Can this conversion happen implicitly?
    ///
    /// - true: Can happen automatically (assignments, function args, etc.)
    /// - false: Requires explicit cast
    pub is_implicit: bool,
}

impl Conversion {
    /// Create an exact match conversion (no conversion needed)
    pub fn identity() -> Self {
        Self {
            kind: ConversionKind::Identity,
            cost: 0,
            is_implicit: true,
        }
    }

    /// Create a primitive conversion
    pub fn primitive(from_type: TypeId, to_type: TypeId, cost: u32, is_implicit: bool) -> Self {
        Self {
            kind: ConversionKind::Primitive { from_type, to_type },
            cost,
            is_implicit,
        }
    }

    /// Create a null-to-handle conversion
    pub fn null_to_handle() -> Self {
        Self {
            kind: ConversionKind::NullToHandle,
            cost: 1,
            is_implicit: true,
        }
    }

    /// Create a handle-to-const conversion
    pub fn handle_to_const() -> Self {
        Self {
            kind: ConversionKind::HandleToConst,
            cost: 2,
            is_implicit: true,
        }
    }

    /// Create a derived-to-base conversion
    pub fn derived_to_base() -> Self {
        Self {
            kind: ConversionKind::DerivedToBase,
            cost: 3,
            is_implicit: true,
        }
    }

    /// Create a class-to-interface conversion
    pub fn class_to_interface() -> Self {
        Self {
            kind: ConversionKind::ClassToInterface,
            cost: 5,
            is_implicit: true,
        }
    }

    /// Create a constructor conversion
    pub fn constructor(constructor_id: FunctionId) -> Self {
        Self {
            kind: ConversionKind::ConstructorConversion { constructor_id },
            cost: 10,
            is_implicit: true,
        }
    }

    /// Create an implicit conversion method
    pub fn implicit_conv_method(method_id: FunctionId) -> Self {
        Self {
            kind: ConversionKind::ImplicitConversionMethod { method_id },
            cost: 10,
            is_implicit: true,
        }
    }

    /// Create an explicit cast method
    pub fn explicit_cast_method(method_id: FunctionId) -> Self {
        Self {
            kind: ConversionKind::ExplicitCastMethod { method_id },
            cost: 100,
            is_implicit: false,
        }
    }

    /// Create an implicit cast method
    pub fn implicit_cast_method(method_id: FunctionId) -> Self {
        Self {
            kind: ConversionKind::ImplicitCastMethod { method_id },
            cost: 100,
            is_implicit: false,
        }
    }
}

impl DataType {
    /// Check if this type can convert to the target type.
    ///
    /// Returns the conversion information if valid, including:
    /// - ConversionKind (what type of conversion)
    /// - Cost (for overload resolution)
    /// - Whether it's implicit or explicit only
    ///
    /// # Example
    ///
    /// ```ignore
    /// use angelscript::semantic::{DataType, INT32_TYPE, FLOAT_TYPE};
    /// # use angelscript::semantic::Registry;
    ///
    /// let int_type = DataType::simple(INT32_TYPE);
    /// let float_type = DataType::simple(FLOAT_TYPE);
    /// let registry = Registry::new();
    ///
    /// // int can implicitly convert to float
    /// let conv = int_type.can_convert_to(&float_type, &registry);
    /// assert!(conv.is_some());
    /// assert!(conv.unwrap().is_implicit);
    ///
    /// // float can only explicitly convert to int
    /// let conv = float_type.can_convert_to(&int_type, &registry);
    /// assert!(conv.is_some());
    /// assert!(!conv.unwrap().is_implicit);
    /// ```
    pub fn can_convert_to(&self, target: &DataType, registry: &Registry) -> Option<Conversion> {
        // Exact match - no conversion needed
        if self == target {
            return Some(Conversion::identity());
        }

        // Same base type with different const qualifiers (for non-handles)
        // e.g., const int -> int is allowed (reading a const value into a non-const variable)
        if !self.is_handle && !target.is_handle && self.type_id == target.type_id {
            // Same type_id means identity conversion, const doesn't matter for value types
            return Some(Conversion::identity());
        }

        // Null literal (NULL_TYPE) converts to any handle type
        if self.type_id == NULL_TYPE && target.is_handle {
            return Some(Conversion::null_to_handle());
        }

        // Try primitive conversions first (most common)
        if let Some(conv) = self.primitive_conversion(target) {
            return Some(conv);
        }

        // Try enum conversions (enum ↔ int)
        if let Some(conv) = self.enum_conversion(target, registry) {
            return Some(conv);
        }

        // Funcdef types are semantically always handles, so we should allow
        // conversions between handle and non-handle forms with the same type_id
        if self.type_id == target.type_id {
            let source_typedef = registry.get_type(self.type_id);
            if matches!(source_typedef, TypeDef::Funcdef { .. }) {
                // Same funcdef type with different handle flags - identity conversion
                return Some(Conversion::identity());
            }
        }

        // Value type to handle of same type (e.g., Node -> Node@)
        // In AngelScript, when you have `Node@ n = Node(args)`, the value is implicitly
        // converted to a handle reference. This is a common pattern for handle initialization.
        if !self.is_handle && target.is_handle && self.type_id == target.type_id {
            // Check if target is a class/object type (not primitive)
            // Note: Template instances are also Class types with template: Some(...)
            let typedef = registry.get_type(self.type_id);
            if matches!(typedef, TypeDef::Class { .. } | TypeDef::Interface { .. }) {
                return Some(Conversion {
                    kind: ConversionKind::ValueToHandle,
                    cost: 1,
                    is_implicit: true,
                });
            }
        }

        // Try handle conversions
        if let Some(conv) = self.handle_conversion(target, registry) {
            return Some(conv);
        }

        // Try user-defined conversions
        if let Some(conv) = self.user_defined_conversion(target, registry) {
            return Some(conv);
        }

        None
    }

    /// Check for enum ↔ integer conversions.
    /// In AngelScript, enums implicitly convert to/from their underlying integer type.
    fn enum_conversion(&self, target: &DataType, registry: &Registry) -> Option<Conversion> {
        // Don't convert handles
        if self.is_handle || target.is_handle {
            return None;
        }

        let source_typedef = registry.get_type(self.type_id);
        let target_typedef = registry.get_type(target.type_id);

        // Enum -> int (implicit) - enums are int32 internally, no conversion needed
        if source_typedef.is_enum() && target.type_id == INT32_TYPE {
            return Some(Conversion::identity());
        }

        // Int -> enum (implicit) - enums are int32 internally, no conversion needed
        if self.type_id == INT32_TYPE && target_typedef.is_enum() {
            return Some(Conversion::identity());
        }

        None
    }

    fn primitive_conversion(&self, target: &DataType) -> Option<Conversion> {
        // Only convert base types (no handles - those are separate rules)
        // Note: const-ness doesn't prevent primitive conversion. A const int can convert to float.
        // The const only affects mutability, not type compatibility.
        if self.is_handle || target.is_handle {
            return None;
        }

        // Match on type pairs
        use {
            DOUBLE_TYPE, FLOAT_TYPE, INT8_TYPE, INT16_TYPE, INT32_TYPE, INT64_TYPE, UINT8_TYPE,
            UINT16_TYPE, UINT32_TYPE, UINT64_TYPE,
        };

        match (self.type_id, target.type_id) {
            // Integer to Float conversions (implicit, cost 1)
            (INT8_TYPE, FLOAT_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (INT16_TYPE, FLOAT_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (INT32_TYPE, FLOAT_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (INT8_TYPE, DOUBLE_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (INT16_TYPE, DOUBLE_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (INT32_TYPE, DOUBLE_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),

            // int64 to float (implicit but higher cost - may lose precision)
            (INT64_TYPE, FLOAT_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT64_TYPE, DOUBLE_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),

            // Unsigned to Float conversions (implicit, cost 1)
            (UINT8_TYPE, FLOAT_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (UINT16_TYPE, FLOAT_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (UINT32_TYPE, FLOAT_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (UINT8_TYPE, DOUBLE_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (UINT16_TYPE, DOUBLE_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (UINT32_TYPE, DOUBLE_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),

            // uint64 to float (implicit but higher cost - may lose precision)
            (UINT64_TYPE, FLOAT_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT64_TYPE, DOUBLE_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),

            // Float to Integer conversions (implicit with higher cost - truncation)
            (FLOAT_TYPE, INT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 3, true)),
            (FLOAT_TYPE, INT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 3, true)),
            (FLOAT_TYPE, INT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 3, true)),
            (FLOAT_TYPE, INT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 3, true)),
            (DOUBLE_TYPE, INT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 3, true)),
            (DOUBLE_TYPE, INT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 3, true)),
            (DOUBLE_TYPE, INT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 3, true)),
            (DOUBLE_TYPE, INT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 3, true)),

            // Float to Unsigned conversions (implicit with higher cost)
            (FLOAT_TYPE, UINT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 3, true)),
            (FLOAT_TYPE, UINT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 3, true)),
            (FLOAT_TYPE, UINT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 3, true)),
            (FLOAT_TYPE, UINT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 3, true)),
            (DOUBLE_TYPE, UINT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 3, true)),
            (DOUBLE_TYPE, UINT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 3, true)),
            (DOUBLE_TYPE, UINT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 3, true)),
            (DOUBLE_TYPE, UINT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 3, true)),

            // Float ↔ Double conversions
            (FLOAT_TYPE, DOUBLE_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (DOUBLE_TYPE, FLOAT_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)), // Implicit but may lose precision

            // Integer widening (signed) - implicit
            (INT8_TYPE, INT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (INT8_TYPE, INT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (INT8_TYPE, INT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (INT16_TYPE, INT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (INT16_TYPE, INT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (INT32_TYPE, INT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),

            // Integer narrowing (signed) - implicit with higher cost (data loss possible)
            (INT64_TYPE, INT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT64_TYPE, INT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT64_TYPE, INT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT32_TYPE, INT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT32_TYPE, INT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT16_TYPE, INT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),

            // Unsigned widening - implicit
            (UINT8_TYPE, UINT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (UINT8_TYPE, UINT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (UINT8_TYPE, UINT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (UINT16_TYPE, UINT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (UINT16_TYPE, UINT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),
            (UINT32_TYPE, UINT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 1, true)),

            // Unsigned narrowing - implicit with higher cost (data loss possible)
            (UINT64_TYPE, UINT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT64_TYPE, UINT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT64_TYPE, UINT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT32_TYPE, UINT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT32_TYPE, UINT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT16_TYPE, UINT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),

            // Signed/Unsigned reinterpret (same size) - implicit with higher cost
            (INT8_TYPE, UINT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT16_TYPE, UINT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT32_TYPE, UINT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT64_TYPE, UINT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT8_TYPE, INT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT16_TYPE, INT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT32_TYPE, INT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT64_TYPE, INT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),

            // Signed to Unsigned (different sizes) - implicit with higher cost
            // int8 -> uint16, uint32, uint64
            (INT8_TYPE, UINT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT8_TYPE, UINT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT8_TYPE, UINT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            // int16 -> uint8, uint32, uint64
            (INT16_TYPE, UINT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT16_TYPE, UINT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT16_TYPE, UINT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            // int32 -> uint8, uint16, uint64
            (INT32_TYPE, UINT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT32_TYPE, UINT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT32_TYPE, UINT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            // int64 -> uint8, uint16, uint32
            (INT64_TYPE, UINT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT64_TYPE, UINT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (INT64_TYPE, UINT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),

            // Unsigned to Signed (different sizes) - implicit with higher cost
            // uint8 -> int16, int32, int64
            (UINT8_TYPE, INT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT8_TYPE, INT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT8_TYPE, INT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            // uint16 -> int8, int32, int64
            (UINT16_TYPE, INT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT16_TYPE, INT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT16_TYPE, INT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            // uint32 -> int8, int16, int64
            (UINT32_TYPE, INT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT32_TYPE, INT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT32_TYPE, INT64_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            // uint64 -> int8, int16, int32
            (UINT64_TYPE, INT8_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT64_TYPE, INT16_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),
            (UINT64_TYPE, INT32_TYPE) => Some(Conversion::primitive(self.type_id, target.type_id, 2, true)),

            // No conversion
            _ => None,
        }
    }


    /// Attempt handle type conversion.
    ///
    /// Handles conversions like T@ → const T@, T@ → T@ const, Derived@ → Base@, etc.
    ///
    /// AngelScript supports two independent const modifiers:
    /// - `is_const` - The handle itself is const (can't reassign)
    /// - `is_handle_to_const` - The object pointed to is const (can't modify)
    ///
    /// Returns None if not a handle conversion.
    fn handle_conversion(&self, target: &DataType, registry: &Registry) -> Option<Conversion> {
        // Both types must be handles for handle conversion
        if !self.is_handle || !target.is_handle {
            return None;
        }

        // Rule 1: Adding const qualifiers (T@ → const T@, T@ → T@ const, etc.)
        // This is safe for same-type conversions
        if self.type_id == target.type_id {
            // Check if we're only adding const (never removing)
            let adding_handle_const = !self.is_const && target.is_const;
            let adding_object_const = !self.is_handle_to_const && target.is_handle_to_const;
            let removing_handle_const = self.is_const && !target.is_const;
            let removing_object_const = self.is_handle_to_const && !target.is_handle_to_const;

            if adding_handle_const || adding_object_const {
                // Adding const is implicit and safe
                return Some(Conversion::handle_to_const());
            } else if removing_handle_const || removing_object_const {
                // Removing const requires explicit cast
                return Some(Conversion { kind: ConversionKind::HandleToConst, cost: 100, is_implicit: false });
            } else {
                // Same type_id handles with no const change - identity conversion
                // This handles cases where DataType fields like ref_modifier differ but type is same
                return Some(Conversion::identity());
            }
        }

        // Rule 2: Derived@ → Base@ (implicit if not removing const, cost 3)
        // Check if self is derived from target via inheritance chain
        if let Some(conv) = self.derived_to_base_conversion(target, registry) {
            return Some(conv);
        }

        // Rule 3: Class@ → Interface@ (implicit if not removing const, cost 5)
        // Check if self implements target interface
        if let Some(conv) = self.class_to_interface_conversion(target, registry) {
            return Some(conv);
        }

        // Rule 4: User-defined opCast/opImplCast conversions
        // These are checked in user_defined_conversion(), not here

        None
    }


    fn derived_to_base_conversion(&self, target: &DataType, registry: &Registry) -> Option<Conversion> {
        // Walk up the inheritance chain to find base class
        let mut current_type = self.type_id;

        loop {
            // Check if current_type is the target
            if current_type == target.type_id {
                // Found it! Now check const compatibility for BOTH const flags
                let adding_handle_const = !self.is_const && target.is_const;
                let adding_object_const = !self.is_handle_to_const && target.is_handle_to_const;
                let removing_handle_const = self.is_const && !target.is_const;
                let removing_object_const = self.is_handle_to_const && !target.is_handle_to_const;

                // Calculate cost based on const changes
                let cost = if removing_handle_const || removing_object_const {
                    // Trying to remove any const - not allowed implicitly
                    return Some(Conversion { kind: ConversionKind::DerivedToBase, cost: 100, is_implicit: false });
                } else if adding_handle_const || adding_object_const {
                    2 // Adding const (lower cost, more permissive)
                } else {
                    3 // No const change
                };

                return Some(Conversion { kind: ConversionKind::DerivedToBase, cost, is_implicit: true });
            }

            // Get the type definition to find base class
            let typedef = registry.get_type(current_type);
            let base_class = match typedef {
                TypeDef::Class { base_class, .. } => *base_class,
                _ => None,
            };

            // If no base class, we've reached the end
            match base_class {
                Some(base) => current_type = base,
                None => return None,
            }
        }
    }


    fn class_to_interface_conversion(&self, target: &DataType, registry: &Registry) -> Option<Conversion> {
        // Target must be an interface
        let target_typedef = registry.get_type(target.type_id);
        if !target_typedef.is_interface() {
            return None;
        }

        // Check if this class (or any base class) implements the target interface
        let mut current_type = self.type_id;

        loop {
            let typedef = registry.get_type(current_type);

            // Get interfaces this class implements
            let interfaces = match typedef {
                TypeDef::Class { interfaces, .. } => interfaces,
                _ => return None, // Not a class
            };

            // Check if target interface is in the list
            if interfaces.contains(&target.type_id) {
                // Found it! Check const compatibility for BOTH const flags
                let adding_handle_const = !self.is_const && target.is_const;
                let adding_object_const = !self.is_handle_to_const && target.is_handle_to_const;
                let removing_handle_const = self.is_const && !target.is_const;
                let removing_object_const = self.is_handle_to_const && !target.is_handle_to_const;

                // Calculate cost based on const changes
                let cost = if removing_handle_const || removing_object_const {
                    // Trying to remove any const - not allowed implicitly
                    return Some(Conversion { kind: ConversionKind::ClassToInterface, cost: 100, is_implicit: false });
                } else if adding_handle_const || adding_object_const {
                    4 // Adding const (slightly lower cost than base)
                } else {
                    5 // No const change
                };

                return Some(Conversion { kind: ConversionKind::ClassToInterface, cost, is_implicit: true });
            }

            // Try base class (base classes can also implement interfaces)
            let base_class = match typedef {
                TypeDef::Class { base_class, .. } => *base_class,
                _ => None,
            };

            match base_class {
                Some(base) => current_type = base,
                None => return None,
            }
        }
    }


    fn user_defined_conversion(
        &self,
        target: &DataType,
        registry: &Registry,
    ) -> Option<Conversion> {
        // For value types (not handles), try:
        // 1. Single-arg constructor (unless explicit)
        // 2. opImplConv() method on source type
        // 3. opConv() method on source type (explicit only)

        if !self.is_handle && !target.is_handle {
            // Try constructor conversion: TargetType(source_value)
            if let Some(conv) = self.constructor_conversion(target, registry) {
                return Some(conv);
            }

            // Try opImplConv/opConv on source type
            if let Some(conv) = self.value_operator_conversion(target, registry) {
                return Some(conv);
            }
        }

        // For handle types, try:
        // 1. opImplCast() method (implicit cast)
        // 2. opCast() method (explicit cast)

        if self.is_handle && target.is_handle
            && let Some(conv) = self.handle_operator_conversion(target, registry) {
                return Some(conv);
            }

        None
    }


    fn constructor_conversion(&self, target: &DataType, registry: &Registry) -> Option<Conversion> {
        // Get the target type definition
        let target_typedef = registry.get_type(target.type_id);

        // Only classes can have constructors
        if !target_typedef.is_class() {
            return None;
        }

        // Look for constructor with exactly one parameter matching our type
        let constructor_id = registry.find_constructor(target.type_id, &[self.clone()])?;

        // Check if the constructor is marked explicit
        // Explicit constructors cannot be used for implicit conversions
        let is_explicit = registry.is_constructor_explicit(constructor_id);

        if is_explicit {
            // Explicit constructors can only be used for explicit conversions
            return Some(Conversion::explicit_cast_method(constructor_id));
        }

        // Non-explicit constructor can be used for implicit conversion
        Some(Conversion::constructor(constructor_id))
    }


    fn value_operator_conversion(&self, target: &DataType, registry: &Registry) -> Option<Conversion> {
        // Get the source type definition
        let source_typedef = registry.get_type(self.type_id);

        // Only classes can have operator methods
        let operator_methods = match source_typedef {
            TypeDef::Class { operator_methods, .. } => operator_methods,
            _ => return None,
        };

        // Try opImplConv first (implicit conversion, cost 10)
        let implicit_behavior = OperatorBehavior::OpImplConv(target.type_id);
        if let Some(function_id) = operator_methods.get(&implicit_behavior).and_then(|v| v.first().copied()) {
            // Found implicit conversion operator
            return Some(Conversion::implicit_conv_method(function_id));
        }

        // Try opConv (explicit conversion, cost 100)
        let explicit_behavior = OperatorBehavior::OpConv(target.type_id);
        if let Some(function_id) = operator_methods.get(&explicit_behavior).and_then(|v| v.first().copied()) {
            return Some(Conversion::explicit_cast_method(function_id));
        }

        None
    }


    fn handle_operator_conversion(&self, target: &DataType, registry: &Registry) -> Option<Conversion> {
        // Get the source type definition
        let source_typedef = registry.get_type(self.type_id);

        // Only classes can have operator methods
        let operator_methods = match source_typedef {
            TypeDef::Class { operator_methods, .. } => operator_methods,
            _ => return None,
        };

        // Try opImplCast first (implicit cast, cost 10)
        let implicit_behavior = OperatorBehavior::OpImplCast(target.type_id);
        if let Some(function_id) = operator_methods.get(&implicit_behavior).and_then(|v| v.first().copied()) {
            return Some(Conversion::implicit_conv_method(function_id));
        }

        // Try opCast (explicit cast, cost 100)
        let explicit_behavior = OperatorBehavior::OpCast(target.type_id);
        if let Some(function_id) = operator_methods.get(&explicit_behavior).and_then(|v| v.first().copied()) {
            return Some(Conversion::explicit_cast_method(function_id));
        }

        None
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::types::type_def::{INT32_TYPE, VOID_TYPE, TypeId};

    // Test-local type ID to simulate a user type (like string)
    // This must be within the primitive type range for Registry::get_type to work without panicking
    // Since we're just testing conversion logic, we can use any valid TypeId
    const TEST_USER_TYPE: TypeId = TypeId(100);

    // ==================== Null Conversion Tests ====================

    #[test]
    fn null_literal_creation() {
        let null = DataType::null_literal();
        assert_eq!(null.type_id, NULL_TYPE);
        assert!(!null.is_handle);
        assert!(!null.is_const);
    }

    #[test]
    fn null_converts_to_any_handle() {
        let registry = Registry::new();
        let null = DataType::null_literal();

        // null -> int@ (handle to primitive)
        let int_handle = DataType::with_handle(INT32_TYPE, false);
        let conv = null.can_convert_to(&int_handle, &registry);
        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert_eq!(conv.kind, ConversionKind::NullToHandle);
        assert_eq!(conv.cost, 1);
        assert!(conv.is_implicit);

        // null -> const int@ (handle to const primitive)
        let const_int_handle = DataType::const_handle(INT32_TYPE, false);
        let conv = null.can_convert_to(&const_int_handle, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().kind, ConversionKind::NullToHandle);

        // null -> user_type@ (handle to user type)
        let user_handle = DataType::with_handle(TEST_USER_TYPE, false);
        let conv = null.can_convert_to(&user_handle, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().kind, ConversionKind::NullToHandle);
    }

    #[test]
    fn null_does_not_convert_to_non_handle() {
        use crate::semantic::types::type_def::TypeDef;

        let mut registry = Registry::new();
        let null = DataType::null_literal();

        // null cannot convert to value types
        let int_type = DataType::simple(INT32_TYPE);
        assert!(null.can_convert_to(&int_type, &registry).is_none());

        // Register a custom class to test
        let user_class = TypeDef::Class {
            name: "UserType".to_string(),
            qualified_name: "UserType".to_string(),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: crate::types::TypeKind::reference(),
            };
        let user_type_id = registry.register_type(user_class, Some("UserType"));

        let user_type = DataType::simple(user_type_id);
        assert!(null.can_convert_to(&user_type, &registry).is_none());
    }

    #[test]
    fn null_to_handle_conversion_creation() {
        let conv = Conversion::null_to_handle();
        assert_eq!(conv.kind, ConversionKind::NullToHandle);
        assert_eq!(conv.cost, 1);
        assert!(conv.is_implicit);
    }

    // ==================== Identity Conversion Tests ====================

    #[test]
    fn identity_conversion_same_type() {
        let registry = Registry::new();
        let int_type = DataType::simple(INT32_TYPE);

        let conv = int_type.can_convert_to(&int_type, &registry);
        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert_eq!(conv.kind, ConversionKind::Identity);
        assert_eq!(conv.cost, 0);
        assert!(conv.is_implicit);
    }

    #[test]
    fn identity_conversion_helper() {
        let conv = Conversion::identity();
        assert_eq!(conv.kind, ConversionKind::Identity);
        assert_eq!(conv.cost, 0);
        assert!(conv.is_implicit);
    }

    // ==================== Primitive Conversion Tests ====================

    #[test]
    fn primitive_int_to_float_implicit() {
        let registry = Registry::new();

        // int8 -> float
        let int8 = DataType::simple(INT8_TYPE);
        let float = DataType::simple(FLOAT_TYPE);
        let conv = int8.can_convert_to(&float, &registry);
        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(matches!(conv.kind, ConversionKind::Primitive { .. }));
        assert_eq!(conv.cost, 1);
        assert!(conv.is_implicit);

        // int16 -> float
        let int16 = DataType::simple(INT16_TYPE);
        let conv = int16.can_convert_to(&float, &registry);
        assert!(conv.is_some());
        assert!(conv.unwrap().is_implicit);

        // int32 -> float
        let int32 = DataType::simple(INT32_TYPE);
        let conv = int32.can_convert_to(&float, &registry);
        assert!(conv.is_some());
        assert!(conv.unwrap().is_implicit);

        // int64 -> float (higher cost due to precision loss)
        let int64 = DataType::simple(INT64_TYPE);
        let conv = int64.can_convert_to(&float, &registry);
        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert_eq!(conv.cost, 2);
        assert!(conv.is_implicit);
    }

    #[test]
    fn primitive_int_to_double_implicit() {
        let registry = Registry::new();
        let double = DataType::simple(DOUBLE_TYPE);

        // int8 -> double
        let int8 = DataType::simple(INT8_TYPE);
        let conv = int8.can_convert_to(&double, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);

        // int16 -> double
        let int16 = DataType::simple(INT16_TYPE);
        let conv = int16.can_convert_to(&double, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);

        // int32 -> double
        let int32 = DataType::simple(INT32_TYPE);
        let conv = int32.can_convert_to(&double, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);

        // int64 -> double
        let int64 = DataType::simple(INT64_TYPE);
        let conv = int64.can_convert_to(&double, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);
    }

    #[test]
    fn primitive_uint_to_float_implicit() {
        let registry = Registry::new();
        let float = DataType::simple(FLOAT_TYPE);

        // uint8 -> float
        let uint8 = DataType::simple(UINT8_TYPE);
        let conv = uint8.can_convert_to(&float, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);

        // uint16 -> float
        let uint16 = DataType::simple(UINT16_TYPE);
        let conv = uint16.can_convert_to(&float, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);

        // uint32 -> float
        let uint32 = DataType::simple(UINT32_TYPE);
        let conv = uint32.can_convert_to(&float, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);

        // uint64 -> float (higher cost)
        let uint64 = DataType::simple(UINT64_TYPE);
        let conv = uint64.can_convert_to(&float, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);
    }

    #[test]
    fn primitive_uint_to_double_implicit() {
        let registry = Registry::new();
        let double = DataType::simple(DOUBLE_TYPE);

        // uint8 -> double
        let uint8 = DataType::simple(UINT8_TYPE);
        let conv = uint8.can_convert_to(&double, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);

        // uint64 -> double
        let uint64 = DataType::simple(UINT64_TYPE);
        let conv = uint64.can_convert_to(&double, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);
    }

    #[test]
    fn primitive_float_to_int_implicit_truncation() {
        let registry = Registry::new();
        let float = DataType::simple(FLOAT_TYPE);

        // float -> int8
        let int8 = DataType::simple(INT8_TYPE);
        let conv = float.can_convert_to(&int8, &registry);
        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert_eq!(conv.cost, 3); // Higher cost for truncation
        assert!(conv.is_implicit);

        // float -> int32
        let int32 = DataType::simple(INT32_TYPE);
        let conv = float.can_convert_to(&int32, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 3);

        // float -> int64
        let int64 = DataType::simple(INT64_TYPE);
        let conv = float.can_convert_to(&int64, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 3);
    }

    #[test]
    fn primitive_double_to_int_implicit_truncation() {
        let registry = Registry::new();
        let double = DataType::simple(DOUBLE_TYPE);

        // double -> int32
        let int32 = DataType::simple(INT32_TYPE);
        let conv = double.can_convert_to(&int32, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 3);

        // double -> uint64
        let uint64 = DataType::simple(UINT64_TYPE);
        let conv = double.can_convert_to(&uint64, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 3);
    }

    #[test]
    fn primitive_float_double_conversion() {
        let registry = Registry::new();
        let float = DataType::simple(FLOAT_TYPE);
        let double = DataType::simple(DOUBLE_TYPE);

        // float -> double (widening, cost 1)
        let conv = float.can_convert_to(&double, &registry);
        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert_eq!(conv.cost, 1);
        assert!(conv.is_implicit);

        // double -> float (narrowing, higher cost)
        let conv = double.can_convert_to(&float, &registry);
        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert_eq!(conv.cost, 2);
        assert!(conv.is_implicit);
    }

    #[test]
    fn primitive_integer_widening() {
        let registry = Registry::new();

        // int8 -> int16 -> int32 -> int64
        let int8 = DataType::simple(INT8_TYPE);
        let int16 = DataType::simple(INT16_TYPE);
        let int32 = DataType::simple(INT32_TYPE);
        let int64 = DataType::simple(INT64_TYPE);

        let conv = int8.can_convert_to(&int16, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);

        let conv = int8.can_convert_to(&int32, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);

        let conv = int8.can_convert_to(&int64, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);

        let conv = int16.can_convert_to(&int32, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);

        let conv = int32.can_convert_to(&int64, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);
    }

    #[test]
    fn primitive_integer_narrowing() {
        let registry = Registry::new();

        // int64 -> int32 -> int16 -> int8
        let int8 = DataType::simple(INT8_TYPE);
        let int16 = DataType::simple(INT16_TYPE);
        let int32 = DataType::simple(INT32_TYPE);
        let int64 = DataType::simple(INT64_TYPE);

        let conv = int64.can_convert_to(&int32, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2); // Higher cost for narrowing

        let conv = int64.can_convert_to(&int16, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = int64.can_convert_to(&int8, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = int32.can_convert_to(&int16, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = int16.can_convert_to(&int8, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);
    }

    #[test]
    fn primitive_unsigned_widening() {
        let registry = Registry::new();

        let uint8 = DataType::simple(UINT8_TYPE);
        let uint16 = DataType::simple(UINT16_TYPE);
        let uint32 = DataType::simple(UINT32_TYPE);
        let uint64 = DataType::simple(UINT64_TYPE);

        let conv = uint8.can_convert_to(&uint16, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);

        let conv = uint8.can_convert_to(&uint32, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);

        let conv = uint16.can_convert_to(&uint64, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 1);
    }

    #[test]
    fn primitive_unsigned_narrowing() {
        let registry = Registry::new();

        let uint8 = DataType::simple(UINT8_TYPE);
        let uint16 = DataType::simple(UINT16_TYPE);
        let uint32 = DataType::simple(UINT32_TYPE);
        let uint64 = DataType::simple(UINT64_TYPE);

        let conv = uint64.can_convert_to(&uint32, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = uint32.can_convert_to(&uint8, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = uint16.can_convert_to(&uint8, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);
    }

    #[test]
    fn primitive_signed_unsigned_reinterpret() {
        let registry = Registry::new();

        // Same size reinterpret
        let int8 = DataType::simple(INT8_TYPE);
        let uint8 = DataType::simple(UINT8_TYPE);
        let conv = int8.can_convert_to(&uint8, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = uint8.can_convert_to(&int8, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let int32 = DataType::simple(INT32_TYPE);
        let uint32 = DataType::simple(UINT32_TYPE);
        let conv = int32.can_convert_to(&uint32, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let int64 = DataType::simple(INT64_TYPE);
        let uint64 = DataType::simple(UINT64_TYPE);
        let conv = int64.can_convert_to(&uint64, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);
    }

    #[test]
    fn primitive_signed_to_unsigned_different_sizes() {
        let registry = Registry::new();

        let int8 = DataType::simple(INT8_TYPE);
        let int16 = DataType::simple(INT16_TYPE);
        let int32 = DataType::simple(INT32_TYPE);
        let int64 = DataType::simple(INT64_TYPE);
        let uint8 = DataType::simple(UINT8_TYPE);
        let uint16 = DataType::simple(UINT16_TYPE);
        let uint32 = DataType::simple(UINT32_TYPE);
        let uint64 = DataType::simple(UINT64_TYPE);

        // int8 -> uint16, uint32, uint64
        let conv = int8.can_convert_to(&uint16, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = int8.can_convert_to(&uint32, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = int8.can_convert_to(&uint64, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        // int16 -> uint8, uint32, uint64
        let conv = int16.can_convert_to(&uint8, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = int16.can_convert_to(&uint32, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = int16.can_convert_to(&uint64, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        // int32 -> uint8, uint16, uint64
        let conv = int32.can_convert_to(&uint8, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = int32.can_convert_to(&uint16, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = int32.can_convert_to(&uint64, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        // int64 -> uint8, uint16, uint32
        let conv = int64.can_convert_to(&uint8, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = int64.can_convert_to(&uint16, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = int64.can_convert_to(&uint32, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);
    }

    #[test]
    fn primitive_unsigned_to_signed_different_sizes() {
        let registry = Registry::new();

        let int8 = DataType::simple(INT8_TYPE);
        let int16 = DataType::simple(INT16_TYPE);
        let int32 = DataType::simple(INT32_TYPE);
        let int64 = DataType::simple(INT64_TYPE);
        let uint8 = DataType::simple(UINT8_TYPE);
        let uint16 = DataType::simple(UINT16_TYPE);
        let uint32 = DataType::simple(UINT32_TYPE);
        let uint64 = DataType::simple(UINT64_TYPE);

        // uint8 -> int16, int32, int64
        let conv = uint8.can_convert_to(&int16, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = uint8.can_convert_to(&int32, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = uint8.can_convert_to(&int64, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        // uint16 -> int8, int32, int64
        let conv = uint16.can_convert_to(&int8, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = uint16.can_convert_to(&int32, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = uint16.can_convert_to(&int64, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        // uint32 -> int8, int16, int64
        let conv = uint32.can_convert_to(&int8, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = uint32.can_convert_to(&int16, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = uint32.can_convert_to(&int64, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        // uint64 -> int8, int16, int32
        let conv = uint64.can_convert_to(&int8, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = uint64.can_convert_to(&int16, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);

        let conv = uint64.can_convert_to(&int32, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().cost, 2);
    }

    #[test]
    fn primitive_no_conversion_void() {
        let registry = Registry::new();
        let int32 = DataType::simple(INT32_TYPE);
        let void_type = DataType::simple(VOID_TYPE);

        // Can't convert to/from void
        assert!(int32.can_convert_to(&void_type, &registry).is_none());
        assert!(void_type.can_convert_to(&int32, &registry).is_none());
    }

    #[test]
    fn primitive_conversion_helper() {
        let conv = Conversion::primitive(INT32_TYPE, FLOAT_TYPE, 1, true);
        assert!(matches!(conv.kind, ConversionKind::Primitive { from_type, to_type }
            if from_type == INT32_TYPE && to_type == FLOAT_TYPE));
        assert_eq!(conv.cost, 1);
        assert!(conv.is_implicit);
    }

    // ==================== Handle Conversion Tests ====================

    #[test]
    fn handle_to_const_handle_implicit() {
        let registry = Registry::new();

        // T@ -> const T@ (adding const is implicit)
        let handle = DataType::with_handle(INT32_TYPE, false);
        let const_handle = DataType::const_handle(INT32_TYPE, false);

        let conv = handle.can_convert_to(&const_handle, &registry);
        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert_eq!(conv.kind, ConversionKind::HandleToConst);
        assert_eq!(conv.cost, 2);
        assert!(conv.is_implicit);
    }

    #[test]
    fn const_handle_to_handle_explicit() {
        let registry = Registry::new();

        // const T@ -> T@ (removing const requires explicit cast)
        let const_handle = DataType::const_handle(INT32_TYPE, false);
        let handle = DataType::with_handle(INT32_TYPE, false);

        let conv = const_handle.can_convert_to(&handle, &registry);
        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert_eq!(conv.kind, ConversionKind::HandleToConst);
        assert_eq!(conv.cost, 100);
        assert!(!conv.is_implicit);
    }

    #[test]
    fn handle_to_const_conversion_helper() {
        let conv = Conversion::handle_to_const();
        assert_eq!(conv.kind, ConversionKind::HandleToConst);
        assert_eq!(conv.cost, 2);
        assert!(conv.is_implicit);
    }

    // ==================== Derived to Base Conversion Tests ====================

    #[test]
    fn derived_to_base_conversion_helper() {
        let conv = Conversion::derived_to_base();
        assert_eq!(conv.kind, ConversionKind::DerivedToBase);
        assert_eq!(conv.cost, 3);
        assert!(conv.is_implicit);
    }

    #[test]
    fn derived_to_base_with_registry() {
        let mut registry = Registry::new();

        // Register base class
        let base_id = registry.register_type(
            TypeDef::Class {
                name: "Base".to_string(),
                qualified_name: "Base".to_string(),
                base_class: None,
                interfaces: vec![],
                fields: vec![],
                methods: vec![],
                operator_methods: Default::default(),
                properties: Default::default(),
                is_abstract: false,
                is_final: false,
                template_params: Vec::new(),
                template: None,
                type_args: Vec::new(),
            type_kind: crate::types::TypeKind::reference(),
            },
            Some("Base"),
        );

        // Register derived class
        let derived_id = registry.register_type(
            TypeDef::Class {
                name: "Derived".to_string(),
                qualified_name: "Derived".to_string(),
                base_class: Some(base_id),
                interfaces: vec![],
                fields: vec![],
                methods: vec![],
                operator_methods: Default::default(),
                properties: Default::default(),
                is_abstract: false,
                is_final: false,
                template_params: Vec::new(),
                template: None,
                type_args: Vec::new(),
            type_kind: crate::types::TypeKind::reference(),
            },
            Some("Derived"),
        );

        // Derived@ -> Base@ should work
        let derived_handle = DataType::with_handle(derived_id, false);
        let base_handle = DataType::with_handle(base_id, false);

        let conv = derived_handle.can_convert_to(&base_handle, &registry);
        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert_eq!(conv.kind, ConversionKind::DerivedToBase);
        assert!(conv.is_implicit);

        // Base@ -> Derived@ should NOT work (no implicit downcast)
        let conv = base_handle.can_convert_to(&derived_handle, &registry);
        assert!(conv.is_none());
    }

    // ==================== Class to Interface Conversion Tests ====================

    #[test]
    fn class_to_interface_conversion_helper() {
        let conv = Conversion::class_to_interface();
        assert_eq!(conv.kind, ConversionKind::ClassToInterface);
        assert_eq!(conv.cost, 5);
        assert!(conv.is_implicit);
    }

    #[test]
    fn class_to_interface_with_registry() {
        let mut registry = Registry::new();

        // Register interface
        let interface_id = registry.register_type(
            TypeDef::Interface {
                name: "IDrawable".to_string(),
                qualified_name: "IDrawable".to_string(),
                methods: vec![],
            },
            Some("IDrawable"),
        );

        // Register class that implements interface
        let class_id = registry.register_type(
            TypeDef::Class {
                name: "Circle".to_string(),
                qualified_name: "Circle".to_string(),
                base_class: None,
                interfaces: vec![interface_id],
                fields: vec![],
                methods: vec![],
                operator_methods: Default::default(),
                properties: Default::default(),
                is_abstract: false,
                is_final: false,
                template_params: Vec::new(),
                template: None,
                type_args: Vec::new(),
            type_kind: crate::types::TypeKind::reference(),
            },
            Some("Circle"),
        );

        // Circle@ -> IDrawable@ should work
        let class_handle = DataType::with_handle(class_id, false);
        let interface_handle = DataType::with_handle(interface_id, false);

        let conv = class_handle.can_convert_to(&interface_handle, &registry);
        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert_eq!(conv.kind, ConversionKind::ClassToInterface);
        assert!(conv.is_implicit);

        // IDrawable@ -> Circle@ should NOT work
        let conv = interface_handle.can_convert_to(&class_handle, &registry);
        assert!(conv.is_none());
    }

    // ==================== User-Defined Conversion Tests ====================

    #[test]
    fn constructor_conversion_helper() {
        use crate::semantic::FunctionId;
        let func_id = FunctionId(42);
        let conv = Conversion::constructor(func_id);
        assert!(matches!(conv.kind, ConversionKind::ConstructorConversion { constructor_id }
            if constructor_id == func_id));
        assert_eq!(conv.cost, 10);
        assert!(conv.is_implicit);
    }

    #[test]
    fn implicit_conv_method_helper() {
        use crate::semantic::FunctionId;
        let func_id = FunctionId(99);
        let conv = Conversion::implicit_conv_method(func_id);
        assert!(matches!(conv.kind, ConversionKind::ImplicitConversionMethod { method_id }
            if method_id == func_id));
        assert_eq!(conv.cost, 10);
        assert!(conv.is_implicit);
    }

    #[test]
    fn explicit_cast_method_helper() {
        use crate::semantic::FunctionId;
        let func_id = FunctionId(100);
        let conv = Conversion::explicit_cast_method(func_id);
        assert!(matches!(conv.kind, ConversionKind::ExplicitCastMethod { method_id }
            if method_id == func_id));
        assert_eq!(conv.cost, 100);
        assert!(!conv.is_implicit);
    }

    #[test]
    fn implicit_cast_method_helper() {
        use crate::semantic::FunctionId;
        let func_id = FunctionId(101);
        let conv = Conversion::implicit_cast_method(func_id);
        assert!(matches!(conv.kind, ConversionKind::ImplicitCastMethod { method_id }
            if method_id == func_id));
        assert_eq!(conv.cost, 100);
        assert!(!conv.is_implicit);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn no_conversion_between_unrelated_types() {
        use crate::semantic::types::type_def::TypeDef;

        let mut registry = Registry::new();

        // Register a custom class type
        let user_class = TypeDef::Class {
            name: "UserType".to_string(),
            qualified_name: "UserType".to_string(),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: crate::types::TypeKind::reference(),
            };
        let user_type_id = registry.register_type(user_class, Some("UserType"));

        // user_type -> int (no conversion)
        let user_type = DataType::simple(user_type_id);
        let int_type = DataType::simple(INT32_TYPE);

        assert!(user_type.can_convert_to(&int_type, &registry).is_none());
        assert!(int_type.can_convert_to(&user_type, &registry).is_none());
    }

    #[test]
    fn no_primitive_conversion_for_handles() {
        let registry = Registry::new();

        // int@ cannot use primitive conversion rules
        let int_handle = DataType::with_handle(INT32_TYPE, false);
        let float_handle = DataType::with_handle(FLOAT_TYPE, false);

        // These are different handle types - no primitive conversion
        let conv = int_handle.can_convert_to(&float_handle, &registry);
        assert!(conv.is_none());
    }

    #[test]
    fn const_values_can_convert() {
        use crate::semantic::RefModifier;
        let registry = Registry::new();

        // const int -> float SHOULD work - const only affects mutability, not conversions
        let const_int = DataType {
            type_id: INT32_TYPE,
            is_handle: false,
            is_const: true,
            is_handle_to_const: false,
            ref_modifier: RefModifier::None,
        };
        let float = DataType::simple(FLOAT_TYPE);

        // Const values CAN participate in primitive conversion
        let conv = const_int.can_convert_to(&float, &registry);
        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(matches!(conv.kind, ConversionKind::Primitive { .. }));
        assert!(conv.is_implicit);
    }

    #[test]
    fn const_int_to_int_conversion() {
        use crate::semantic::RefModifier;
        let registry = Registry::new();

        // const int -> int should work (same base type, const doesn't matter for reading)
        let const_int = DataType {
            type_id: INT32_TYPE,
            is_handle: false,
            is_const: true,
            is_handle_to_const: false,
            ref_modifier: RefModifier::None,
        };
        let int = DataType::simple(INT32_TYPE);

        // This should work - same base type, const is ignored for value conversions
        let conv = const_int.can_convert_to(&int, &registry);
        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert_eq!(conv.kind, ConversionKind::Identity);
        assert!(conv.is_implicit);
    }

    #[test]
    fn conversion_kind_debug() {
        // Test that ConversionKind implements Debug correctly
        let kind = ConversionKind::Identity;
        let debug_str = format!("{:?}", kind);
        assert!(debug_str.contains("Identity"));

        let kind = ConversionKind::Primitive {
            from_type: INT32_TYPE,
            to_type: FLOAT_TYPE,
        };
        let debug_str = format!("{:?}", kind);
        assert!(debug_str.contains("Primitive"));
    }

    #[test]
    fn conversion_kind_clone() {
        let kind = ConversionKind::NullToHandle;
        let cloned = kind.clone();
        assert_eq!(kind, cloned);
    }

    #[test]
    fn conversion_struct_clone() {
        let conv = Conversion::identity();
        let cloned = conv.clone();
        assert_eq!(conv, cloned);
    }
}
