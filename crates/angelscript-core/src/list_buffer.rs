//! List initialization buffer types.
//!
//! These types support initialization list syntax in AngelScript:
//!
//! ```angelscript
//! array<int> a = {1, 2, 3};
//! dictionary@ d = {{"key1", 1}, {"key2", 2}};
//! ```
//!
//! # Buffer Types
//!
//! - [`ListBuffer`] - For simple repeated elements: `{1, 2, 3}`
//! - [`TupleListBuffer`] - For repeated tuples: `{{"a", 1}, {"b", 2}}`
//!
//! # Usage
//!
//! Native functions registered with `list_factory` or `list_construct` behaviors
//! receive list data through these buffer types. The VM populates the buffers
//! from initialization list expressions before calling the native function.

use crate::TypeHash;
use crate::native_fn::VmSlot;

/// Buffer containing initialization list data.
///
/// Provides typed access to elements passed via `{1, 2, 3}` syntax.
/// Used by list construction/factory behaviors.
///
/// # Example
///
/// ```ignore
/// fn array_list_factory(buffer: &ListBuffer) -> ScriptArray {
///     let mut arr = ScriptArray::new(buffer.element_type());
///     for slot in buffer.iter() {
///         arr.push(slot.clone_if_possible().unwrap());
///     }
///     arr
/// }
/// ```
#[derive(Debug)]
pub struct ListBuffer<'a> {
    /// Raw element data
    elements: &'a [VmSlot],
    /// Element type (for type checking)
    element_type: TypeHash,
}

impl<'a> ListBuffer<'a> {
    /// Create a new list buffer from a slice of slots.
    ///
    /// # Parameters
    ///
    /// - `elements`: The VmSlot values from the initialization list
    /// - `element_type`: The expected type ID of each element
    pub fn new(elements: &'a [VmSlot], element_type: TypeHash) -> Self {
        Self {
            elements,
            element_type,
        }
    }

    /// Number of elements in the list.
    #[inline]
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Get element at index.
    ///
    /// Returns `None` if index is out of bounds.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&VmSlot> {
        self.elements.get(index)
    }

    /// Get the underlying slice of elements.
    #[inline]
    pub fn as_slice(&self) -> &[VmSlot] {
        self.elements
    }

    /// Iterate over elements.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &VmSlot> {
        self.elements.iter()
    }

    /// Get the type ID of list elements.
    #[inline]
    pub fn element_type(&self) -> TypeHash {
        self.element_type
    }
}

impl<'a> IntoIterator for &'a ListBuffer<'a> {
    type Item = &'a VmSlot;
    type IntoIter = std::slice::Iter<'a, VmSlot>;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.iter()
    }
}

/// Buffer for tuple-based initialization lists.
///
/// Used for dictionary-style initialization: `{{"key1", 1}, {"key2", 2}}`.
/// Tuples are stored flattened: `[key1, val1, key2, val2, ...]`.
///
/// # Example
///
/// ```ignore
/// fn dict_list_factory(buffer: &TupleListBuffer) -> ScriptDict {
///     let mut dict = ScriptDict::new();
///     for tuple in buffer.iter() {
///         let key = ScriptKey::from_slot(&tuple[0]).unwrap();
///         let value = tuple[1].clone_if_possible().unwrap();
///         dict.insert(key, value);
///     }
///     dict
/// }
/// ```
#[derive(Debug)]
pub struct TupleListBuffer<'a> {
    /// Tuples stored as flattened array
    data: &'a [VmSlot],
    /// Number of elements per tuple
    tuple_size: usize,
    /// Types of each tuple element
    element_types: Vec<TypeHash>,
}

impl<'a> TupleListBuffer<'a> {
    /// Create a new tuple list buffer.
    ///
    /// # Parameters
    ///
    /// - `data`: Flattened tuple data `[k1, v1, k2, v2, ...]`
    /// - `tuple_size`: Number of elements per tuple
    /// - `element_types`: Type IDs for each element position
    ///
    /// # Panics
    ///
    /// Panics if `element_types.len() != tuple_size` or if `data.len()` is
    /// not divisible by `tuple_size`.
    pub fn new(data: &'a [VmSlot], tuple_size: usize, element_types: Vec<TypeHash>) -> Self {
        assert_eq!(
            element_types.len(),
            tuple_size,
            "element_types must match tuple_size"
        );
        assert!(
            data.len().is_multiple_of(tuple_size),
            "data length must be divisible by tuple_size"
        );

        Self {
            data,
            tuple_size,
            element_types,
        }
    }

