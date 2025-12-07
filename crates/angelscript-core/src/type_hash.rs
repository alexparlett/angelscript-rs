//! Deterministic hash-based type identity system.
//!
//! This module provides [`TypeHash`], a 64-bit hash that uniquely identifies types,
//! functions, methods, and constructors. Unlike sequential IDs, hashes are computed
//! deterministically from names and signatures, enabling:
//!
//! - Forward references (hash computed before registration)
//! - No registration order dependencies
//! - Same name = same hash (unified FFI/Script identity)
//! - Single map lookups (no secondary name→id maps)
//!
//! # Hash Computation
//!
//! Uses XXHash64 with domain-specific mixing constants to prevent collisions
//! between different entity types (types vs functions vs methods).
//!
//! # Examples
//!
//! ```
//! use angelscript_core::TypeHash;
//!
//! // Type hash from name
//! let int_hash = TypeHash::from_name("int");
//! let same_hash = TypeHash::from_name("int");
//! assert_eq!(int_hash, same_hash);  // Deterministic
//!
//! // Function hash includes parameter types
//! let func1 = TypeHash::from_function("foo", &[TypeHash::from_name("int")]);
//! let func2 = TypeHash::from_function("foo", &[TypeHash::from_name("float")]);
//! assert_ne!(func1, func2);  // Different signatures = different hashes
//! ```

use std::fmt;
use xxhash_rust::xxh64::xxh64;

/// Domain-specific mixing constants for hash computation.
///
/// These constants ensure that different entity types (types, functions, methods)
/// produce distinct hashes even if they share the same name.
pub mod hash_constants {
    /// Separator constant for path components (e.g., namespace separators)
    pub const SEP: u64 = 0x4bc94d6bd06053ad;

    /// Domain marker for type hashes
    pub const TYPE: u64 = 0x2fac10b63a6cc57c;

    /// Domain marker for global function hashes
    pub const FUNCTION: u64 = 0x5ea77ffbcdf5f302;

    /// Domain marker for instance method hashes
    pub const METHOD: u64 = 0x7d3c8b4a92e15f6d;

    /// Domain marker for operator method hashes
    pub const OPERATOR: u64 = 0x3e9f5d2a8c7b1403;

    /// Domain marker for constructor hashes
    pub const CONSTRUCTOR: u64 = 0x9a7f3d5e2b8c4601;

    /// Domain marker for identifier hashes
    pub const IDENT: u64 = 0x1a095090689d4647;

    /// Parameter position mixing constants.
    /// Each parameter position gets a unique constant to ensure parameter order matters.
    pub const PARAM_MARKERS: [u64; 32] = [
        0x9e3779b97f4a7c15,
        0xbf58476d1ce4e5b9,
        0x94d049bb133111eb,
        0xd6e8feb86659fd93,
        0xe7037ed1a0b428db,
        0xc6a4a7935bd1e995,
        0x8648dbbc94d49b8d,
        0xa2b48b2c69e0d657,
        0x7c3e9f2a5b8d1403,
        0x5d8c7b4a3e9f2106,
        0x3f1e9d8c7b5a4203,
        0x1a2b3c4d5e6f7089,
        0x9f8e7d6c5b4a3210,
        0x2468ace013579bdf,
        0xfdb97531eca86420,
        0x123456789abcdef0,
        0xfedcba9876543210,
        0x0f1e2d3c4b5a6978,
        0x89abcdef01234567,
        0x76543210fedcba98,
        0xabcdef0123456789,
        0x3210fedcba987654,
        0xcdef0123456789ab,
        0x6789abcdef012345,
        0x456789abcdef0123,
        0xef0123456789abcd,
        0x23456789abcdef01,
        0xba9876543210fedc,
        0xdcba9876543210fe,
        0x10fedcba98765432,
        0x5432dcba98761fed,
        0x98761fedcba54320,
    ];
}

