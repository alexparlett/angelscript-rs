use crate::types::script_memory::Void;

/// Trait for types that can be safely converted to and from AngelScript pointers.
///
/// `ScriptData` provides a safe abstraction for passing Rust data to and from
/// AngelScript. It handles the conversion between Rust types and the raw void
/// pointers that AngelScript uses internally.
///
/// # Purpose
///
/// This trait enables:
/// - Safe conversion of Rust types to AngelScript-compatible pointers
/// - Reconstruction of Rust types from AngelScript pointers
/// - Type-safe data exchange between Rust and script code
/// - Memory management coordination between Rust and AngelScript
///
/// # Thread Safety
///
/// All types implementing `ScriptData` must be `Send + Sync` to ensure they
/// can be safely shared between threads, which is essential for AngelScript's
/// threading model.
///
/// # Memory Safety
///
/// The trait methods involve raw pointer operations, so implementations must
/// ensure that:
/// - Pointers remain valid for their expected lifetime
/// - Type information is preserved across conversions
/// - Memory layout is compatible between conversions
///
/// # Examples
///
/// ## Basic Usage with Primitive Types
///
/// ```rust
/// use angelscript_rs::ScriptData;
///
/// // Primitive types automatically implement ScriptData
/// let mut value = 42i32;
/// let ptr = value.to_script_ptr();
///
/// // Convert back from pointer
/// let restored: i32 = ScriptData::from_script_ptr(ptr);
/// assert_eq!(restored, 42);
/// ```
///
/// ## Custom Type Implementation
///
/// ```rust
/// use angelscript_rs::{ScriptData, Void};
/// use std::sync::Arc;
///
/// #[derive(Clone, Debug, PartialEq)]
/// struct MyData {
///     id: u32,
///     name: String,
///     values: Vec<f32>,
/// }
///
/// // MyData automatically implements ScriptData due to the blanket implementation
/// // if it implements Send + Sync
///
/// unsafe impl Send for MyData {}
/// unsafe impl Sync for MyData {}
///
/// // Usage
/// let mut data = MyData {
///     id: 123,
///     name: "test".to_string(),
///     values: vec![1.0, 2.0, 3.0],
/// };
///
/// let ptr = data.to_script_ptr();
/// let restored: MyData = ScriptData::from_script_ptr(ptr);
/// assert_eq!(restored.id, 123);
/// ```
///
/// ## Reference-Counted Types
///
/// ```rust
/// use std::sync::Arc;
///
/// // Arc types work well with ScriptData for shared ownership
/// let data = Arc::new(String::from("shared data"));
/// let mut arc_data = data.clone();
///
/// let ptr = arc_data.to_script_ptr();
/// let restored: Arc<String> = ScriptData::from_script_ptr(ptr);
/// assert_eq!(*restored, "shared data");
/// ```
///
/// ## Integration with AngelScript Objects
///
/// ```rust
/// use angelscript_rs::{Engine, ScriptData};
///
/// #[derive(Debug)]
/// struct GameEntity {
///     position: (f32, f32, f32),
///     health: i32,
///     name: String,
/// }
///
/// unsafe impl Send for GameEntity {}
/// unsafe impl Sync for GameEntity {}
///
/// // Register with AngelScript
/// let engine = Engine::create()?;
///
/// // The ScriptData implementation allows this type to be used
/// // in AngelScript registrations
/// engine.register_object_type("GameEntity",
///                             std::mem::size_of::<GameEntity>() as i32,
///                             ObjectTypeFlags::Value)?;
/// ```
pub trait ScriptData: Send + Sync {
    /// Converts the data to a raw pointer for use with AngelScript.
    ///
    /// This method provides AngelScript with a pointer to the data that it can
    /// store and pass around. The pointer must remain valid for as long as
    /// AngelScript holds a reference to it.
    ///
    /// # Safety Considerations
    ///
    /// - The returned pointer must point to valid memory
    /// - The memory layout must be stable (no moves after pointer creation)
    /// - The pointer must remain valid until AngelScript releases it
    ///
    /// # Returns
    /// A raw mutable pointer to the data
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut value = 42i32;
    /// let ptr = value.to_script_ptr();
    ///
    /// // ptr now points to the memory location of value
    /// // AngelScript can use this pointer to access the data
    /// ```
    ///
    /// ## Custom Implementation Example
    ///
    /// ```rust
    /// use angelscript_rs::{ScriptData, Void};
    ///
    /// struct CustomType {
    ///     data: Box<[u8]>,
    /// }
    ///
    /// unsafe impl Send for CustomType {}
    /// unsafe impl Sync for CustomType {}
    ///
    /// impl ScriptData for CustomType {
    ///     fn to_script_ptr(&mut self) -> *mut Void {
    ///         // Return pointer to the boxed data
    ///         self.data.as_mut_ptr() as *mut Void
    ///     }
    ///
    ///     fn from_script_ptr(ptr: *mut Void) -> Self {
    ///         unsafe {
    ///             // Reconstruct from pointer
    ///             // This is a simplified example - real implementation
    ///             // would need proper memory management
    ///             let data_ptr = ptr as *mut u8;
    ///             let data = Box::from_raw(std::slice::from_raw_parts_mut(data_ptr, 0));
    ///             CustomType { data }
    ///         }
    ///     }
    /// }
    /// ```
    fn to_script_ptr(&mut self) -> *mut Void;

