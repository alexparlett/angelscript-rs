//! Primitive type conversions.
//!
//! This module handles conversions between primitive types (integers, floats, bool).

use angelscript_core::{DataType, TypeHash, primitives};

use super::{Conversion, ConversionKind};

/// Find primitive type conversion.
pub fn find_primitive_conversion(source: &DataType, target: &DataType) -> Option<Conversion> {
    let from = source.type_hash;
    let to = target.type_hash;

    // Identity conversion (same type) - cheapest, no operation needed
    if from == to && is_primitive_numeric(from) {
        return Some(Conversion {
            kind: ConversionKind::Identity,
            cost: Conversion::COST_IDENTITY,
            is_implicit: true,
        });
    }

    // Integer widening (always implicit)
    if is_integer_widening(from, to) {
        return Some(Conversion {
            kind: ConversionKind::Primitive { from, to },
            cost: Conversion::COST_PRIMITIVE_WIDENING,
            is_implicit: true,
        });
    }

    // Integer narrowing (implicit but higher cost in AngelScript)
    if is_integer_narrowing(from, to) {
        return Some(Conversion {
            kind: ConversionKind::Primitive { from, to },
            cost: Conversion::COST_PRIMITIVE_NARROWING,
            is_implicit: true, // AngelScript allows implicit narrowing
        });
    }

    // Sign conversions (same size, different signedness)
    if let Some(cost) = get_sign_conversion_cost(from, to) {
        return Some(Conversion {
            kind: ConversionKind::Primitive { from, to },
            cost,
            is_implicit: true,
        });
    }

    // Integer to float (implicit, but lower priority than sign conversions)
    if is_int_to_float(from, to) {
        return Some(Conversion {
            kind: ConversionKind::Primitive { from, to },
            cost: Conversion::COST_INT_TO_FLOAT,
            is_implicit: true,
        });
    }

    // Float to integer (implicit with truncation, lowest priority for primitives)
    if is_float_to_int(from, to) {
        return Some(Conversion {
            kind: ConversionKind::Primitive { from, to },
            cost: Conversion::COST_FLOAT_TO_INT,
            is_implicit: true,
        });
    }

    // Float widening (float -> double)
    if from == primitives::FLOAT && to == primitives::DOUBLE {
        return Some(Conversion {
            kind: ConversionKind::Primitive { from, to },
            cost: Conversion::COST_PRIMITIVE_WIDENING,
            is_implicit: true,
        });
    }

    // Float narrowing (double -> float)
    if from == primitives::DOUBLE && to == primitives::FLOAT {
        return Some(Conversion {
            kind: ConversionKind::Primitive { from, to },
            cost: Conversion::COST_PRIMITIVE_NARROWING,
            is_implicit: true,
        });
    }

    // Catch-all: any numeric to any other numeric is allowed
    // This handles edge cases not covered above (e.g., cross-signed narrowing)
    if is_primitive_numeric(from) && is_primitive_numeric(to) {
        return Some(Conversion {
            kind: ConversionKind::Primitive { from, to },
            cost: Conversion::COST_PRIMITIVE_NARROWING, // Use narrowing cost as conservative default
            is_implicit: true,
        });
    }

    None
}

fn is_integer_widening(from: TypeHash, to: TypeHash) -> bool {
    matches!(
        (from, to),
        // Signed widening
        (primitives::INT8, primitives::INT16)
            | (primitives::INT8, primitives::INT32)
            | (primitives::INT8, primitives::INT64)
            | (primitives::INT16, primitives::INT32)
            | (primitives::INT16, primitives::INT64)
            | (primitives::INT32, primitives::INT64)
            // Unsigned widening
            | (primitives::UINT8, primitives::UINT16)
            | (primitives::UINT8, primitives::UINT32)
            | (primitives::UINT8, primitives::UINT64)
            | (primitives::UINT16, primitives::UINT32)
            | (primitives::UINT16, primitives::UINT64)
            | (primitives::UINT32, primitives::UINT64)
            // Unsigned to larger signed
            | (primitives::UINT8, primitives::INT16)
            | (primitives::UINT8, primitives::INT32)
            | (primitives::UINT8, primitives::INT64)
            | (primitives::UINT16, primitives::INT32)
            | (primitives::UINT16, primitives::INT64)
            | (primitives::UINT32, primitives::INT64)
    )
}

fn is_integer_narrowing(from: TypeHash, to: TypeHash) -> bool {
    matches!(
        (from, to),
        // Signed narrowing
        (primitives::INT64, primitives::INT32)
            | (primitives::INT64, primitives::INT16)
            | (primitives::INT64, primitives::INT8)
            | (primitives::INT32, primitives::INT16)
            | (primitives::INT32, primitives::INT8)
            | (primitives::INT16, primitives::INT8)
            // Unsigned narrowing
            | (primitives::UINT64, primitives::UINT32)
            | (primitives::UINT64, primitives::UINT16)
            | (primitives::UINT64, primitives::UINT8)
            | (primitives::UINT32, primitives::UINT16)
            | (primitives::UINT32, primitives::UINT8)
            | (primitives::UINT16, primitives::UINT8)
    )
}

