use crate::types::script_data::ScriptData;
use std::ffi::c_void;

/// Type alias for void pointers used in script memory operations.
///
/// This provides a more semantic name for `c_void` when used in the context
/// of AngelScript memory management.
pub type Void = c_void;

/// A safe wrapper around raw memory pointers used by AngelScript.
///
/// `ScriptMemoryLocation` provides a type-safe interface for working with memory
/// pointers that are passed between Rust and AngelScript. It handles the conversion
/// between Rust types and the raw pointers expected by the AngelScript C API.
///
/// # Memory Management
///
/// This type supports several memory management patterns:
/// - **Boxed Values**: Heap-allocated Rust values with reference counting
/// - **Raw Pointers**: Direct pointer manipulation for C interop
/// - **Null Pointers**: Safe handling of null pointer cases
///
/// # Thread Safety
///
/// This type implements `Send` and `Sync`, making it safe to share between threads.
/// However, the safety of the underlying data depends on the specific type being
/// pointed to and how it's used.
///
/// # Examples
///
/// ## Basic Pointer Operations
///
/// ```rust
/// use angelscript_rs::ScriptMemoryLocation;
///
/// // Create a null pointer
/// let null_ptr = ScriptMemoryLocation::null();
/// assert!(null_ptr.is_null());
///
/// // Create from a raw pointer
/// let value = 42i32;
/// let ptr = ScriptMemoryLocation::from_const(&value as *const _ as *const std::ffi::c_void);
/// assert!(!ptr.is_null());
/// ```
///
/// ## Boxed Value Management
///
/// ```rust
/// use std::sync::atomic::AtomicUsize;
///
/// // Create a boxed value with reference counting
/// let data = String::from("Hello, AngelScript!");
/// let memory_loc = ScriptMemoryLocation::from_boxed(data);
/// let ref_count = AtomicUsize::new(1);
///
/// // Access the boxed value
/// let text_ref = memory_loc.as_boxed_ref::<String>();
/// println!("Stored text: {}", text_ref);
///
/// // Add a reference
/// memory_loc.addref_boxed(&ref_count);
///
/// // Release a reference (unsafe - only when you know it's safe)
/// unsafe {
///     let was_freed = memory_loc.release_boxed::<String>(&ref_count);
///     if was_freed {
///         println!("Object was freed");
///     }
/// }
/// ```
///
/// ## Type Conversion
///
/// ```rust
/// // Store a value
/// let mut memory_loc = ScriptMemoryLocation::from_boxed(42i32);
///
/// // Read it back
/// let value_ref = memory_loc.as_ref::<i32>();
/// assert_eq!(*value_ref, 42);
///
/// // Modify it
/// let value_mut = memory_loc.as_ref_mut::<i32>();
/// *value_mut = 100;
/// assert_eq!(*memory_loc.as_ref::<i32>(), 100);
/// ```
#[derive(Hash, PartialEq, Eq, Copy, Clone, Debug)]
pub struct ScriptMemoryLocation(*mut Void);

impl ScriptMemoryLocation {
    /// Checks if the pointer is null.
    ///
    /// # Returns
    /// `true` if the pointer is null, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// let null_ptr = ScriptMemoryLocation::null();
    /// assert!(null_ptr.is_null());
    ///
    /// let valid_ptr = ScriptMemoryLocation::from_boxed(42);
    /// assert!(!valid_ptr.is_null());
    /// ```
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    /// Creates a null pointer.
    ///
    /// # Returns
    /// A `ScriptMemoryLocation` containing a null pointer
    ///
    /// # Examples
    ///
    /// ```rust
    /// let null_ptr = ScriptMemoryLocation::null();
    /// assert!(null_ptr.is_null());
    /// ```
    pub fn null() -> Self {
        ScriptMemoryLocation(std::ptr::null_mut())
    }

    /// Creates a memory location by boxing a value on the heap.
    ///
    /// This takes ownership of the value, moves it to the heap, and returns
    /// a memory location pointing to it. The value will be managed with
    /// reference counting when used with `addref_boxed` and `release_boxed`.
    ///
    /// # Arguments
    /// * `value` - The value to box and store
    ///
    /// # Returns
    /// A memory location pointing to the boxed value
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Box a simple value
    /// let memory_loc = ScriptMemoryLocation::from_boxed(42i32);
    /// let value = memory_loc.as_boxed_ref::<i32>();
    /// assert_eq!(*value, 42);
    ///
    /// // Box a complex type
    /// #[derive(Debug, PartialEq)]
    /// struct MyData {
    ///     name: String,
    ///     value: i32,
    /// }
    ///
    /// let data = MyData {
    ///     name: "test".to_string(),
    ///     value: 123,
    /// };
    /// let memory_loc = ScriptMemoryLocation::from_boxed(data);
    /// let data_ref = memory_loc.as_boxed_ref::<MyData>();
    /// assert_eq!(data_ref.name, "test");
    /// assert_eq!(data_ref.value, 123);
    /// ```
    pub fn from_boxed<T>(value: T) -> Self {
        let boxed = Box::new(value);
        let ptr = Box::into_raw(boxed);
        Self::from_const(ptr as *mut std::ffi::c_void)
    }