    /// Reconstructs the data from a raw pointer provided by AngelScript.
    ///
    /// This method takes a pointer that was previously created by `to_script_ptr`
    /// and reconstructs the original Rust type. The implementation must ensure
    /// that the reconstruction is safe and produces a valid value.
    ///
    /// # Safety Considerations
    ///
    /// - The pointer must have been created by a previous call to `to_script_ptr`
    /// - The pointer must point to valid memory of the correct type
    /// - The memory must not have been freed or invalidated
    /// - The type must match exactly what was originally converted
    ///
    /// # Arguments
    /// * `ptr` - Raw pointer to the data, as provided by AngelScript
    ///
    /// # Returns
    /// The reconstructed Rust value
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut original = 42i32;
    /// let ptr = original.to_script_ptr();
    ///
    /// // Later, reconstruct from the pointer
    /// let reconstructed: i32 = ScriptData::from_script_ptr(ptr);
    /// assert_eq!(reconstructed, 42);
    /// ```
    ///
    /// ## Error Handling
    ///
    /// ```rust
    /// // This would be unsafe and could cause undefined behavior:
    /// // let invalid_ptr = std::ptr::null_mut();
    /// // let value: i32 = ScriptData::from_script_ptr(invalid_ptr); // DON'T DO THIS
    ///
    /// // Always ensure pointers are valid before conversion
    /// fn safe_conversion(ptr: *mut Void) -> Option<i32> {
    ///     if ptr.is_null() {
    ///         None
    ///     } else {
    ///         Some(ScriptData::from_script_ptr(ptr))
    ///     }
    /// }
    /// ```
    fn from_script_ptr(ptr: *mut Void) -> Self
    where
        Self: Sized;
}