    /// Number of tuples in the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        if self.tuple_size == 0 {
            0
        } else {
            self.data.len() / self.tuple_size
        }
    }

    /// Check if the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the tuple size (number of elements per tuple).
    #[inline]
    pub fn tuple_size(&self) -> usize {
        self.tuple_size
    }

    /// Get tuple at index as slice.
    ///
    /// Returns `None` if index is out of bounds.
    pub fn get_tuple(&self, index: usize) -> Option<&[VmSlot]> {
        let start = index * self.tuple_size;
        let end = start + self.tuple_size;
        if end <= self.data.len() {
            Some(&self.data[start..end])
        } else {
            None
        }
    }

    /// Get the type ID for a tuple element position.
    ///
    /// Returns `None` if position is out of bounds.
    #[inline]
    pub fn element_type(&self, position: usize) -> Option<TypeHash> {
        self.element_types.get(position).copied()
    }

    /// Get all element types.
    #[inline]
    pub fn element_types(&self) -> &[TypeHash] {
        &self.element_types
    }

    /// Iterate over tuples.
    ///
    /// Each iteration yields a slice of `tuple_size` elements.
    pub fn iter(&self) -> impl Iterator<Item = &[VmSlot]> {
        self.data.chunks_exact(self.tuple_size)
    }
}

/// Pattern describing expected initialization list format.
///
/// Used by `list_construct` and `list_factory` behaviors to specify
/// what types the initialization list should contain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListPattern {
    /// Zero or more elements of type T: `{repeat T}`.
    ///
    /// Example: `array<int> a = {1, 2, 3};`
    Repeat(TypeHash),

    /// Fixed sequence of types: `{int, string}`.
    ///
    /// Example: `MyStruct s = {42, "hello"};`
    Fixed(Vec<TypeHash>),

    /// Repeated tuples: `{repeat {K, V}}`.
    ///
    /// Example: `dictionary@ d = {{"a", 1}, {"b", 2}};`
    RepeatTuple(Vec<TypeHash>),
}

impl ListPattern {
    /// Create a repeat pattern for a single element type.
    pub fn repeat(element_type: TypeHash) -> Self {
        ListPattern::Repeat(element_type)
    }

    /// Create a fixed sequence pattern.
    pub fn fixed(types: Vec<TypeHash>) -> Self {
        ListPattern::Fixed(types)
    }

    /// Create a repeat tuple pattern (for dictionary-style init).
    pub fn repeat_tuple(tuple_types: Vec<TypeHash>) -> Self {
        ListPattern::RepeatTuple(tuple_types)
    }

    /// Check if this pattern matches a list of values.
    ///
    /// For `Repeat` patterns, any number of elements is valid.
    /// For `Fixed` patterns, the exact sequence must match.
    /// For `RepeatTuple` patterns, each tuple must match the expected types.
    pub fn matches(&self, value_types: &[TypeHash]) -> bool {
        match self {
            ListPattern::Repeat(expected) => value_types.iter().all(|t| t == expected),
            ListPattern::Fixed(expected) => {
                value_types.len() == expected.len()
                    && value_types.iter().zip(expected).all(|(a, b)| a == b)
            }
            ListPattern::RepeatTuple(tuple_types) => {
                if tuple_types.is_empty() {
                    return value_types.is_empty();
                }
                if !value_types.len().is_multiple_of(tuple_types.len()) {
                    return false;
                }
                value_types
                    .chunks(tuple_types.len())
                    .all(|chunk| chunk.iter().zip(tuple_types).all(|(a, b)| a == b))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives as primitive_hashes;

    // ListBuffer tests
    #[test]
    fn test_list_buffer_empty() {
        let buffer = ListBuffer::new(&[], primitive_hashes::INT32);
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        assert!(buffer.get(0).is_none());
    }

    #[test]
    fn test_list_buffer_with_elements() {
        let elements = vec![VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3)];
        let buffer = ListBuffer::new(&elements, primitive_hashes::INT32);

        assert!(!buffer.is_empty());
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.element_type(), primitive_hashes::INT32);
    }

    #[test]
    fn test_list_buffer_get() {
        let elements = vec![VmSlot::Int(10), VmSlot::Int(20), VmSlot::Int(30)];
        let buffer = ListBuffer::new(&elements, primitive_hashes::INT32);

        assert!(matches!(buffer.get(0), Some(VmSlot::Int(10))));
        assert!(matches!(buffer.get(1), Some(VmSlot::Int(20))));
        assert!(matches!(buffer.get(2), Some(VmSlot::Int(30))));
        assert!(buffer.get(3).is_none());
    }

    #[test]
    fn test_list_buffer_iter() {
        let elements = vec![VmSlot::Int(1), VmSlot::Int(2)];
        let buffer = ListBuffer::new(&elements, primitive_hashes::INT32);

        let collected: Vec<_> = buffer.iter().collect();
        assert_eq!(collected.len(), 2);
    }

    #[test]
    fn test_list_buffer_into_iter() {
        let elements = vec![VmSlot::Int(1), VmSlot::Int(2)];
        let buffer = ListBuffer::new(&elements, primitive_hashes::INT32);

        let mut count = 0;
        for _slot in &buffer {
            count += 1;
        }
        assert_eq!(count, 2);
    }

    // TupleListBuffer tests
    #[test]
    fn test_tuple_list_buffer_empty() {
        let buffer = TupleListBuffer::new(
            &[],
            2,
            vec![primitive_hashes::STRING, primitive_hashes::INT32],
        );

        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.tuple_size(), 2);
    }

