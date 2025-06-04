use crate::context::Context;
use crate::engine::Engine;
use crate::error::{Error, Result};
use crate::lockable_shared_bool::LockableSharedBool;
use crate::thread_manager::ThreadManager;
use crate::VoidPtr;
use angelscript_bindings::{asALLOCFUNC_t, asAcquireExclusiveLock, asAcquireSharedLock, asAllocMem, asAtomicDec, asAtomicInc, asCreateLockableSharedBool, asCreateScriptEngine, asDWORD, asFREEFUNC_t, asFreeMem, asGetActiveContext, asGetLibraryOptions, asGetLibraryVersion, asGetThreadManager, asPrepareMultithread, asReleaseExclusiveLock, asReleaseSharedLock, asResetGlobalMemoryFunctions, asSetGlobalMemoryFunctions, asThreadCleanup, asUnprepareMultithread, ANGELSCRIPT_VERSION};
use std::ffi::CStr;
use std::ptr;

/// Wrapper for AngelScript global functions
pub struct AngelScript;

impl AngelScript {
    // ========== ENGINE MANAGEMENT ==========

    /// Creates a new script engine
    ///
    /// # Arguments
    /// * `version` - The AngelScript version to use (use ANGELSCRIPT_VERSION constant)
    ///
    /// # Returns
    /// A new Engine instance or None if creation failed
    pub fn create_script_engine() -> Result<Engine> {
        unsafe {
            let engine_ptr = asCreateScriptEngine(ANGELSCRIPT_VERSION as asDWORD);
            Engine::new(engine_ptr)
        }
    }

