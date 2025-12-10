//! Global property entry types.
//!
//! This module provides types for global properties registered with the engine:
//!
//! - [`ConstantValue`] - Primitive constant values stored directly
//! - [`GlobalPropertyEntry`] - Complete global property entry
//! - [`GlobalPropertyImpl`] - How the property value is stored/accessed

use crate::{DataType, TypeHash, TypeSource};

/// Primitive constant values stored directly in the registry.
///
/// These are immutable values that can be inlined by the compiler.
/// For non-primitive constants (like `Vec3::ZERO`), use `Arc<RwLock<T>>`
/// with the `.const_()` modifier instead.
///
/// # Example
///
/// ```
/// use angelscript_core::ConstantValue;
///
/// let pi = ConstantValue::Double(std::f64::consts::PI);
/// let max_players = ConstantValue::Int32(64);
/// let enabled = ConstantValue::Bool(true);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstantValue {
    /// Boolean constant
    Bool(bool),
    /// 8-bit signed integer
    Int8(i8),
    /// 16-bit signed integer
    Int16(i16),
    /// 32-bit signed integer
    Int32(i32),
    /// 64-bit signed integer
    Int64(i64),
    /// 8-bit unsigned integer
    Uint8(u8),
    /// 16-bit unsigned integer
    Uint16(u16),
    /// 32-bit unsigned integer
    Uint32(u32),
    /// 64-bit unsigned integer
    Uint64(u64),
    /// 32-bit floating point
    Float(f32),
    /// 64-bit floating point
    Double(f64),
}

impl ConstantValue {
    /// Get the data type for this constant value.
    pub fn data_type(&self) -> DataType {
        use crate::primitives;
        let hash = match self {
            ConstantValue::Bool(_) => primitives::BOOL,
            ConstantValue::Int8(_) => primitives::INT8,
            ConstantValue::Int16(_) => primitives::INT16,
            ConstantValue::Int32(_) => primitives::INT32,
            ConstantValue::Int64(_) => primitives::INT64,
            ConstantValue::Uint8(_) => primitives::UINT8,
            ConstantValue::Uint16(_) => primitives::UINT16,
            ConstantValue::Uint32(_) => primitives::UINT32,
            ConstantValue::Uint64(_) => primitives::UINT64,
            ConstantValue::Float(_) => primitives::FLOAT,
            ConstantValue::Double(_) => primitives::DOUBLE,
        };
        DataType::with_const(hash)
    }

    /// Get the type hash for this constant value.
    pub fn type_hash(&self) -> TypeHash {
        self.data_type().type_hash
    }
}

/// A global property registered with the engine.
///
/// Global properties can be:
/// - **Constants**: Immutable primitive values like `math::PI`
/// - **Mutable FFI**: Shared state via `Arc<RwLock<T>>`
/// - **Script globals**: Variables declared at script module scope
#[derive(Debug)]
pub struct GlobalPropertyEntry {
    /// Simple name (e.g., "PI")
    pub name: String,
    /// Namespace path (e.g., `["math"]`).
    pub namespace: Vec<String>,
    /// Qualified name including namespace (e.g., "math::PI")
    pub qualified_name: String,
    /// Type hash for lookup
    pub type_hash: TypeHash,
    /// Complete data type with modifiers
    pub data_type: DataType,
    /// Whether the object is const (for reference types: `const T@ const`)
    pub is_const: bool,
    /// Where this property was defined
    pub source: TypeSource,
    /// How the property value is stored/accessed
    pub implementation: GlobalPropertyImpl,
}

impl GlobalPropertyEntry {
    /// Create a new constant global property in the global namespace.
    pub fn constant(name: impl Into<String>, value: ConstantValue) -> Self {
        let name = name.into();
        let data_type = value.data_type();
        Self {
            name: name.clone(),
            namespace: Vec::new(),
            qualified_name: name.clone(),
            type_hash: TypeHash::from_name(&name),
            data_type,
            is_const: true,
            source: TypeSource::ffi_untyped(),
            implementation: GlobalPropertyImpl::Constant(value),
        }
    }

    /// Set the namespace and update qualified name.
    pub fn with_namespace(mut self, namespace: Vec<String>) -> Self {
        self.namespace = namespace.clone();
        if namespace.is_empty() {
            self.qualified_name = self.name.clone();
        } else {
            self.qualified_name = format!("{}::{}", namespace.join("::"), self.name);
        }
        self.type_hash = TypeHash::from_name(&self.qualified_name);
        self
    }