/// Get the cost for sign conversion (same size, different signedness).
/// Returns None if not a sign conversion.
fn get_sign_conversion_cost(from: TypeHash, to: TypeHash) -> Option<u32> {
    // Signed to unsigned (same size)
    if matches!(
        (from, to),
        (primitives::INT8, primitives::UINT8)
            | (primitives::INT16, primitives::UINT16)
            | (primitives::INT32, primitives::UINT32)
            | (primitives::INT64, primitives::UINT64)
    ) {
        return Some(Conversion::COST_SIGNED_TO_UNSIGNED);
    }

    // Unsigned to signed (same size)
    if matches!(
        (from, to),
        (primitives::UINT8, primitives::INT8)
            | (primitives::UINT16, primitives::INT16)
            | (primitives::UINT32, primitives::INT32)
            | (primitives::UINT64, primitives::INT64)
    ) {
        return Some(Conversion::COST_UNSIGNED_TO_SIGNED);
    }

    None
}

fn is_int_to_float(from: TypeHash, to: TypeHash) -> bool {
    let is_int = matches!(
        from,
        primitives::INT8
            | primitives::INT16
            | primitives::INT32
            | primitives::INT64
            | primitives::UINT8
            | primitives::UINT16
            | primitives::UINT32
            | primitives::UINT64
    );
    let is_float = matches!(to, primitives::FLOAT | primitives::DOUBLE);
    is_int && is_float
}

fn is_float_to_int(from: TypeHash, to: TypeHash) -> bool {
    let is_float = matches!(from, primitives::FLOAT | primitives::DOUBLE);
    let is_int = matches!(
        to,
        primitives::INT8
            | primitives::INT16
            | primitives::INT32
            | primitives::INT64
            | primitives::UINT8
            | primitives::UINT16
            | primitives::UINT32
            | primitives::UINT64
    );
    is_float && is_int
}

