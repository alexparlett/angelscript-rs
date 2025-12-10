//! Handle conversions.
//!
//! This module handles conversions involving handles (reference types):
//! - Null to any handle
//! - Handle to const handle
//! - Value to handle (explicit only)

use angelscript_core::DataType;

use super::{Conversion, ConversionKind};

/// Find handle-related conversions.
pub fn find_handle_conversion(source: &DataType, target: &DataType) -> Option<Conversion> {
    // Null to any handle type
    if source.is_null() && target.is_handle {
        return Some(Conversion {
            kind: ConversionKind::NullToHandle,
            cost: Conversion::COST_CONST_ADDITION,
            is_implicit: true,
        });
    }

    // Handle to const handle (same type)
    // T@ -> const T@ (handle to const handle)
    if source.type_hash == target.type_hash
        && source.is_handle
        && target.is_handle
        && !source.is_handle_to_const
        && target.is_handle_to_const
    {
        return Some(Conversion {
            kind: ConversionKind::HandleToConst,
            cost: Conversion::COST_CONST_ADDITION,
            is_implicit: true,
        });
    }

    // Value to handle (@value) - explicit only
    // T -> T@ (taking handle of value)
    if !source.is_handle && target.is_handle && source.type_hash == target.type_hash {
        return Some(Conversion {
            kind: ConversionKind::ValueToHandle,
            cost: Conversion::COST_EXPLICIT_ONLY,
            is_implicit: false,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{TypeHash, primitives};

    #[test]
    fn null_to_handle() {
        let player_hash = TypeHash::from_name("Player");
        let from = DataType::null_literal();
        let to = DataType::simple(player_hash).as_handle();
        let conv = find_handle_conversion(&from, &to);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
        assert!(matches!(conv.kind, ConversionKind::NullToHandle));
    }

    #[test]
    fn null_to_const_handle() {
        let player_hash = TypeHash::from_name("Player");
        let from = DataType::null_literal();
        let to = DataType::simple(player_hash).as_handle_to_const();
        let conv = find_handle_conversion(&from, &to);

        assert!(conv.is_some());
        assert!(conv.unwrap().is_implicit);
    }

    #[test]
    fn null_to_non_handle_fails() {
        let from = DataType::null_literal();
        let to = DataType::simple(primitives::INT32);
        let conv = find_handle_conversion(&from, &to);

        assert!(conv.is_none());
    }

    #[test]
    fn handle_to_const_handle() {
        let player_hash = TypeHash::from_name("Player");
        let from = DataType::simple(player_hash).as_handle();
        let to = DataType::simple(player_hash).as_handle_to_const();
        let conv = find_handle_conversion(&from, &to);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
        assert!(matches!(conv.kind, ConversionKind::HandleToConst));
        assert_eq!(conv.cost, Conversion::COST_CONST_ADDITION);
    }

    #[test]
    fn const_handle_to_handle_fails() {
        // Cannot remove const
        let player_hash = TypeHash::from_name("Player");
        let from = DataType::simple(player_hash).as_handle_to_const();
        let to = DataType::simple(player_hash).as_handle();
        let conv = find_handle_conversion(&from, &to);

        assert!(conv.is_none());
    }

    #[test]
    fn handle_to_different_type_fails() {
        let player_hash = TypeHash::from_name("Player");
        let enemy_hash = TypeHash::from_name("Enemy");
        let from = DataType::simple(player_hash).as_handle();
        let to = DataType::simple(enemy_hash).as_handle_to_const();
        let conv = find_handle_conversion(&from, &to);

        // This would require hierarchy conversion, not handled here
        assert!(conv.is_none());
    }

    #[test]
    fn value_to_handle_is_explicit() {
        let player_hash = TypeHash::from_name("Player");
        let from = DataType::simple(player_hash);
        let to = DataType::simple(player_hash).as_handle();
        let conv = find_handle_conversion(&from, &to);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(!conv.is_implicit); // Explicit only
        assert!(matches!(conv.kind, ConversionKind::ValueToHandle));
        assert_eq!(conv.cost, Conversion::COST_EXPLICIT_ONLY);
    }

    #[test]
    fn handle_to_value_fails() {
        // Cannot implicitly convert handle to value
        let player_hash = TypeHash::from_name("Player");
        let from = DataType::simple(player_hash).as_handle();
        let to = DataType::simple(player_hash);
        let conv = find_handle_conversion(&from, &to);

        assert!(conv.is_none());
    }

    #[test]
    fn same_handle_no_conversion() {
        // Same type handles need no conversion from this module
        let player_hash = TypeHash::from_name("Player");
        let from = DataType::simple(player_hash).as_handle();
        let to = DataType::simple(player_hash).as_handle();
        let conv = find_handle_conversion(&from, &to);

        // Identity handled at higher level
        assert!(conv.is_none());
    }
}
