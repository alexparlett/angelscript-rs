use crate::core::error::{ScriptError, ScriptResult};
use angelscript_sys::{asILockableSharedBool, asILockableSharedBool__bindgen_vtable};

/// A thread-safe, reference-counted boolean value.
///
/// `LockableSharedBool` provides a thread-safe boolean that can be shared between
/// multiple threads and contexts. It's primarily used internally by AngelScript
/// for implementing weak references and other synchronization primitives.
///
/// # Thread Safety
///
/// This type is fully thread-safe and implements both `Send` and `Sync`. Multiple
/// threads can safely read, write, and lock the boolean value concurrently.
///
/// # Memory Management
///
/// The boolean uses reference counting for memory management. When the last
/// reference is dropped, the underlying memory is automatically freed.
///
/// # Locking
///
/// The boolean can be locked to ensure atomic read-modify-write operations.
/// For convenience, use the `lock_guard()` method which provides RAII-style
/// locking that automatically unlocks when the guard is dropped.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// // Create a lockable shared boolean
/// let shared_bool = Engine::create_lockable_shared_bool()
///     .expect("Failed to create lockable shared bool");
///
/// // Set and get values
/// shared_bool.set(true);
/// assert_eq!(shared_bool.get(), true);
///
/// shared_bool.set(false);
/// assert_eq!(shared_bool.get(), false);
/// ```
///
/// ## Thread-Safe Operations
///
/// ```rust
/// use std::sync::Arc;
/// use std::thread;
///
/// let shared_bool = Arc::new(
///     Engine::create_lockable_shared_bool()
///         .expect("Failed to create lockable shared bool")
/// );
///
/// // Clone for use in another thread
/// let shared_bool_clone = Arc::clone(&shared_bool);
///
/// let handle = thread::spawn(move || {
///     // Use RAII lock guard for thread-safe operations
///     let guard = shared_bool_clone.lock_guard();
///     if !guard.get() {
///         guard.set(true);
///     }
///     // Lock is automatically released when guard is dropped
/// });
///
/// handle.join().unwrap();
/// ```
///
/// ## Manual Locking
///
/// ```rust
/// // Manual lock/unlock (not recommended - use lock_guard() instead)
/// shared_bool.lock();
/// let current_value = shared_bool.get();
/// shared_bool.set(!current_value);
/// shared_bool.unlock();
/// ```
#[derive(Debug)]
pub struct LockableSharedBool {
    inner: *mut asILockableSharedBool,
}

impl LockableSharedBool {
    /// Creates a LockableSharedBool wrapper from a raw AngelScript pointer.
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid and the object is properly initialized.
    ///
    /// # Arguments
    /// * `ptr` - Raw pointer to AngelScript lockable shared bool
    ///
    /// # Returns
    /// A new LockableSharedBool wrapper
    pub(crate) fn from_raw(ptr: *mut asILockableSharedBool) -> Self {
        let wrapper = Self { inner: ptr };
        wrapper
            .add_ref()
            .expect("Failed to add reference to LockableSharedBool");
        wrapper
    }

    // ========== VTABLE ORDER (matches asILockableSharedBool__bindgen_vtable) ==========