/// Check if a type hash is a primitive numeric type.
pub fn is_primitive_numeric(hash: TypeHash) -> bool {
    matches!(
        hash,
        primitives::INT8
            | primitives::INT16
            | primitives::INT32
            | primitives::INT64
            | primitives::UINT8
            | primitives::UINT16
            | primitives::UINT32
            | primitives::UINT64
            | primitives::FLOAT
            | primitives::DOUBLE
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_widening_int8_to_int16() {
        let from = DataType::simple(primitives::INT8);
        let to = DataType::simple(primitives::INT16);
        let conv = find_primitive_conversion(&from, &to);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
        assert_eq!(conv.cost, Conversion::COST_PRIMITIVE_WIDENING);
    }

    #[test]
    fn integer_widening_int32_to_int64() {
        let from = DataType::simple(primitives::INT32);
        let to = DataType::simple(primitives::INT64);
        let conv = find_primitive_conversion(&from, &to);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
    }

    #[test]
    fn integer_narrowing_int64_to_int32() {
        let from = DataType::simple(primitives::INT64);
        let to = DataType::simple(primitives::INT32);
        let conv = find_primitive_conversion(&from, &to);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit); // AngelScript allows implicit narrowing
        assert_eq!(conv.cost, Conversion::COST_PRIMITIVE_NARROWING);
    }

    #[test]
    fn int_to_float() {
        let from = DataType::simple(primitives::INT32);
        let to = DataType::simple(primitives::FLOAT);
        let conv = find_primitive_conversion(&from, &to);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
    }

    #[test]
    fn int_to_double() {
        let from = DataType::simple(primitives::INT32);
        let to = DataType::simple(primitives::DOUBLE);
        let conv = find_primitive_conversion(&from, &to);

        assert!(conv.is_some());
        assert!(conv.unwrap().is_implicit);
    }

    #[test]
    fn float_to_int() {
        let from = DataType::simple(primitives::FLOAT);
        let to = DataType::simple(primitives::INT32);
        let conv = find_primitive_conversion(&from, &to);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
        // Higher cost due to truncation
        assert!(conv.cost > Conversion::COST_PRIMITIVE_NARROWING);
    }

    #[test]
    fn float_to_double() {
        let from = DataType::simple(primitives::FLOAT);
        let to = DataType::simple(primitives::DOUBLE);
        let conv = find_primitive_conversion(&from, &to);

        assert!(conv.is_some());
        assert!(conv.unwrap().is_implicit);
    }

    #[test]
    fn double_to_float() {
        let from = DataType::simple(primitives::DOUBLE);
        let to = DataType::simple(primitives::FLOAT);
        let conv = find_primitive_conversion(&from, &to);

        assert!(conv.is_some());
        assert!(conv.unwrap().is_implicit);
    }

    #[test]
    fn unsigned_to_larger_signed() {
        let from = DataType::simple(primitives::UINT8);
        let to = DataType::simple(primitives::INT16);
        let conv = find_primitive_conversion(&from, &to);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
        assert_eq!(conv.cost, Conversion::COST_PRIMITIVE_WIDENING);
    }

    #[test]
    fn cross_sign_same_size() {
        let from = DataType::simple(primitives::INT32);
        let to = DataType::simple(primitives::UINT32);
        let conv = find_primitive_conversion(&from, &to);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
        // Sign conversions have their own cost, higher than narrowing
        assert_eq!(conv.cost, Conversion::COST_SIGNED_TO_UNSIGNED);
    }

    #[test]
    fn identity_conversion_for_same_type() {
        let from = DataType::simple(primitives::INT32);
        let to = DataType::simple(primitives::INT32);
        let conv = find_primitive_conversion(&from, &to);

        // Identity conversion is the cheapest (cost 0)
        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
        assert_eq!(conv.cost, Conversion::COST_IDENTITY);
        assert!(matches!(conv.kind, ConversionKind::Identity));
    }

    #[test]
    fn no_conversion_for_non_primitives() {
        let player_hash = TypeHash::from_name("Player");
        let from = DataType::simple(player_hash);
        let to = DataType::simple(primitives::INT32);
        let conv = find_primitive_conversion(&from, &to);

        assert!(conv.is_none());
    }

    #[test]
    fn is_primitive_numeric_works() {
        assert!(is_primitive_numeric(primitives::INT32));
        assert!(is_primitive_numeric(primitives::FLOAT));
        assert!(is_primitive_numeric(primitives::DOUBLE));
        assert!(!is_primitive_numeric(primitives::BOOL));
        assert!(!is_primitive_numeric(primitives::VOID));
        assert!(!is_primitive_numeric(TypeHash::from_name("Player")));
    }

    // =========================================================================
    // Catch-all numeric conversion tests
    // =========================================================================

    #[test]
    fn catch_all_int64_to_uint8() {
        // Cross-signed narrowing: int64 -> uint8
        // Not explicitly covered by widening/narrowing/sign rules
        let from = DataType::simple(primitives::INT64);
        let to = DataType::simple(primitives::UINT8);
        let conv = find_primitive_conversion(&from, &to);

        assert!(
            conv.is_some(),
            "int64 to uint8 should be allowed via catch-all"
        );
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
        assert_eq!(conv.cost, Conversion::COST_PRIMITIVE_NARROWING);
    }

    #[test]
    fn catch_all_uint64_to_int8() {
        // Cross-signed narrowing: uint64 -> int8
        let from = DataType::simple(primitives::UINT64);
        let to = DataType::simple(primitives::INT8);
        let conv = find_primitive_conversion(&from, &to);

        assert!(
            conv.is_some(),
            "uint64 to int8 should be allowed via catch-all"
        );
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
    }

    #[test]
    fn catch_all_int16_to_uint32() {
        // Cross-signed: int16 -> uint32
        let from = DataType::simple(primitives::INT16);
        let to = DataType::simple(primitives::UINT32);
        let conv = find_primitive_conversion(&from, &to);

        assert!(conv.is_some(), "int16 to uint32 should be allowed");
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
    }

    #[test]
    fn catch_all_uint16_to_int8() {
        // Cross-signed narrowing: uint16 -> int8
        let from = DataType::simple(primitives::UINT16);
        let to = DataType::simple(primitives::INT8);
        let conv = find_primitive_conversion(&from, &to);

        assert!(
            conv.is_some(),
            "uint16 to int8 should be allowed via catch-all"
        );
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
    }

    #[test]
    fn identity_all_numeric_types() {
        // Verify identity works for all numeric types
        let types = [
            primitives::INT8,
            primitives::INT16,
            primitives::INT32,
            primitives::INT64,
            primitives::UINT8,
            primitives::UINT16,
            primitives::UINT32,
            primitives::UINT64,
            primitives::FLOAT,
            primitives::DOUBLE,
        ];

        for ty in types {
            let from = DataType::simple(ty);
            let to = DataType::simple(ty);
            let conv = find_primitive_conversion(&from, &to);

            assert!(conv.is_some(), "Identity for {:?} should work", ty);
            let conv = conv.unwrap();
            assert_eq!(
                conv.cost,
                Conversion::COST_IDENTITY,
                "Identity should be cheapest for {:?}",
                ty
            );
            assert!(matches!(conv.kind, ConversionKind::Identity));
        }
    }

    #[test]
    fn double_to_int8() {
        // Extreme conversion: double -> int8
        let from = DataType::simple(primitives::DOUBLE);
        let to = DataType::simple(primitives::INT8);
        let conv = find_primitive_conversion(&from, &to);

        assert!(conv.is_some(), "double to int8 should be allowed");
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
        // float_to_int has its own cost
        assert_eq!(conv.cost, Conversion::COST_FLOAT_TO_INT);
    }

    #[test]
    fn int8_to_double() {
        // Extreme widening: int8 -> double
        let from = DataType::simple(primitives::INT8);
        let to = DataType::simple(primitives::DOUBLE);
        let conv = find_primitive_conversion(&from, &to);

        assert!(conv.is_some(), "int8 to double should be allowed");
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
        // int_to_float has its own cost
        assert_eq!(conv.cost, Conversion::COST_INT_TO_FLOAT);
    }
}
