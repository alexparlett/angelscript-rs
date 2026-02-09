use std::fmt;

/// Lightweight reference to a heap-allocated script object.
/// Uses generational indexing to detect stale references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle {
    /// Index into the object pool.
    pub index: u32,
    /// Generation counter — must match pool entry to be valid.
    pub generation: u32,
}

impl Handle {
    pub fn new(index: u32, generation: u32) -> Self {
        Handle { index, generation }
    }
}

impl fmt::Display for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Handle({}:{})", self.index, self.generation)
    }
}

/// Runtime value representation for the VM.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// No value (void return).
    Void,
    /// Boolean value.
    Bool(bool),
    /// 8-bit signed integer.
    I8(i8),
    /// 16-bit signed integer.
    I16(i16),
    /// 32-bit signed integer.
    I32(i32),
    /// 64-bit signed integer.
    I64(i64),
    /// 8-bit unsigned integer.
    U8(u8),
    /// 16-bit unsigned integer.
    U16(u16),
    /// 32-bit unsigned integer.
    U32(u32),
    /// 64-bit unsigned integer.
    U64(u64),
    /// 32-bit float.
    F32(f32),
    /// 64-bit float.
    F64(f64),
    /// Object handle (reference to heap object).
    Handle(Handle),
    /// Null handle.
    Null,
}

impl Value {
    /// Get as a 32-bit integer, if applicable.
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Value::I8(v) => Some(*v as i32),
            Value::I16(v) => Some(*v as i32),
            Value::I32(v) => Some(*v),
            Value::U8(v) => Some(*v as i32),
            Value::U16(v) => Some(*v as i32),
            Value::Bool(v) => Some(if *v { 1 } else { 0 }),
            _ => None,
        }
    }

    /// Get as a 64-bit integer, if applicable.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::I8(v) => Some(*v as i64),
            Value::I16(v) => Some(*v as i64),
            Value::I32(v) => Some(*v as i64),
            Value::I64(v) => Some(*v),
            Value::U8(v) => Some(*v as i64),
            Value::U16(v) => Some(*v as i64),
            Value::U32(v) => Some(*v as i64),
            Value::Bool(v) => Some(if *v { 1 } else { 0 }),
            _ => None,
        }
    }

    /// Get as a 64-bit float, if applicable.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::F32(v) => Some(*v as f64),
            Value::F64(v) => Some(*v),
            Value::I32(v) => Some(*v as f64),
            Value::I64(v) => Some(*v as f64),
            Value::U32(v) => Some(*v as f64),
            Value::U64(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// Whether this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Whether this value is void.
    pub fn is_void(&self) -> bool {
        matches!(self, Value::Void)
    }

    /// Convert to raw bytes (for VM stack operations).
    pub fn to_dword(&self) -> u32 {
        match self {
            Value::Bool(v) => {
                if *v {
                    1u32
                } else {
                    0u32
                }
            }
            Value::I8(v) => *v as u32,
            Value::I16(v) => *v as u32,
            Value::I32(v) => *v as u32,
            Value::U8(v) => *v as u32,
            Value::U16(v) => *v as u32,
            Value::U32(v) => *v,
            Value::F32(v) => v.to_bits(),
            _ => 0,
        }
    }

    /// Convert to raw 64-bit value (for VM stack operations).
    pub fn to_qword(&self) -> u64 {
        match self {
            Value::I64(v) => *v as u64,
            Value::U64(v) => *v,
            Value::F64(v) => v.to_bits(),
            _ => self.to_dword() as u64,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Void => write!(f, "void"),
            Value::Bool(v) => write!(f, "{}", v),
            Value::I8(v) => write!(f, "{}", v),
            Value::I16(v) => write!(f, "{}", v),
            Value::I32(v) => write!(f, "{}", v),
            Value::I64(v) => write!(f, "{}", v),
            Value::U8(v) => write!(f, "{}", v),
            Value::U16(v) => write!(f, "{}", v),
            Value::U32(v) => write!(f, "{}", v),
            Value::U64(v) => write!(f, "{}", v),
            Value::F32(v) => write!(f, "{}f", v),
            Value::F64(v) => write!(f, "{}d", v),
            Value::Handle(h) => write!(f, "{}", h),
            Value::Null => write!(f, "null"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_conversions() {
        assert_eq!(Value::I32(42).as_i32(), Some(42));
        assert_eq!(Value::I8(7).as_i32(), Some(7));
        assert_eq!(Value::Bool(true).as_i32(), Some(1));
        assert_eq!(Value::F32(3.14).as_i32(), None);
    }

    #[test]
    fn value_to_dword() {
        assert_eq!(Value::I32(42).to_dword(), 42);
        assert_eq!(Value::Bool(true).to_dword(), 1);
        assert_eq!(Value::Bool(false).to_dword(), 0);
        assert_eq!(Value::F32(1.0).to_dword(), 1.0f32.to_bits());
    }

    #[test]
    fn value_to_qword() {
        assert_eq!(Value::I64(100).to_qword(), 100);
        assert_eq!(Value::F64(2.0).to_qword(), 2.0f64.to_bits());
    }

    #[test]
    fn handle_display() {
        let h = Handle::new(5, 3);
        assert_eq!(format!("{}", h), "Handle(5:3)");
    }

    #[test]
    fn value_null_void() {
        assert!(Value::Null.is_null());
        assert!(!Value::I32(0).is_null());
        assert!(Value::Void.is_void());
        assert!(!Value::Null.is_void());
    }
}