    /// Gets an immutable reference to a boxed value.
    ///
    /// # Safety
    /// This method assumes the pointer was created with `from_boxed` and
    /// points to a valid value of type `T`. Using this with an incorrect
    /// type or invalid pointer will cause undefined behavior.
    ///
    /// # Arguments
    /// * `T` - The type of the boxed value
    ///
    /// # Returns
    /// An immutable reference to the boxed value
    ///
    /// # Examples
    ///
    /// ```rust
    /// let memory_loc = ScriptMemoryLocation::from_boxed(String::from("hello"));
    /// let text = memory_loc.as_boxed_ref::<String>();
    /// assert_eq!(text, "hello");
    /// ```
    pub fn as_boxed_ref<T>(&self) -> &T {
        unsafe {
            let ptr = self.as_ptr() as *const T;
            &*ptr
        }
    }

    /// Gets a mutable reference to a boxed value.
    ///
    /// # Safety
    /// This method assumes the pointer was created with `from_boxed` and
    /// points to a valid value of type `T`. Using this with an incorrect
    /// type or invalid pointer will cause undefined behavior.
    ///
    /// # Arguments
    /// * `T` - The type of the boxed value
    ///
    /// # Returns
    /// A mutable reference to the boxed value
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut memory_loc = ScriptMemoryLocation::from_boxed(String::from("hello"));
    /// {
    ///     let text = memory_loc.as_boxed_ref_mut::<String>();
    ///     text.push_str(" world");
    /// }
    /// let text = memory_loc.as_boxed_ref::<String>();
    /// assert_eq!(text, "hello world");
    /// ```
    pub fn as_boxed_ref_mut<T>(&mut self) -> &mut T {
        unsafe {
            let ptr = self.as_ptr() as *mut T;
            &mut *ptr
        }
    }