/// A deterministic 64-bit hash identifying a type, function, or method.
///
/// Computed from the qualified name (for types) or name+signature (for functions).
/// The same input always produces the same hash, enabling forward references
/// and eliminating registration order dependencies.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct TypeHash(pub u64);

impl TypeHash {
    /// Empty/invalid hash constant.
    pub const EMPTY: TypeHash = TypeHash(0);

    /// Create a type hash from a qualified type name.
    ///
    /// The same name always produces the same hash.
    ///
    /// # Examples
    ///
    /// ```
    /// use angelscript_core::TypeHash;
    ///
    /// let hash1 = TypeHash::from_name("int");
    /// let hash2 = TypeHash::from_name("int");
    /// assert_eq!(hash1, hash2);
    ///
    /// let qualified = TypeHash::from_name("Game::Player");
    /// ```
    #[inline]
    pub fn from_name(name: &str) -> Self {
        TypeHash(hash_constants::TYPE ^ xxh64(name.as_bytes(), 0))
    }

    /// Create a function hash from name and parameter type hashes.
    ///
    /// Different parameter types produce different hashes, enabling overload distinction.
    /// Parameter order matters - `(int, float)` produces a different hash than `(float, int)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use angelscript_core::TypeHash;
    ///
    /// let int_hash = TypeHash::from_name("int");
    /// let float_hash = TypeHash::from_name("float");
    ///
    /// let func1 = TypeHash::from_function("print", &[int_hash]);
    /// let func2 = TypeHash::from_function("print", &[float_hash]);
    /// assert_ne!(func1, func2);  // Different overloads
    /// ```
    #[inline]
    pub fn from_function(name: &str, param_hashes: &[TypeHash]) -> Self {
        let mut hash = hash_constants::FUNCTION ^ xxh64(name.as_bytes(), 0);
        for (i, param) in param_hashes.iter().enumerate() {
            let marker = hash_constants::PARAM_MARKERS
                .get(i)
                .copied()
                .unwrap_or_else(|| hash_constants::PARAM_MARKERS[0].wrapping_add(i as u64));
            // Use wrapping_mul to make parameter order matter (not commutative like XOR)
            hash = hash.wrapping_mul(hash_constants::SEP).wrapping_add(marker ^ param.0);
        }
        TypeHash(hash)
    }

    /// Create a method hash from owner type, method name, parameter type hashes, and const qualifiers.
    ///
    /// Methods are distinguished from global functions by incorporating the owner type.
    /// Parameter order matters. The `is_const` flag indicates if this is a const method,
    /// and `return_is_const` indicates if the return type is const-qualified.
    #[inline]
    pub fn from_method(owner: TypeHash, name: &str, param_hashes: &[TypeHash], is_const: bool, return_is_const: bool) -> Self {
        // Include const flags in initial hash computation
        let const_modifier = if is_const { 0x1 } else { 0x0 } | if return_is_const { 0x2 } else { 0x0 };
        let mut hash = hash_constants::METHOD ^ owner.0 ^ xxh64(name.as_bytes(), 0) ^ const_modifier;
        for (i, param) in param_hashes.iter().enumerate() {
            let marker = hash_constants::PARAM_MARKERS
                .get(i)
                .copied()
                .unwrap_or_else(|| hash_constants::PARAM_MARKERS[0].wrapping_add(i as u64));
            // Use wrapping_mul to make parameter order matter (not commutative like XOR)
            hash = hash.wrapping_mul(hash_constants::SEP).wrapping_add(marker ^ param.0);
        }
        TypeHash(hash)
    }

    /// Create a constructor hash from owner type and parameter type hashes.
    ///
    /// Constructors don't have a name, so they're identified by owner + params.
    /// Parameter order matters.
    #[inline]
    pub fn from_constructor(owner: TypeHash, param_hashes: &[TypeHash]) -> Self {
        let mut hash = hash_constants::CONSTRUCTOR ^ owner.0;
        for (i, param) in param_hashes.iter().enumerate() {
            let marker = hash_constants::PARAM_MARKERS
                .get(i)
                .copied()
                .unwrap_or_else(|| hash_constants::PARAM_MARKERS[0].wrapping_add(i as u64));
            // Use wrapping_mul to make parameter order matter (not commutative like XOR)
            hash = hash.wrapping_mul(hash_constants::SEP).wrapping_add(marker ^ param.0);
        }
        TypeHash(hash)
    }