    /// Gets the AngelScript library version string
    pub fn get_library_version() -> &'static str {
        unsafe {
            let version_ptr = asGetLibraryVersion();
            CStr::from_ptr(version_ptr).to_str().unwrap_or("Unknown")
        }
    }

    /// Gets the AngelScript library compilation options
    pub fn get_library_options() -> &'static str {
        unsafe {
            let options_ptr = asGetLibraryOptions();
            CStr::from_ptr(options_ptr).to_str().unwrap_or("Unknown")
        }
    }

    // ========== CONTEXT MANAGEMENT ==========

    /// Gets the currently active script context
    ///
    /// # Returns
    /// The active Context or None if no context is active
    pub fn get_active_context() -> Option<Context> {
        unsafe {
            let context_ptr = asGetActiveContext();
            if context_ptr.is_null() {
                None
            } else {
                Some(Context::from_raw(context_ptr))
            }
        }
    }

    // ========== THREADING SUPPORT ==========

    /// Prepares AngelScript for multithreaded use
    ///
    /// # Arguments
    /// * `external_mgr` - Optional external thread manager
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn prepare_multithread(external_mgr: Option<&mut ThreadManager>) -> Result<()> {
        unsafe {
            let mgr_ptr = match external_mgr {
                Some(mgr) => mgr.as_ptr(),
                None => ptr::null_mut(),
            };
            Error::from_code(asPrepareMultithread(mgr_ptr))
        }
    }

    /// Unprepares AngelScript from multithreaded use
    pub fn unprepare_multithread() {
        unsafe {
            asUnprepareMultithread();
        }
    }

    /// Gets the current thread manager
    ///
    /// # Returns
    /// The ThreadManager or None if not available
    pub fn get_thread_manager() -> Option<ThreadManager> {
        unsafe {
            let mgr_ptr = asGetThreadManager();
            if mgr_ptr.is_null() {
                None
            } else {
                Some(ThreadManager::from_raw(mgr_ptr))
            }
        }
    }

    // ========== THREAD SYNCHRONIZATION ==========

    /// Acquires an exclusive lock for thread synchronization
    pub fn acquire_exclusive_lock() {
        unsafe {
            asAcquireExclusiveLock();
        }
    }

    /// Releases an exclusive lock
    pub fn release_exclusive_lock() {
        unsafe {
            asReleaseExclusiveLock();
        }
    }

    /// Acquires a shared lock for thread synchronization
    pub fn acquire_shared_lock() {
        unsafe {
            asAcquireSharedLock();
        }
    }

    /// Releases a shared lock
    pub fn release_shared_lock() {
        unsafe {
            asReleaseSharedLock();
        }
    }

    // ========== ATOMIC OPERATIONS ==========

    /// Atomically increments an integer value
    ///
    /// # Arguments
    /// * `value` - Mutable reference to the value to increment
    ///
    /// # Returns
    /// The new value after increment
    pub fn atomic_inc(value: &mut i32) -> i32 {
        unsafe { asAtomicInc(value as *mut i32) }
    }

    /// Atomically decrements an integer value
    ///
    /// # Arguments
    /// * `value` - Mutable reference to the value to decrement
    ///
    /// # Returns
    /// The new value after decrement
    pub fn atomic_dec(value: &mut i32) -> i32 {
        unsafe { asAtomicDec(value as *mut i32) }
    }

    // ========== THREAD CLEANUP ==========

    /// Performs thread-specific cleanup
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn thread_cleanup() -> Result<()> {
        unsafe { Error::from_code(asThreadCleanup()) }
    }

    // ========== MEMORY MANAGEMENT ==========

    /// Sets custom global memory allocation functions
    ///
    /// # Arguments
    /// * `alloc_func` - Custom allocation function
    /// * `free_func` - Custom deallocation function
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_global_memory_functions(
        alloc_func: asALLOCFUNC_t,
        free_func: asFREEFUNC_t,
    ) -> Result<()> {
        unsafe { Error::from_code(asSetGlobalMemoryFunctions(alloc_func, free_func)) }
    }

    /// Resets global memory functions to default
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn reset_global_memory_functions() -> Result<()> {
        unsafe { Error::from_code(asResetGlobalMemoryFunctions()) }
    }

    /// Allocates memory using AngelScript's allocator
    ///
    /// # Arguments
    /// * `size` - Size in bytes to allocate
    ///
    /// # Returns
    /// Raw pointer to allocated memory or null on failure
    ///
    /// # Safety
    /// The returned pointer must be freed with `free_mem()`
    pub fn alloc_mem(size: usize) -> VoidPtr {
        unsafe { VoidPtr::from_mut_raw(asAllocMem(size)) }
    }

    /// Frees memory allocated by AngelScript's allocator
    ///
    /// # Arguments
    /// * `mem` - Pointer to memory to free
    ///
    /// # Safety
    /// The pointer must have been allocated with `alloc_mem()`
    pub fn free_mem(mut mem: VoidPtr) {
        unsafe {
            asFreeMem(mem.as_mut_ptr());
        }
    }

    // ========== UTILITY OBJECTS ==========

    /// Creates a new lockable shared boolean
    ///
    /// # Returns
    /// A new LockableSharedBool instance or None if creation failed
    pub fn create_lockable_shared_bool() -> Option<LockableSharedBool> {
        unsafe {
            let ptr = asCreateLockableSharedBool();
            if ptr.is_null() {
                None
            } else {
                Some(LockableSharedBool::from_raw(ptr))
            }
        }
    }

    pub fn exclusive_lock() -> ExclusiveLockGuard {
        ExclusiveLockGuard::new()
    }

    /// Creates a shared lock guard for RAII locking
    pub fn shared_lock() -> SharedLockGuard {
        SharedLockGuard::new()
    }
}

// ========== RAII LOCK GUARDS ==========

/// RAII guard for exclusive locks
pub struct ExclusiveLockGuard;

impl ExclusiveLockGuard {
    /// Acquires an exclusive lock and returns a guard
    pub fn new() -> Self {
        AngelScript::acquire_exclusive_lock();
        Self
    }
}

impl Drop for ExclusiveLockGuard {
    fn drop(&mut self) {
        AngelScript::release_exclusive_lock();
    }
}

/// RAII guard for shared locks
pub struct SharedLockGuard;

impl SharedLockGuard {
    /// Acquires a shared lock and returns a guard
    pub fn new() -> Self {
        AngelScript::acquire_shared_lock();
        Self
    }
}

impl Drop for SharedLockGuard {
    fn drop(&mut self) {
        AngelScript::release_shared_lock();
    }
}