/// Blanket implementation of ScriptData for all sized types that are Send + Sync.
///
/// This implementation provides automatic `ScriptData` support for most Rust types
/// by using direct pointer operations. It works by:
///
/// 1. **`to_script_ptr`**: Returns a pointer to the value's memory location
/// 2. **`from_script_ptr`**: Reads the value from the pointer location
///
/// # Supported Types
///
/// This implementation works for:
/// - All primitive types (`i32`, `f64`, `bool`, etc.)
/// - Structs and enums that implement `Send + Sync`
/// - Standard library types like `String`, `Vec<T>`, `HashMap<K,V>`, etc.
/// - Custom types that implement `Send + Sync`
///
/// # Memory Management
///
/// The blanket implementation uses `ptr.read()` which performs a bitwise copy
/// of the data. This means:
/// - The original data is consumed/moved
/// - No reference counting or shared ownership
/// - Suitable for value types and owned data
///
/// # Examples
///
/// ## Primitive Types
///
/// ```rust
/// // All these types automatically implement ScriptData
/// let mut int_val = 42i32;
/// let mut float_val = 3.14f64;
/// let mut bool_val = true;
/// let mut char_val = 'A';
///
/// // Convert to pointers
/// let int_ptr = int_val.to_script_ptr();
/// let float_ptr = float_val.to_script_ptr();
/// let bool_ptr = bool_val.to_script_ptr();
/// let char_ptr = char_val.to_script_ptr();
///
/// // Convert back
/// let restored_int: i32 = ScriptData::from_script_ptr(int_ptr);
/// let restored_float: f64 = ScriptData::from_script_ptr(float_ptr);
/// let restored_bool: bool = ScriptData::from_script_ptr(bool_ptr);
/// let restored_char: char = ScriptData::from_script_ptr(char_ptr);
/// ```
///
/// ## Compound Types
///
/// ```rust
/// #[derive(Debug, PartialEq)]
/// struct Point {
///     x: f32,
///     y: f32,
/// }
///
/// unsafe impl Send for Point {}
/// unsafe impl Sync for Point {}
///
/// let mut point = Point { x: 1.0, y: 2.0 };
/// let ptr = point.to_script_ptr();
/// let restored: Point = ScriptData::from_script_ptr(ptr);
/// assert_eq!(restored, Point { x: 1.0, y: 2.0 });
/// ```
///
/// ## Standard Library Types
///
/// ```rust
/// // String
/// let mut text = String::from("Hello, AngelScript!");
/// let ptr = text.to_script_ptr();
/// let restored: String = ScriptData::from_script_ptr(ptr);
/// assert_eq!(restored, "Hello, AngelScript!");
///
/// // Vec
/// let mut numbers = vec![1, 2, 3, 4, 5];
/// let ptr = numbers.to_script_ptr();
/// let restored: Vec<i32> = ScriptData::from_script_ptr(ptr);
/// assert_eq!(restored, vec![1, 2, 3, 4, 5]);
/// ```
///
/// # Limitations
///
/// This blanket implementation may not be suitable for:
/// - Types requiring custom cleanup logic
/// - Reference-counted types that need special handling
/// - Types with complex internal pointers or references
/// - Types that need custom serialization/deserialization
///
/// For such types, implement `ScriptData` manually:
///
/// ```rust
/// use std::sync::Arc;
///
/// struct ComplexType {
///     data: Arc<Vec<String>>,
/// }
///
/// unsafe impl Send for ComplexType {}
/// unsafe impl Sync for ComplexType {}
///
/// impl ScriptData for ComplexType {
///     fn to_script_ptr(&mut self) -> *mut Void {
///         // Custom implementation for reference-counted data
///         Arc::as_ptr(&self.data) as *mut Void
///     }
///
///     fn from_script_ptr(ptr: *mut Void) -> Self {
///         unsafe {
///             // Custom reconstruction logic
///             let arc_ptr = ptr as *const Vec<String>;
///             let data = Arc::from_raw(arc_ptr);
///             ComplexType { data }
///         }
///     }
/// }
/// ```
impl<T: Sized + Send + Sync> ScriptData for T {
    /// Converts the value to a script pointer by returning its memory address.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut value = 42i32;
    /// let ptr = value.to_script_ptr();
    /// // ptr now points to the memory location of value
    /// ```
    fn to_script_ptr(&mut self) -> *mut Void {
        self as *mut T as *mut Void
    }

    /// Reconstructs the value by reading from the pointer location.
    ///
    /// # Safety
    /// This performs a `ptr.read()` which moves the value from the pointer
    /// location. The pointer should not be used after this operation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut original = 42i32;
    /// let ptr = original.to_script_ptr();
    /// let restored: i32 = ScriptData::from_script_ptr(ptr);
    /// assert_eq!(restored, 42);
    /// ```
    fn from_script_ptr(ptr: *mut Void) -> Self {
        unsafe { (ptr as *mut T).read() }
    }
}