    /// Create an operator method hash from owner type, operator name, parameter type hashes, and const qualifiers.
    ///
    /// Operators are like methods but use a different domain constant to distinguish
    /// `opAdd` from a regular method named "opAdd". The `is_const` flag indicates if this
    /// is a const method, and `return_is_const` indicates if the return type is const-qualified.
    #[inline]
    pub fn from_operator(owner: TypeHash, operator_name: &str, param_hashes: &[TypeHash], is_const: bool, return_is_const: bool) -> Self {
        let name_hash = xxh64(operator_name.as_bytes(), 0);
        // Include const flags in initial hash computation
        let const_modifier = if is_const { 0x1 } else { 0x0 } | if return_is_const { 0x2 } else { 0x0 };
        let mut hash = hash_constants::OPERATOR ^ owner.0 ^ name_hash ^ const_modifier;
        for (i, param) in param_hashes.iter().enumerate() {
            let marker = hash_constants::PARAM_MARKERS
                .get(i)
                .copied()
                .unwrap_or_else(|| hash_constants::PARAM_MARKERS[0].wrapping_add(i as u64));
            hash = hash.wrapping_mul(hash_constants::SEP).wrapping_add(marker ^ param.0);
        }
        TypeHash(hash)
    }

    /// Create a template instance hash from template hash and type argument hashes.
    ///
    /// Type argument order matters - `dict<int, string>` produces a different hash
    /// than `dict<string, int>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use angelscript_core::TypeHash;
    ///
    /// let array_template = TypeHash::from_name("array");
    /// let int_hash = TypeHash::from_name("int");
    ///
    /// let array_int = TypeHash::from_template_instance(array_template, &[int_hash]);
    /// ```
    #[inline]
    pub fn from_template_instance(template: TypeHash, args: &[TypeHash]) -> Self {
        let mut hash = template.0;
        for (i, arg) in args.iter().enumerate() {
            let marker = hash_constants::PARAM_MARKERS
                .get(i)
                .copied()
                .unwrap_or_else(|| hash_constants::PARAM_MARKERS[0].wrapping_add(i as u64));
            // Use wrapping_mul to make type argument order matter (not commutative like XOR)
            hash = hash.wrapping_mul(hash_constants::SEP).wrapping_add(marker ^ arg.0);
        }
        TypeHash(hash)
    }

    /// Check if this is an empty/invalid hash.
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Get the underlying u64 value.
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Create a TypeHash from a Rust type's TypeId.
    ///
    /// This is used for runtime type verification in the FFI system.
    /// Note: This produces a different hash than `from_name()` since it's
    /// based on Rust's internal type representation, not the AngelScript name.
    #[inline]
    pub fn of<T: 'static>() -> Self {
        use std::any::TypeId;
        use std::hash::{Hash, Hasher};

        // Hash the TypeId to get a u64
        let type_id = TypeId::of::<T>();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        type_id.hash(&mut hasher);
        TypeHash(hasher.finish())
    }

    /// Create a TypeHash from an existing TypeId.
    ///
    /// Used when you have a TypeId value but need to convert it to TypeHash.
    #[inline]
    pub fn of_type_id(type_id: std::any::TypeId) -> Self {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        type_id.hash(&mut hasher);
        TypeHash(hasher.finish())
    }
}

impl fmt::Debug for TypeHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TypeHash({:#018x})", self.0)
    }
}

impl fmt::Display for TypeHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#018x}", self.0)
    }
}

/// Well-known constant hashes for primitive types.
///
/// These are pre-computed from `TypeHash::from_name()` for efficiency.
/// The values are the full computed hash (TYPE constant already XORed in).
pub mod primitives {
    use super::TypeHash;