    /// Releases a reference to a boxed value with atomic reference counting.
    ///
    /// This decrements the reference count and frees the memory if the count
    /// reaches zero. This is typically used in AngelScript object release
    /// behaviors.
    ///
    /// # Safety
    /// This method is unsafe because:
    /// - It assumes the pointer was created with `from_boxed`
    /// - It assumes the reference count accurately reflects the number of references
    /// - After this returns `true`, the pointer becomes invalid
    /// - The caller must ensure no other references to the data exist when freed
    ///
    /// # Arguments
    /// * `ref_count` - Atomic reference counter for the object
    ///
    /// # Returns
    /// `true` if the object was freed, `false` if references still exist
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::sync::atomic::AtomicUsize;
    ///
    /// let memory_loc = ScriptMemoryLocation::from_boxed(String::from("test"));
    /// let ref_count = AtomicUsize::new(2); // Assume 2 references
    ///
    /// unsafe {
    ///     // First release - object not freed
    ///     let freed = memory_loc.release_boxed::<String>(&ref_count);
    ///     assert!(!freed);
    ///
    ///     // Second release - object freed
    ///     let freed = memory_loc.release_boxed::<String>(&ref_count);
    ///     assert!(freed);
    ///     // memory_loc is now invalid!
    /// }
    /// ```
    pub unsafe fn release_boxed<T>(&self, ref_count: &std::sync::atomic::AtomicUsize) -> bool {
        unsafe {
            let count = ref_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed) - 1;

            if count == 0 {
                let ptr = self.as_ptr() as *mut T;
                let _boxed = Box::from_raw(ptr);
                true
            } else {
                false
            }
        }
    }

    /// Adds a reference to a boxed value with atomic reference counting.
    ///
    /// This increments the reference count for the object, indicating that
    /// another reference to the data exists. This is typically used in
    /// AngelScript object addref behaviors.
    ///
    /// # Arguments
    /// * `ref_count` - Atomic reference counter for the object
    ///
    /// # Returns
    /// The new reference count after incrementing
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::sync::atomic::AtomicUsize;
    ///
    /// let memory_loc = ScriptMemoryLocation::from_boxed(String::from("test"));
    /// let ref_count = AtomicUsize::new(1);
    ///
    /// let new_count = memory_loc.addref_boxed(&ref_count);
    /// assert_eq!(new_count, 2);
    ///
    /// let new_count = memory_loc.addref_boxed(&ref_count);
    /// assert_eq!(new_count, 3);
    /// ```
    pub fn addref_boxed(&self, ref_count: &std::sync::atomic::AtomicUsize) -> usize {
        ref_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
    }

    /// Creates a memory location from a mutable raw pointer.
    ///
    /// # Arguments
    /// * `ptr` - The raw mutable pointer
    ///
    /// # Returns
    /// A memory location wrapping the pointer
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut value = 42i32;
    /// let ptr = &mut value as *mut _ as *mut std::ffi::c_void;
    /// let memory_loc = ScriptMemoryLocation::from_mut(ptr);
    /// assert!(!memory_loc.is_null());
    /// ```
    pub fn from_mut(ptr: *mut Void) -> Self {
        ScriptMemoryLocation(ptr)
    }

    /// Creates a memory location from a const raw pointer.
    ///
    /// # Arguments
    /// * `ptr` - The raw const pointer
    ///
    /// # Returns
    /// A memory location wrapping the pointer
    ///
    /// # Examples
    ///
    /// ```rust
    /// let value = 42i32;
    /// let ptr = &value as *const _ as *const std::ffi::c_void;
    /// let memory_loc = ScriptMemoryLocation::from_const(ptr);
    /// assert!(!memory_loc.is_null());
    /// ```
    pub fn from_const(ptr: *const Void) -> Self {
        ScriptMemoryLocation(ptr as *mut Void)
    }

    /// Gets the raw pointer as a const pointer.
    ///
    /// # Returns
    /// The underlying const pointer
    ///
    /// # Examples
    ///
    /// ```rust
    /// let memory_loc = ScriptMemoryLocation::null();
    /// let ptr = memory_loc.as_ptr();
    /// assert!(ptr.is_null());
    /// ```
    pub fn as_ptr(&self) -> *const Void {
        self.0
    }

    /// Gets the raw pointer as a mutable pointer.
    ///
    /// # Returns
    /// The underlying mutable pointer
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut memory_loc = ScriptMemoryLocation::null();
    /// let ptr = memory_loc.as_mut_ptr();
    /// assert!(ptr.is_null());
    /// ```
    pub fn as_mut_ptr(&mut self) -> *mut Void {
        self.0
    }

    /// Writes a value to the memory location.
    ///
    /// # Safety
    /// This method is unsafe because it writes directly to the pointer without
    /// checking if it's valid or if there's enough space for the value.
    ///
    /// # Arguments
    /// * `value` - The value to write
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut value = 0i32;
    /// let ptr = &mut value as *mut _ as *mut std::ffi::c_void;
    /// let mut memory_loc = ScriptMemoryLocation::from_mut(ptr);
    ///
    /// memory_loc.set(42i32);
    /// assert_eq!(value, 42);
    /// ```
    pub fn set<T>(&mut self, value: T) {
        unsafe {
            self.0.cast::<T>().write(value);
        }
    }

    /// Reads a value from the memory location using ScriptData conversion.
    ///
    /// This method uses the `ScriptData` trait to convert the raw pointer
    /// back to a Rust type. It panics if the pointer is null.
    ///
    /// # Panics
    /// Panics if the pointer is null.
    ///
    /// # Arguments
    /// * `T` - The type to read, must implement `ScriptData`
    ///
    /// # Returns
    /// The value read from memory
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Assuming MyType implements ScriptData
    /// let memory_loc = ScriptMemoryLocation::from_boxed(MyType::new());
    /// let value: MyType = memory_loc.read();
    /// ```
    pub fn read<T: ScriptData>(&self) -> T {
        assert!(!self.is_null(), "Tried to access a null Ptr");
        ScriptData::from_script_ptr(self.0)
    }

    /// Gets an immutable reference to the value at this memory location.
    ///
    /// # Panics
    /// Panics if the pointer is null.
    ///
    /// # Safety
    /// This method assumes the pointer points to a valid value of type `T`.
    /// Using an incorrect type will cause undefined behavior.
    ///
    /// # Arguments
    /// * `T` - The type to interpret the memory as
    ///
    /// # Returns
    /// An immutable reference to the value
    ///
    /// # Examples
    ///
    /// ```rust
    /// let value = 42i32;
    /// let ptr = &value as *const _ as *const std::ffi::c_void;
    /// let memory_loc = ScriptMemoryLocation::from_const(ptr);
    ///
    /// let value_ref = memory_loc.as_ref::<i32>();
    /// assert_eq!(*value_ref, 42);
    /// ```
    pub fn as_ref<T>(&self) -> &T {
        // Null pointer check
        assert!(!self.is_null(), "Tried to access a null Ptr");
        unsafe { self.0.cast::<T>().as_ref().unwrap() }
    }

    /// Gets a mutable reference to the value at this memory location.
    ///
    /// # Panics
    /// Panics if the pointer is null.
    ///
    /// # Safety
    /// This method assumes the pointer points to a valid value of type `T`.
    /// Using an incorrect type will cause undefined behavior.
    ///
    /// # Arguments
    /// * `T` - The type to interpret the memory as
    ///
    /// # Returns
    /// A mutable reference to the value
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut value = 42i32;
    /// let ptr = &mut value as *mut _ as *mut std::ffi::c_void;
    /// let mut memory_loc = ScriptMemoryLocation::from_mut(ptr);
    ///
    /// let value_ref = memory_loc.as_ref_mut::<i32>();
    /// *value_ref = 100;
    /// assert_eq!(value, 100);
    /// ```
    pub fn as_ref_mut<T>(&mut self) -> &mut T {
        assert!(!self.is_null(), "Tried to access a null Ptr");
        unsafe { self.0.cast::<T>().as_mut().unwrap() }
    }
}

unsafe impl Send for ScriptMemoryLocation {}
unsafe impl Sync for ScriptMemoryLocation {}
