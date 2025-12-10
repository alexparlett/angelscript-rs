//! Constant pool for compiled modules.
//!
//! The constant pool stores values that are referenced by bytecode instructions,
//! such as numeric literals, string data, and type hashes.

use angelscript_core::TypeHash;
use rustc_hash::FxHashMap;

/// Values stored in the constant pool.
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    /// Signed integer (i64 to support all int sizes).
    Int(i64),
    /// Unsigned integer.
    Uint(u64),
    /// 32-bit float.
    Float32(f32),
    /// 64-bit float.
    Float64(f64),
    /// Raw string literal bytes.
    ///
    /// NOT a script string type. The VM passes this to
    /// `Context::string_factory().create()` to produce the actual
    /// string value (e.g., ScriptString).
    StringData(Vec<u8>),
    /// Type hash (for function calls, type checks, etc.).
    TypeHash(TypeHash),
}

/// Module-level constant pool with deduplication.
///
/// Shared across all functions in a module to avoid duplicate strings/values.
#[derive(Debug, Clone, Default)]
pub struct ConstantPool {
    /// The actual constants.
    constants: Vec<Constant>,
    /// Deduplication index: maps constant to its index.
    index: FxHashMap<ConstantKey, u32>,
}

/// Key for constant deduplication (hashable version of Constant).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ConstantKey {
    Int(i64),
    Uint(u64),
    Float32(u32), // Bit pattern for hashing
    Float64(u64), // Bit pattern for hashing
    StringData(Vec<u8>),
    TypeHash(TypeHash),
}

impl ConstantPool {
    /// Create a new empty constant pool.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a constant pool with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            constants: Vec::with_capacity(capacity),
            index: FxHashMap::with_capacity_and_hasher(capacity, Default::default()),
        }
    }

    /// Add or get existing constant, returns index.
    ///
    /// Deduplicates identical constants.
    pub fn add(&mut self, constant: Constant) -> u32 {
        let key = Self::to_key(&constant);

        if let Some(&idx) = self.index.get(&key) {
            return idx;
        }

        let idx = self.constants.len() as u32;
        self.constants.push(constant);
        self.index.insert(key, idx);
        idx
    }

    /// Add an integer constant.
    pub fn add_int(&mut self, value: i64) -> u32 {
        self.add(Constant::Int(value))
    }

    /// Add an unsigned integer constant.
    pub fn add_uint(&mut self, value: u64) -> u32 {
        self.add(Constant::Uint(value))
    }

    /// Add a 32-bit float constant.
    pub fn add_f32(&mut self, value: f32) -> u32 {
        self.add(Constant::Float32(value))
    }

    /// Add a 64-bit float constant.
    pub fn add_f64(&mut self, value: f64) -> u32 {
        self.add(Constant::Float64(value))
    }

    /// Add string data (raw bytes).
    pub fn add_string(&mut self, data: Vec<u8>) -> u32 {
        self.add(Constant::StringData(data))
    }

    /// Add a type hash.
    pub fn add_type_hash(&mut self, hash: TypeHash) -> u32 {
        self.add(Constant::TypeHash(hash))
    }

    /// Get constant by index.
    pub fn get(&self, index: u32) -> Option<&Constant> {
        self.constants.get(index as usize)
    }

    /// Get all constants (for serialization).
    pub fn constants(&self) -> &[Constant] {
        &self.constants
    }

    /// Number of constants.
    pub fn len(&self) -> usize {
        self.constants.len()
    }

    /// Check if the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.constants.is_empty()
    }

    /// Convert a Constant to its hashable key representation.
    fn to_key(constant: &Constant) -> ConstantKey {
        match constant {
            Constant::Int(v) => ConstantKey::Int(*v),
            Constant::Uint(v) => ConstantKey::Uint(*v),
            Constant::Float32(v) => ConstantKey::Float32(v.to_bits()),
            Constant::Float64(v) => ConstantKey::Float64(v.to_bits()),
            Constant::StringData(b) => ConstantKey::StringData(b.clone()),
            Constant::TypeHash(h) => ConstantKey::TypeHash(*h),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_pool_is_empty() {
        let pool = ConstantPool::new();
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn add_int() {
        let mut pool = ConstantPool::new();
        let idx = pool.add_int(42);
        assert_eq!(idx, 0);
        assert_eq!(pool.get(idx), Some(&Constant::Int(42)));
    }

    #[test]
    fn add_float() {
        let mut pool = ConstantPool::new();
        let idx32 = pool.add_f32(1.5);
        let idx64 = pool.add_f64(2.5);

        assert_eq!(idx32, 0);
        assert_eq!(idx64, 1);
        assert!(matches!(pool.get(idx32), Some(Constant::Float32(v)) if (*v - 1.5).abs() < 0.001));
    }

    #[test]
    fn add_string() {
        let mut pool = ConstantPool::new();
        let idx = pool.add_string(b"hello".to_vec());
        assert_eq!(idx, 0);
        assert_eq!(
            pool.get(idx),
            Some(&Constant::StringData(b"hello".to_vec()))
        );
    }

    #[test]
    fn add_type_hash() {
        let mut pool = ConstantPool::new();
        let hash = TypeHash::from_name("MyClass");
        let idx = pool.add_type_hash(hash);
        assert_eq!(idx, 0);
        assert_eq!(pool.get(idx), Some(&Constant::TypeHash(hash)));
    }

    #[test]
    fn deduplication() {
        let mut pool = ConstantPool::new();

        let idx1 = pool.add_int(100);
        let idx2 = pool.add_int(200);
        let idx3 = pool.add_int(100); // Duplicate

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0); // Same as idx1
        assert_eq!(pool.len(), 2); // Only 2 unique constants
    }

    #[test]
    fn float_deduplication_by_bits() {
        let mut pool = ConstantPool::new();

        let idx1 = pool.add_f64(1.0);
        let idx2 = pool.add_f64(1.0);

        assert_eq!(idx1, idx2);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn string_deduplication() {
        let mut pool = ConstantPool::new();

        let idx1 = pool.add_string(b"test".to_vec());
        let idx2 = pool.add_string(b"other".to_vec());
        let idx3 = pool.add_string(b"test".to_vec()); // Duplicate

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0); // Same as idx1
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn get_out_of_bounds() {
        let pool = ConstantPool::new();
        assert_eq!(pool.get(0), None);
        assert_eq!(pool.get(100), None);
    }

    #[test]
    fn constants_slice() {
        let mut pool = ConstantPool::new();
        pool.add_int(1);
        pool.add_int(2);
        pool.add_int(3);

        let constants = pool.constants();
        assert_eq!(constants.len(), 3);
        assert_eq!(constants[0], Constant::Int(1));
        assert_eq!(constants[1], Constant::Int(2));
        assert_eq!(constants[2], Constant::Int(3));
    }
}
