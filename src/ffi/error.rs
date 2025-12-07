//! Error types for the FFI system.

use std::any::TypeId;
use thiserror::Error;

/// Errors that can occur when converting between Rust and script values.
#[derive(Debug, Error)]
pub enum ConversionError {
    /// Type mismatch during conversion
    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        expected: &'static str,
        actual: &'static str,
    },

    /// Attempted to convert a null handle to a non-nullable type
    #[error("null handle cannot be converted to {target_type}")]
    NullHandle { target_type: &'static str },

    /// Integer overflow during conversion
    #[error("integer overflow: value {value} does not fit in {target_type}")]
    IntegerOverflow { value: i64, target_type: &'static str },

    /// Float conversion error
    #[error("float conversion error: value {value} cannot be represented as {target_type}")]
    FloatConversion {
        value: f64,
        target_type: &'static str,
    },

    /// Invalid UTF-8 in string
    #[error("invalid UTF-8 string data")]
    InvalidUtf8,

    /// Generic conversion failure
    #[error("conversion failed: {message}")]
    Failed { message: String },
}

/// Errors that can occur during native function execution.
#[derive(Debug, Error)]
pub enum NativeError {
    /// Error converting arguments or return values
    #[error("conversion error: {0}")]
    Conversion(#[from] ConversionError),

    /// Invalid `this` reference for method call
    #[error("invalid 'this' reference: {message}")]
    InvalidThis { message: String },

    /// Argument index out of bounds
    #[error("argument index {index} out of bounds (function has {count} arguments)")]
    ArgumentIndexOutOfBounds { index: usize, count: usize },

    /// Type mismatch for `this` reference
    #[error("'this' type mismatch: expected {expected:?}, got {actual:?}")]
    ThisTypeMismatch { expected: TypeId, actual: TypeId },

    /// Stale object handle (object was freed)
    #[error("stale object handle: object at index {index} has been freed")]
    StaleHandle { index: u32 },

    /// Native function panicked
    #[error("native function panicked: {message}")]
    Panic { message: String },

    /// Generic native error
    #[error("native error: {message}")]
    Other { message: String },
}

impl NativeError {
    /// Create an "invalid this" error with a message.
    pub fn invalid_this(message: impl Into<String>) -> Self {
        NativeError::InvalidThis {
            message: message.into(),
        }
    }

    /// Create a generic native error.
    pub fn other(message: impl Into<String>) -> Self {
        NativeError::Other {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_error_type_mismatch() {
        let err = ConversionError::TypeMismatch {
            expected: "int",
            actual: "string",
        };
        assert!(err.to_string().contains("type mismatch"));
        assert!(err.to_string().contains("int"));
        assert!(err.to_string().contains("string"));
    }

    #[test]
    fn conversion_error_null_handle() {
        let err = ConversionError::NullHandle {
            target_type: "MyClass",
        };
        assert!(err.to_string().contains("null handle"));
        assert!(err.to_string().contains("MyClass"));
    }

    #[test]
    fn conversion_error_integer_overflow() {
        let err = ConversionError::IntegerOverflow {
            value: 256,
            target_type: "int8",
        };
        assert!(err.to_string().contains("integer overflow"));
        assert!(err.to_string().contains("256"));
        assert!(err.to_string().contains("int8"));
    }

    #[test]
    fn conversion_error_float_conversion() {
        let err = ConversionError::FloatConversion {
            value: f64::INFINITY,
            target_type: "float",
        };
        assert!(err.to_string().contains("float conversion"));
    }

    #[test]
    fn conversion_error_invalid_utf8() {
        let err = ConversionError::InvalidUtf8;
        assert!(err.to_string().contains("UTF-8"));
    }

    #[test]
    fn conversion_error_failed() {
        let err = ConversionError::Failed {
            message: "custom error".to_string(),
        };
        assert!(err.to_string().contains("custom error"));
    }

    #[test]
    fn native_error_from_conversion() {
        let conv_err = ConversionError::InvalidUtf8;
        let native_err: NativeError = conv_err.into();
        assert!(matches!(native_err, NativeError::Conversion(_)));
    }

    #[test]
    fn native_error_invalid_this() {
        let err = NativeError::invalid_this("no this reference");
        assert!(err.to_string().contains("no this reference"));
    }

    #[test]
    fn native_error_argument_out_of_bounds() {
        let err = NativeError::ArgumentIndexOutOfBounds { index: 5, count: 3 };
        assert!(err.to_string().contains("5"));
        assert!(err.to_string().contains("3"));
    }

    #[test]
    fn native_error_this_type_mismatch() {
        let err = NativeError::ThisTypeMismatch {
            expected: TypeId::of::<i32>(),
            actual: TypeId::of::<String>(),
        };
        assert!(err.to_string().contains("type mismatch"));
    }

    #[test]
    fn native_error_stale_handle() {
        let err = NativeError::StaleHandle { index: 42 };
        assert!(err.to_string().contains("stale"));
        assert!(err.to_string().contains("42"));
    }

    #[test]
    fn native_error_panic() {
        let err = NativeError::Panic {
            message: "something went wrong".to_string(),
        };
        assert!(err.to_string().contains("panicked"));
    }

    #[test]
    fn native_error_other() {
        let err = NativeError::other("generic error");
        assert!(err.to_string().contains("generic error"));
    }
}