    /// Hash for `void` type
    pub const VOID: TypeHash = TypeHash(0xe4b3797ddcf989ea);

    /// Hash for `bool` type
    pub const BOOL: TypeHash = TypeHash(0x1e0c8fa4cced99c1);

    /// Hash for `int8` type
    pub const INT8: TypeHash = TypeHash(0x2b44191092e74388);

    /// Hash for `int16` type
    pub const INT16: TypeHash = TypeHash(0x95aebfc985e9b115);

    /// Hash for `int` type (32-bit signed integer)
    pub const INT32: TypeHash = TypeHash(0x4f5e5320cd1c92bf);

    /// Hash for `int64` type
    pub const INT64: TypeHash = TypeHash(0x7d6c550df59a1924);

    /// Hash for `uint8` type
    pub const UINT8: TypeHash = TypeHash(0x0e8b2d31cdfa9716);

    /// Hash for `uint16` type
    pub const UINT16: TypeHash = TypeHash(0x269d68dfde65ae7f);

    /// Hash for `uint` type (32-bit unsigned integer)
    pub const UINT32: TypeHash = TypeHash(0x543fb8f520aa3e26);

    /// Hash for `uint64` type
    pub const UINT64: TypeHash = TypeHash(0x32ba58d17fda82dd);

    /// Hash for `float` type
    pub const FLOAT: TypeHash = TypeHash(0x02d5a2fddaf5bb69);

    /// Hash for `double` type
    pub const DOUBLE: TypeHash = TypeHash(0xeb125587f6c2a79b);

    /// Hash for `string` type
    /// Note: string is a registered type (not a true primitive), so this matches TypeHash::from_name("string")
    pub const STRING: TypeHash = TypeHash(0x7a8d5fb1ba695978);

    /// Hash for null literal type (converts to any handle)
    pub const NULL: TypeHash = TypeHash(0x1165f1b6597b5a46);

    /// Placeholder for self-referential template types during FFI import.
    /// Used for methods like `array<T> opAssign(const array<T> &in)` where the
    /// return/param type is the template itself with its own params.
    /// This is a special sentinel value, not computed from a name.
    pub const SELF: TypeHash = TypeHash(0xfffffffffffffffe);

    /// Hash for `?` type - accepts any value (for generic FFI functions)
    /// This is a special sentinel value, not computed from a name.
    pub const VARIABLE_PARAM: TypeHash = TypeHash(0x3f3f3f3f3f3f3f3f);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_hash_determinism() {
        // Same name should always produce same hash
        let hash1 = TypeHash::from_name("int");
        let hash2 = TypeHash::from_name("int");
        assert_eq!(hash1, hash2);

        let hash3 = TypeHash::from_name("Game::Player");
        let hash4 = TypeHash::from_name("Game::Player");
        assert_eq!(hash3, hash4);
    }

    #[test]
    fn type_hash_uniqueness() {
        // Different names should produce different hashes
        let int_hash = TypeHash::from_name("int");
        let float_hash = TypeHash::from_name("float");
        let string_hash = TypeHash::from_name("string");
        let player_hash = TypeHash::from_name("Player");

        assert_ne!(int_hash, float_hash);
        assert_ne!(int_hash, string_hash);
        assert_ne!(int_hash, player_hash);
        assert_ne!(float_hash, string_hash);
    }

    #[test]
    fn function_hash_determinism() {
        let int_hash = TypeHash::from_name("int");
        let func1 = TypeHash::from_function("print", &[int_hash]);
        let func2 = TypeHash::from_function("print", &[int_hash]);
        assert_eq!(func1, func2);
    }