    /// Set the qualified name (including namespace).
    #[deprecated(note = "Use with_namespace instead for consistency")]
    pub fn with_qualified_name(mut self, qualified_name: impl Into<String>) -> Self {
        self.qualified_name = qualified_name.into();
        self.type_hash = TypeHash::from_name(&self.qualified_name);
        self
    }
}

/// How a global property value is stored and accessed.
#[derive(Debug)]
pub enum GlobalPropertyImpl {
    /// Constant value (primitives only).
    ///
    /// These values are immutable and can be inlined by the compiler.
    Constant(ConstantValue),

    /// Mutable FFI property via `Arc<RwLock<T>>`.
    ///
    /// The accessor provides type-erased read/write access.
    Mutable(Box<dyn GlobalPropertyAccessor>),

    /// Script-declared global variable.
    ///
    /// The slot index refers to the module's global variable table.
    Script {
        /// Slot index in the module's global table
        slot: u32,
        /// The data type of this script global
        data_type: DataType,
    },
}

impl GlobalPropertyImpl {
    /// Get the data type for this property implementation.
    pub fn data_type(&self) -> DataType {
        match self {
            GlobalPropertyImpl::Constant(value) => value.data_type(),
            GlobalPropertyImpl::Mutable(accessor) => accessor.data_type(),
            GlobalPropertyImpl::Script { data_type, .. } => *data_type,
        }
    }
}

/// Type-erased accessor for mutable global properties.
///
/// This trait is implemented for `Arc<RwLock<T>>` to provide
/// read/write access to global property values.
pub trait GlobalPropertyAccessor: Send + Sync + std::fmt::Debug {
    /// Get the data type of this property.
    fn data_type(&self) -> DataType;

    /// Read the current value as a type-erased box.
    fn read(&self) -> Box<dyn std::any::Any + Send + Sync>;

    /// Write a new value from a type-erased box.
    ///
    /// Returns an error if the type doesn't match.
    fn write(&self, value: Box<dyn std::any::Any + Send + Sync>) -> Result<(), PropertyError>;
}

/// Errors that can occur when accessing global properties.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PropertyError {
    /// Type mismatch when writing a value.
    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        /// Expected type name
        expected: &'static str,
        /// Actual type name
        actual: &'static str,
    },

    /// Failed to acquire lock (would block or poisoned).
    #[error("failed to acquire lock")]
    LockFailed,
}

// ============================================================================
// IntoGlobalProperty Trait
// ============================================================================

use std::sync::{Arc, RwLock};

/// Trait for types that can be registered as global properties.
///
/// This trait provides a unified way to convert different Rust types into
/// `GlobalPropertyImpl`:
/// - Primitives (i32, f64, bool, etc.) → `Constant` (inherently immutable)
/// - `Arc<RwLock<T>>` → `Mutable` (shared state with script)
///
/// # Example
///
/// ```ignore
/// use angelscript_core::IntoGlobalProperty;
///
/// // Primitives are inherently const
/// let pi_impl = 3.14159f64.into_global_impl();
/// assert!(f64::is_inherently_const());
///
/// // Arc<RwLock<T>> is mutable
/// let score = Arc::new(RwLock::new(0i32));
/// let score_impl = score.into_global_impl();
/// assert!(!Arc::<RwLock<i32>>::is_inherently_const());
/// ```
pub trait IntoGlobalProperty {
    /// Convert this value into a global property implementation.
    fn into_global_impl(self) -> GlobalPropertyImpl;

    /// Whether this type is inherently constant.
    ///
    /// Primitives are inherently const (immutable values).
    /// `Arc<RwLock<T>>` is not (mutable shared state).
    fn is_inherently_const() -> bool;
}

// ============================================================================
// Primitive Implementations (via macro)
// ============================================================================

/// Helper macro to implement IntoGlobalProperty for primitive types.
macro_rules! impl_into_global_property_primitive {
    ($rust_ty:ty, $variant:ident) => {
        impl IntoGlobalProperty for $rust_ty {
            fn into_global_impl(self) -> GlobalPropertyImpl {
                GlobalPropertyImpl::Constant(ConstantValue::$variant(self))
            }

            fn is_inherently_const() -> bool {
                true
            }
        }
    };
}