    #[test]
    fn test_tuple_list_buffer_with_pairs() {
        let data = vec![
            VmSlot::String("a".into()),
            VmSlot::Int(1),
            VmSlot::String("b".into()),
            VmSlot::Int(2),
        ];
        let buffer = TupleListBuffer::new(
            &data,
            2,
            vec![primitive_hashes::STRING, primitive_hashes::INT32],
        );

        assert!(!buffer.is_empty());
        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer.tuple_size(), 2);
    }

    #[test]
    fn test_tuple_list_buffer_get_tuple() {
        let data = vec![
            VmSlot::String("key1".into()),
            VmSlot::Int(100),
            VmSlot::String("key2".into()),
            VmSlot::Int(200),
        ];
        let buffer = TupleListBuffer::new(
            &data,
            2,
            vec![primitive_hashes::STRING, primitive_hashes::INT32],
        );

        let tuple0 = buffer.get_tuple(0).unwrap();
        assert_eq!(tuple0.len(), 2);
        assert!(matches!(&tuple0[0], VmSlot::String(s) if s == "key1"));
        assert!(matches!(tuple0[1], VmSlot::Int(100)));

        let tuple1 = buffer.get_tuple(1).unwrap();
        assert!(matches!(&tuple1[0], VmSlot::String(s) if s == "key2"));
        assert!(matches!(tuple1[1], VmSlot::Int(200)));

        assert!(buffer.get_tuple(2).is_none());
    }

    #[test]
    fn test_tuple_list_buffer_element_types() {
        let buffer = TupleListBuffer::new(
            &[],
            2,
            vec![primitive_hashes::STRING, primitive_hashes::INT32],
        );

        assert_eq!(buffer.element_type(0), Some(primitive_hashes::STRING));
        assert_eq!(buffer.element_type(1), Some(primitive_hashes::INT32));
        assert_eq!(buffer.element_type(2), None);
        assert_eq!(
            buffer.element_types(),
            &[primitive_hashes::STRING, primitive_hashes::INT32]
        );
    }

    #[test]
    fn test_tuple_list_buffer_iter() {
        let data = vec![
            VmSlot::String("a".into()),
            VmSlot::Int(1),
            VmSlot::String("b".into()),
            VmSlot::Int(2),
            VmSlot::String("c".into()),
            VmSlot::Int(3),
        ];
        let buffer = TupleListBuffer::new(
            &data,
            2,
            vec![primitive_hashes::STRING, primitive_hashes::INT32],
        );

        let tuples: Vec<_> = buffer.iter().collect();
        assert_eq!(tuples.len(), 3);
        assert_eq!(tuples[0].len(), 2);
        assert_eq!(tuples[1].len(), 2);
        assert_eq!(tuples[2].len(), 2);
    }

    #[test]
    #[should_panic(expected = "element_types must match tuple_size")]
    fn test_tuple_list_buffer_mismatched_types_panics() {
        TupleListBuffer::new(&[], 2, vec![primitive_hashes::STRING]); // Only 1 type but tuple_size=2
    }

    #[test]
    #[should_panic(expected = "data length must be divisible by tuple_size")]
    fn test_tuple_list_buffer_invalid_data_length_panics() {
        let data = vec![VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3)]; // 3 elements
        TupleListBuffer::new(
            &data,
            2,
            vec![primitive_hashes::INT32, primitive_hashes::INT32],
        ); // tuple_size=2
    }

    // ListPattern tests
    #[test]
    fn test_list_pattern_repeat() {
        let pattern = ListPattern::repeat(primitive_hashes::INT32);

        // Empty list matches
        assert!(pattern.matches(&[]));

        // Single element matches
        assert!(pattern.matches(&[primitive_hashes::INT32]));

        // Multiple elements match
        assert!(pattern.matches(&[
            primitive_hashes::INT32,
            primitive_hashes::INT32,
            primitive_hashes::INT32
        ]));

        // Different type doesn't match
        assert!(!pattern.matches(&[primitive_hashes::STRING]));

        // Mixed types don't match
        assert!(!pattern.matches(&[primitive_hashes::INT32, primitive_hashes::STRING]));
    }

    #[test]
    fn test_list_pattern_fixed() {
        let pattern = ListPattern::fixed(vec![primitive_hashes::INT32, primitive_hashes::STRING]);

        // Exact match
        assert!(pattern.matches(&[primitive_hashes::INT32, primitive_hashes::STRING]));

        // Empty doesn't match
        assert!(!pattern.matches(&[]));

        // Wrong order doesn't match
        assert!(!pattern.matches(&[primitive_hashes::STRING, primitive_hashes::INT32]));

        // Too few elements
        assert!(!pattern.matches(&[primitive_hashes::INT32]));

        // Too many elements
        assert!(!pattern.matches(&[
            primitive_hashes::INT32,
            primitive_hashes::STRING,
            primitive_hashes::INT32
        ]));
    }

    #[test]
    fn test_list_pattern_repeat_tuple() {
        let pattern =
            ListPattern::repeat_tuple(vec![primitive_hashes::STRING, primitive_hashes::INT32]);

        // Empty list matches
        assert!(pattern.matches(&[]));

        // Single tuple matches
        assert!(pattern.matches(&[primitive_hashes::STRING, primitive_hashes::INT32]));

        // Multiple tuples match
        assert!(pattern.matches(&[
            primitive_hashes::STRING,
            primitive_hashes::INT32,
            primitive_hashes::STRING,
            primitive_hashes::INT32,
            primitive_hashes::STRING,
            primitive_hashes::INT32
        ]));

        // Wrong tuple size doesn't match
        assert!(!pattern.matches(&[primitive_hashes::STRING]));

        // Wrong types in tuple don't match
        assert!(!pattern.matches(&[primitive_hashes::INT32, primitive_hashes::STRING]));
    }

    #[test]
    fn test_list_pattern_fixed_empty() {
        let pattern = ListPattern::fixed(vec![]);

        // Empty matches empty
        assert!(pattern.matches(&[]));

        // Non-empty doesn't match
        assert!(!pattern.matches(&[primitive_hashes::INT32]));
    }

    #[test]
    fn test_list_pattern_repeat_tuple_empty() {
        let pattern = ListPattern::repeat_tuple(vec![]);

        // Empty matches empty
        assert!(pattern.matches(&[]));

        // Non-empty doesn't match (can't divide into zero-size tuples)
        // Note: This is a degenerate case
    }

    #[test]
    fn test_list_pattern_constructors() {
        let repeat = ListPattern::repeat(primitive_hashes::FLOAT);
        assert!(matches!(repeat, ListPattern::Repeat(t) if t == primitive_hashes::FLOAT));

        let fixed = ListPattern::fixed(vec![primitive_hashes::INT32, primitive_hashes::STRING]);
        assert!(matches!(fixed, ListPattern::Fixed(ref v) if v.len() == 2));

        let tuple =
            ListPattern::repeat_tuple(vec![primitive_hashes::STRING, primitive_hashes::INT32]);
        assert!(matches!(tuple, ListPattern::RepeatTuple(ref v) if v.len() == 2));
    }
}