    #[test]
    fn function_hash_overload_distinction() {
        let int_hash = TypeHash::from_name("int");
        let float_hash = TypeHash::from_name("float");
        let string_hash = TypeHash::from_name("string");

        let func_int = TypeHash::from_function("print", &[int_hash]);
        let func_float = TypeHash::from_function("print", &[float_hash]);
        let func_string = TypeHash::from_function("print", &[string_hash]);
        let func_two_params = TypeHash::from_function("print", &[int_hash, float_hash]);

        assert_ne!(func_int, func_float);
        assert_ne!(func_int, func_string);
        assert_ne!(func_int, func_two_params);
    }

    #[test]
    fn function_hash_parameter_order_matters() {
        let int_hash = TypeHash::from_name("int");
        let float_hash = TypeHash::from_name("float");

        let func1 = TypeHash::from_function("foo", &[int_hash, float_hash]);
        let func2 = TypeHash::from_function("foo", &[float_hash, int_hash]);
        assert_ne!(func1, func2);
    }

    #[test]
    fn method_hash_includes_owner() {
        let int_hash = TypeHash::from_name("int");
        let player_hash = TypeHash::from_name("Player");
        let enemy_hash = TypeHash::from_name("Enemy");

        // Same method name and params, different owners
        let player_update = TypeHash::from_method(player_hash, "update", &[int_hash], false, false);
        let enemy_update = TypeHash::from_method(enemy_hash, "update", &[int_hash], false, false);
        assert_ne!(player_update, enemy_update);
    }

    #[test]
    fn method_vs_function_distinction() {
        let int_hash = TypeHash::from_name("int");
        let player_hash = TypeHash::from_name("Player");

        // Global function vs method with same name and params
        let global_func = TypeHash::from_function("update", &[int_hash]);
        let method = TypeHash::from_method(player_hash, "update", &[int_hash], false, false);
        assert_ne!(global_func, method);
    }

    #[test]
    fn constructor_hash_determinism() {
        let player_hash = TypeHash::from_name("Player");
        let int_hash = TypeHash::from_name("int");

        let ctor1 = TypeHash::from_constructor(player_hash, &[int_hash]);
        let ctor2 = TypeHash::from_constructor(player_hash, &[int_hash]);
        assert_eq!(ctor1, ctor2);
    }

    #[test]
    fn constructor_hash_overload_distinction() {
        let player_hash = TypeHash::from_name("Player");
        let int_hash = TypeHash::from_name("int");
        let string_hash = TypeHash::from_name("string");

        let default_ctor = TypeHash::from_constructor(player_hash, &[]);
        let int_ctor = TypeHash::from_constructor(player_hash, &[int_hash]);
        let string_ctor = TypeHash::from_constructor(player_hash, &[string_hash]);

        assert_ne!(default_ctor, int_ctor);
        assert_ne!(default_ctor, string_ctor);
        assert_ne!(int_ctor, string_ctor);
    }

    #[test]
    fn template_instance_hash() {
        let array_template = TypeHash::from_name("array");
        let int_hash = TypeHash::from_name("int");
        let float_hash = TypeHash::from_name("float");

        let array_int = TypeHash::from_template_instance(array_template, &[int_hash]);
        let array_float = TypeHash::from_template_instance(array_template, &[float_hash]);

        // Different type args = different instance hashes
        assert_ne!(array_int, array_float);

        // Same type args = same instance hash
        let array_int2 = TypeHash::from_template_instance(array_template, &[int_hash]);
        assert_eq!(array_int, array_int2);

        // Instance hash differs from template hash
        assert_ne!(array_int, array_template);
    }

    #[test]
    fn template_instance_multi_param() {
        let dict_template = TypeHash::from_name("dictionary");
        let string_hash = TypeHash::from_name("string");
        let int_hash = TypeHash::from_name("int");
        let float_hash = TypeHash::from_name("float");

        let dict_string_int = TypeHash::from_template_instance(dict_template, &[string_hash, int_hash]);
        let dict_string_float = TypeHash::from_template_instance(dict_template, &[string_hash, float_hash]);
        let dict_int_string = TypeHash::from_template_instance(dict_template, &[int_hash, string_hash]);

        assert_ne!(dict_string_int, dict_string_float);
        assert_ne!(dict_string_int, dict_int_string);  // Order matters
    }

