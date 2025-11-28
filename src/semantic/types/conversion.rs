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

        // Null literal (NULL_TYPE) converts to any handle type
        if self.type_id == NULL_TYPE && target.is_handle {
            return Some(Conversion::null_to_handle());
        }

        // Try primitive conversions first (most common)
        if let Some(conv) = self.primitive_conversion(target) {
            return Some(conv);
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

    fn primitive_conversion(&self, target: &DataType) -> Option<Conversion> {
        // Only convert base types (no handles, no const - those are separate rules)
        if self.is_handle || target.is_handle || self.is_const || target.is_const {
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
            }
            // else: no const change, types are identical (handled by exact match earlier)
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

        if self.is_handle && target.is_handle {
            if let Some(conv) = self.handle_operator_conversion(target, registry) {
                return Some(conv);
            }
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
        if let Some(&function_id) = operator_methods.get(&implicit_behavior) {
            // Found implicit conversion operator
            return Some(Conversion::implicit_conv_method(function_id));
        }

        // Try opConv (explicit conversion, cost 100)
        let explicit_behavior = OperatorBehavior::OpConv(target.type_id);
        if let Some(&function_id) = operator_methods.get(&explicit_behavior) {
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
        if let Some(&function_id) = operator_methods.get(&implicit_behavior) {
            return Some(Conversion::implicit_conv_method(function_id));
        }

        // Try opCast (explicit cast, cost 100)
        let explicit_behavior = OperatorBehavior::OpCast(target.type_id);
        if let Some(&function_id) = operator_methods.get(&explicit_behavior) {
            return Some(Conversion::explicit_cast_method(function_id));
        }

        None
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::types::type_def::{INT32_TYPE, STRING_TYPE, VOID_TYPE};

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

        // null -> string@ (handle to string)
        let string_handle = DataType::with_handle(STRING_TYPE, false);
        let conv = null.can_convert_to(&string_handle, &registry);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().kind, ConversionKind::NullToHandle);
    }

    #[test]
    fn null_does_not_convert_to_non_handle() {
        let registry = Registry::new();
        let null = DataType::null_literal();

        // null cannot convert to value types
        let int_type = DataType::simple(INT32_TYPE);
        assert!(null.can_convert_to(&int_type, &registry).is_none());

        let string_type = DataType::simple(STRING_TYPE);
        assert!(null.can_convert_to(&string_type, &registry).is_none());
    }

    #[test]
    fn null_to_handle_conversion_creation() {
        let conv = Conversion::null_to_handle();
        assert_eq!(conv.kind, ConversionKind::NullToHandle);
        assert_eq!(conv.cost, 1);
        assert!(conv.is_implicit);
    }
}