    /// Increments the reference count.
    ///
    /// This is called automatically when cloning or sharing the boolean.
    /// Manual calls are rarely needed.
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// let shared_bool = Engine::create_lockable_shared_bool()?;
    /// shared_bool.add_ref()?; // Manually increment reference count
    /// // Remember to call release() to balance this
    /// ```
    pub fn add_ref(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asILockableSharedBool_AddRef)(self.inner)) }
    }

    /// Decrements the reference count.
    ///
    /// When the reference count reaches zero, the object is destroyed.
    /// This is called automatically when the wrapper is dropped.
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// let shared_bool = Engine::create_lockable_shared_bool()?;
    /// // shared_bool.release() is called automatically when dropped
    /// ```
    pub fn release(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asILockableSharedBool_Release)(self.inner)) }
    }

    /// Gets the current boolean value.
    ///
    /// This operation is atomic and thread-safe without requiring explicit locking
    /// for simple reads. However, for read-modify-write operations, use locking
    /// to ensure atomicity.
    ///
    /// # Returns
    /// The current boolean value
    ///
    /// # Examples
    ///
    /// ```rust
    /// let shared_bool = Engine::create_lockable_shared_bool()?;
    /// shared_bool.set(true);
    ///
    /// assert_eq!(shared_bool.get(), true);
    /// ```
    ///
    /// # Thread Safety
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use std::thread;
    ///
    /// let shared_bool = Arc::new(Engine::create_lockable_shared_bool()?);
    /// let shared_bool_clone = Arc::clone(&shared_bool);
    ///
    /// thread::spawn(move || {
    ///     let value = shared_bool_clone.get(); // Thread-safe read
    ///     println!("Value from thread: {}", value);
    /// });
    /// ```
    pub fn get(&self) -> bool {
        unsafe { (self.as_vtable().asILockableSharedBool_Get)(self.inner) }
    }

    /// Sets the boolean value.
    ///
    /// This operation is atomic and thread-safe. The new value will be
    /// immediately visible to all threads.
    ///
    /// # Arguments
    /// * `value` - The new boolean value to set
    ///
    /// # Examples
    ///
    /// ```rust
    /// let shared_bool = Engine::create_lockable_shared_bool()?;
    ///
    /// shared_bool.set(true);
    /// assert_eq!(shared_bool.get(), true);
    ///
    /// shared_bool.set(false);
    /// assert_eq!(shared_bool.get(), false);
    /// ```
    ///
    /// # Thread Safety
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use std::thread;
    ///
    /// let shared_bool = Arc::new(Engine::create_lockable_shared_bool()?);
    /// let shared_bool_clone = Arc::clone(&shared_bool);
    ///
    /// thread::spawn(move || {
    ///     shared_bool_clone.set(true); // Thread-safe write
    /// });
    /// ```
    pub fn set(&self, value: bool) {
        unsafe { (self.as_vtable().asILockableSharedBool_Set)(self.inner, value) }
    }

    /// Acquires an exclusive lock on the boolean.
    ///
    /// This lock ensures that no other thread can read or write the boolean
    /// until `unlock()` is called. Use this for atomic read-modify-write operations.
    ///
    /// **Warning**: Always pair `lock()` with `unlock()` to avoid deadlocks.
    /// Consider using `lock_guard()` instead for RAII-style locking.
    ///
    /// # Deadlock Prevention
    ///
    /// - Always unlock in the same thread that acquired the lock
    /// - Don't hold locks longer than necessary
    /// - Avoid nested locking of the same boolean
    /// - Use `lock_guard()` for automatic unlock on scope exit
    ///
    /// # Examples
    ///
    /// ```rust
    /// let shared_bool = Engine::create_lockable_shared_bool()?;
    ///
    /// // Manual locking (not recommended)
    /// shared_bool.lock();
    /// let current = shared_bool.get();
    /// shared_bool.set(!current); // Atomic toggle
    /// shared_bool.unlock();
    ///
    /// // Preferred: use lock_guard() for RAII
    /// {
    ///     let guard = shared_bool.lock_guard();
    ///     let current = guard.get();
    ///     guard.set(!current);
    /// } // Automatically unlocked here
    /// ```
    pub fn lock(&self) {
        unsafe { (self.as_vtable().asILockableSharedBool_Lock)(self.inner) }
    }

    /// Releases the exclusive lock on the boolean.
    ///
    /// This must be called after `lock()` to allow other threads to access
    /// the boolean. Failing to call `unlock()` will cause deadlocks.
    ///
    /// **Warning**: Only call `unlock()` if you previously called `lock()`.
    /// Calling `unlock()` without a corresponding `lock()` is undefined behavior.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let shared_bool = Engine::create_lockable_shared_bool()?;
    ///
    /// shared_bool.lock();
    /// // ... perform atomic operations ...
    /// shared_bool.unlock(); // Must be called to release the lock
    /// ```
    pub fn unlock(&self) {
        unsafe { (self.as_vtable().asILockableSharedBool_Unlock)(self.inner) }
    }

    /// Gets the vtable for the underlying AngelScript object.
    ///
    /// # Safety
    /// This function provides direct access to the AngelScript vtable.
    fn as_vtable(&self) -> &asILockableSharedBool__bindgen_vtable {
        unsafe { &*(*self.inner).vtable_ }
    }
}

impl Drop for LockableSharedBool {
    fn drop(&mut self) {
        self.release()
            .expect("Failed to release LockableSharedBool");
    }
}

unsafe impl Send for LockableSharedBool {}
unsafe impl Sync for LockableSharedBool {}