    #[test]
    fn empty_hash() {
        assert!(TypeHash::EMPTY.is_empty());
        assert!(!TypeHash::from_name("int").is_empty());
    }

    #[test]
    fn hash_display() {
        let hash = TypeHash::from_name("int");
        let display = format!("{}", hash);
        assert!(display.starts_with("0x"));
    }

    #[test]
    fn hash_debug() {
        let hash = TypeHash::from_name("int");
        let debug = format!("{:?}", hash);
        assert!(debug.starts_with("TypeHash(0x"));
    }

    #[test]
    fn primitive_constants_are_valid() {
        // Verify primitive constants are computed and non-empty
        assert!(!primitives::VOID.is_empty());
        assert!(!primitives::BOOL.is_empty());
        assert!(!primitives::INT8.is_empty());
        assert!(!primitives::INT16.is_empty());
        assert!(!primitives::INT32.is_empty());
        assert!(!primitives::INT64.is_empty());
        assert!(!primitives::UINT8.is_empty());
        assert!(!primitives::UINT16.is_empty());
        assert!(!primitives::UINT32.is_empty());
        assert!(!primitives::UINT64.is_empty());
        assert!(!primitives::FLOAT.is_empty());
        assert!(!primitives::DOUBLE.is_empty());
        assert!(!primitives::NULL.is_empty());
        assert!(!primitives::SELF.is_empty());
        assert!(!primitives::VARIABLE_PARAM.is_empty());
    }

    #[test]
    fn primitive_constants_match_from_name() {
        // Verify that primitive constants match what from_name would compute
        assert_eq!(primitives::VOID, TypeHash::from_name("void"));
        assert_eq!(primitives::BOOL, TypeHash::from_name("bool"));
        assert_eq!(primitives::INT8, TypeHash::from_name("int8"));
        assert_eq!(primitives::INT16, TypeHash::from_name("int16"));
        assert_eq!(primitives::INT32, TypeHash::from_name("int"));
        assert_eq!(primitives::INT64, TypeHash::from_name("int64"));
        assert_eq!(primitives::UINT8, TypeHash::from_name("uint8"));
        assert_eq!(primitives::UINT16, TypeHash::from_name("uint16"));
        assert_eq!(primitives::UINT32, TypeHash::from_name("uint"));
        assert_eq!(primitives::UINT64, TypeHash::from_name("uint64"));
        assert_eq!(primitives::FLOAT, TypeHash::from_name("float"));
        assert_eq!(primitives::DOUBLE, TypeHash::from_name("double"));
        assert_eq!(primitives::NULL, TypeHash::from_name("null"));
        // String is a registered type (not a true primitive) so it also uses from_name
        assert_eq!(primitives::STRING, TypeHash::from_name("string"));
    }

    #[test]
    fn primitive_constants_are_unique() {
        use std::collections::HashSet;

        let primitives = [
            primitives::VOID,
            primitives::BOOL,
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
            primitives::NULL,
            primitives::SELF,
            primitives::VARIABLE_PARAM,
        ];

        let unique: HashSet<_> = primitives.iter().collect();
        assert_eq!(unique.len(), primitives.len(), "All primitive hashes should be unique");
    }

    #[test]
    fn type_hash_ordering() {
        let hash1 = TypeHash(100);
        let hash2 = TypeHash(200);
        assert!(hash1 < hash2);
        assert!(hash2 > hash1);
    }

    #[test]
    fn type_hash_as_u64() {
        let hash = TypeHash(0x123456789abcdef0);
        assert_eq!(hash.as_u64(), 0x123456789abcdef0);
    }

    #[test]
    fn many_parameters_supported() {
        let int_hash = TypeHash::from_name("int");
        let params: Vec<TypeHash> = (0..50).map(|_| int_hash).collect();

        // Should not panic with more than 32 parameters
        let func = TypeHash::from_function("many_params", &params);
        assert!(!func.is_empty());
    }
}