impl_into_global_property_primitive!(bool, Bool);
impl_into_global_property_primitive!(i8, Int8);
impl_into_global_property_primitive!(i16, Int16);
impl_into_global_property_primitive!(i32, Int32);
impl_into_global_property_primitive!(i64, Int64);
impl_into_global_property_primitive!(u8, Uint8);
impl_into_global_property_primitive!(u16, Uint16);
impl_into_global_property_primitive!(u32, Uint32);
impl_into_global_property_primitive!(u64, Uint64);
impl_into_global_property_primitive!(f32, Float);
impl_into_global_property_primitive!(f64, Double);

// ============================================================================
// Arc<RwLock<T>> Implementation
// ============================================================================

impl<T> IntoGlobalProperty for Arc<RwLock<T>>
where
    T: crate::Any + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    fn into_global_impl(self) -> GlobalPropertyImpl {
        GlobalPropertyImpl::Mutable(Box::new(self))
    }

    fn is_inherently_const() -> bool {
        false
    }
}

// ============================================================================
// GlobalPropertyAccessor for Arc<RwLock<T>>
// ============================================================================

impl<T> GlobalPropertyAccessor for Arc<RwLock<T>>
where
    T: crate::Any + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    fn data_type(&self) -> DataType {
        DataType::simple(T::type_hash())
    }

    fn read(&self) -> Box<dyn std::any::Any + Send + Sync> {
        // Clone the value out of the lock
        let guard = RwLock::read(self).expect("RwLock poisoned");
        Box::new((*guard).clone())
    }

    fn write(&self, value: Box<dyn std::any::Any + Send + Sync>) -> Result<(), PropertyError> {
        let typed = value.downcast::<T>().map_err(|_| PropertyError::TypeMismatch {
            expected: T::type_name(),
            actual: "unknown",
        })?;
        let mut guard = RwLock::write(self).map_err(|_| PropertyError::LockFailed)?;
        *guard = *typed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives;

    #[test]
    fn constant_value_data_type() {
        assert_eq!(ConstantValue::Bool(true).type_hash(), primitives::BOOL);
        assert_eq!(ConstantValue::Int8(0).type_hash(), primitives::INT8);
        assert_eq!(ConstantValue::Int16(0).type_hash(), primitives::INT16);
        assert_eq!(ConstantValue::Int32(0).type_hash(), primitives::INT32);
        assert_eq!(ConstantValue::Int64(0).type_hash(), primitives::INT64);
        assert_eq!(ConstantValue::Uint8(0).type_hash(), primitives::UINT8);
        assert_eq!(ConstantValue::Uint16(0).type_hash(), primitives::UINT16);
        assert_eq!(ConstantValue::Uint32(0).type_hash(), primitives::UINT32);
        assert_eq!(ConstantValue::Uint64(0).type_hash(), primitives::UINT64);
        assert_eq!(ConstantValue::Float(0.0).type_hash(), primitives::FLOAT);
        assert_eq!(ConstantValue::Double(0.0).type_hash(), primitives::DOUBLE);
    }

    #[test]
    fn constant_value_is_const() {
        let value = ConstantValue::Double(3.14159);
        assert!(value.data_type().is_const);
    }

    #[test]
    fn constant_value_copy() {
        let a = ConstantValue::Int32(42);
        let b = a; // Copy
        assert_eq!(a, b);
    }

    #[test]
    fn global_property_entry_constant() {
        let entry = GlobalPropertyEntry::constant("MAX_PLAYERS", ConstantValue::Int32(64));

        assert_eq!(entry.name, "MAX_PLAYERS");
        assert_eq!(entry.qualified_name, "MAX_PLAYERS");
        assert!(entry.is_const);
        assert!(matches!(entry.implementation, GlobalPropertyImpl::Constant(_)));
    }

    #[test]
    fn global_property_entry_with_namespace_deprecated() {
        #[allow(deprecated)]
        let entry = GlobalPropertyEntry::constant("PI", ConstantValue::Double(std::f64::consts::PI))
            .with_qualified_name("math::PI");

        assert_eq!(entry.name, "PI");
        assert_eq!(entry.qualified_name, "math::PI");
        assert_eq!(entry.type_hash, TypeHash::from_name("math::PI"));
    }

    #[test]
    fn global_property_entry_with_namespace() {
        let entry = GlobalPropertyEntry::constant("MAX_SPEED", ConstantValue::Double(100.0))
            .with_namespace(vec!["Game".to_string(), "Config".to_string()]);

        assert_eq!(entry.name, "MAX_SPEED");
        assert_eq!(entry.namespace, vec!["Game".to_string(), "Config".to_string()]);
        assert_eq!(entry.qualified_name, "Game::Config::MAX_SPEED");
        assert_eq!(entry.type_hash, TypeHash::from_name("Game::Config::MAX_SPEED"));
    }

    #[test]
    fn global_property_entry_empty_namespace() {
        let entry = GlobalPropertyEntry::constant("GRAVITY", ConstantValue::Double(9.81));

        assert_eq!(entry.name, "GRAVITY");
        assert!(entry.namespace.is_empty());
        assert_eq!(entry.qualified_name, "GRAVITY");
    }

    #[test]
    fn arc_rwlock_accessor_read() {
        let value = Arc::new(RwLock::new(42i32));
        let accessor: &dyn GlobalPropertyAccessor = &value;

        let read = accessor.read();
        let result = read.downcast_ref::<i32>().unwrap();
        assert_eq!(*result, 42);
    }

    #[test]
    fn arc_rwlock_accessor_write() {
        let value = Arc::new(RwLock::new(42i32));
        let accessor: &dyn GlobalPropertyAccessor = &value;

        accessor.write(Box::new(100i32)).unwrap();

        let guard = RwLock::read(&value).unwrap();
        assert_eq!(*guard, 100);
    }

    #[test]
    fn arc_rwlock_accessor_data_type() {
        let value = Arc::new(RwLock::new(3.14f64));
        let accessor: &dyn GlobalPropertyAccessor = &value;

        assert_eq!(accessor.data_type().type_hash, primitives::DOUBLE);
    }

    #[test]
    fn arc_rwlock_accessor_write_wrong_type() {
        let value = Arc::new(RwLock::new(42i32));
        let accessor: &dyn GlobalPropertyAccessor = &value;

        // Try to write wrong type
        let result = accessor.write(Box::new("wrong type".to_string()));
        assert!(result.is_err());

        if let Err(PropertyError::TypeMismatch { expected, .. }) = result {
            assert_eq!(expected, "int");
        } else {
            panic!("Expected TypeMismatch error");
        }
    }

    #[test]
    fn property_error_to_runtime_error() {
        use crate::RuntimeError;

        let prop_err = PropertyError::TypeMismatch {
            expected: "int",
            actual: "string",
        };
        let runtime_err: RuntimeError = prop_err.into();

        assert!(matches!(runtime_err, RuntimeError::Property(_)));
    }

    // ========================================================================
    // IntoGlobalProperty tests
    // ========================================================================

    #[test]
    fn into_global_property_primitives() {
        // Test a few representative primitives
        assert!(i32::is_inherently_const());
        assert!(f64::is_inherently_const());
        assert!(bool::is_inherently_const());

        let impl_i32 = 42i32.into_global_impl();
        assert!(matches!(impl_i32, GlobalPropertyImpl::Constant(ConstantValue::Int32(42))));

        let impl_f64 = 3.14f64.into_global_impl();
        assert!(matches!(impl_f64, GlobalPropertyImpl::Constant(ConstantValue::Double(v)) if (v - 3.14).abs() < 0.001));

        let impl_bool = true.into_global_impl();
        assert!(matches!(impl_bool, GlobalPropertyImpl::Constant(ConstantValue::Bool(true))));
    }

    #[test]
    fn into_global_property_impl_data_types() {
        // Test that we can get data_type from the impl
        let impl_i32 = 42i32.into_global_impl();
        let dt = impl_i32.data_type();
        assert_eq!(dt.type_hash, primitives::INT32);
        assert!(dt.is_const);

        let impl_f64 = 3.14f64.into_global_impl();
        let dt = impl_f64.data_type();
        assert_eq!(dt.type_hash, primitives::DOUBLE);
        assert!(dt.is_const);
    }

    #[test]
    fn into_global_property_arc_rwlock() {
        let value = Arc::new(RwLock::new(42i32));

        assert!(!<Arc<RwLock<i32>> as IntoGlobalProperty>::is_inherently_const());

        let impl_arc = value.into_global_impl();
        assert!(matches!(impl_arc, GlobalPropertyImpl::Mutable(_)));

        // Get data_type from the impl
        let dt = impl_arc.data_type();
        assert_eq!(dt.type_hash, primitives::INT32);
        assert!(!dt.is_const); // Arc<RwLock<T>> is mutable by default
    }
}