/// RAII guard for automatic lock management.
///
/// This guard automatically acquires a lock when created and releases it when dropped,
/// ensuring that locks are always properly released even if an error occurs or an
/// early return happens.
///
/// # Examples
///
/// ```rust
/// let shared_bool = Engine::create_lockable_shared_bool()?;
///
/// {
///     let guard = shared_bool.lock_guard();
///
///     // Perform atomic operations
///     if guard.get() {
///         guard.set(false);
///     } else {
///         guard.set(true);
///     }
///
///     // Lock is automatically released when guard goes out of scope
/// }
///
/// // Boolean is now unlocked and available to other threads
/// ```
///
/// # Panic Safety
///
/// The guard will properly unlock even if a panic occurs:
///
/// ```rust
/// let shared_bool = Engine::create_lockable_shared_bool()?;
///
/// let result = std::panic::catch_unwind(|| {
///     let guard = shared_bool.lock_guard();
///     guard.set(true);
///     panic!("Something went wrong!");
///     // Guard's Drop implementation still runs, unlocking the boolean
/// });
///
/// // Boolean is properly unlocked despite the panic
/// assert!(result.is_err());
/// ```
pub struct LockableSharedBoolGuard<'a> {
    bool_ref: &'a LockableSharedBool,
}

impl<'a> LockableSharedBoolGuard<'a> {
    /// Creates a new guard and immediately acquires the lock.
    ///
    /// # Arguments
    /// * `bool_ref` - Reference to the boolean to lock
    ///
    /// # Returns
    /// A new guard that holds the lock
    ///
    /// # Examples
    ///
    /// ```rust
    /// let shared_bool = Engine::create_lockable_shared_bool()?;
    /// let guard = LockableSharedBoolGuard::new(&shared_bool);
    /// // Boolean is now locked
    /// ```
    pub fn new(bool_ref: &'a LockableSharedBool) -> Self {
        bool_ref.lock();
        Self { bool_ref }
    }

    /// Gets the boolean value while the lock is held.
    ///
    /// # Returns
    /// The current boolean value
    ///
    /// # Examples
    ///
    /// ```rust
    /// let shared_bool = Engine::create_lockable_shared_bool()?;
    /// let guard = shared_bool.lock_guard();
    /// let value = guard.get();
    /// println!("Locked value: {}", value);
    /// ```
    pub fn get(&self) -> bool {
        self.bool_ref.get()
    }

    /// Sets the boolean value while the lock is held.
    ///
    /// # Arguments
    /// * `value` - The new boolean value
    ///
    /// # Examples
    ///
    /// ```rust
    /// let shared_bool = Engine::create_lockable_shared_bool()?;
    /// let guard = shared_bool.lock_guard();
    /// guard.set(true);
    /// ```
    pub fn set(&self, value: bool) {
        self.bool_ref.set(value)
    }
}

impl<'a> Drop for LockableSharedBoolGuard<'a> {
    fn drop(&mut self) {
        self.bool_ref.unlock();
    }
}

// Extension trait for convenient RAII locking
impl LockableSharedBool {
    /// Creates a RAII guard that locks the boolean and automatically unlocks on drop.
    ///
    /// This is the preferred way to perform atomic operations on the boolean,
    /// as it ensures the lock is always released even if an error occurs.
    ///
    /// # Returns
    /// A guard that holds the lock until dropped
    ///
    /// # Examples
    ///
    /// ## Basic Usage
    ///
    /// ```rust
    /// let shared_bool = Engine::create_lockable_shared_bool()?;
    ///
    /// {
    ///     let guard = shared_bool.lock_guard();
    ///     let current = guard.get();
    ///     guard.set(!current); // Atomic toggle
    /// } // Lock automatically released here
    /// ```
    ///
    /// ## Conditional Operations
    ///
    /// ```rust
    /// let shared_bool = Engine::create_lockable_shared_bool()?;
    ///
    /// let guard = shared_bool.lock_guard();
    /// if guard.get() {
    ///     println!("Boolean is true, setting to false");
    ///     guard.set(false);
    /// } else {
    ///     println!("Boolean is false, setting to true");
    ///     guard.set(true);
    /// }
    /// ```
    ///
    /// ## Error Handling
    ///
    /// ```rust
    /// let shared_bool = Engine::create_lockable_shared_bool()?;
    ///
    /// let result = (|| -> Result<(), Box<dyn std::error::Error>> {
    ///     let guard = shared_bool.lock_guard();
    ///
    ///     if guard.get() {
    ///         return Err("Boolean was already true".into());
    ///     }
    ///
    ///     guard.set(true);
    ///     Ok(())
    /// })();
    ///
    /// // Lock is released regardless of whether an error occurred
    /// match result {
    ///     Ok(()) => println!("Operation succeeded"),
    ///     Err(e) => println!("Operation failed: {}", e),
    /// }
    /// ```
    pub fn lock_guard(&self) -> LockableSharedBoolGuard {
        LockableSharedBoolGuard::new(self)
    }
}
