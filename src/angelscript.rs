use crate::context::Context;
use crate::engine::Engine;
use crate::error::{Error, Result};
use crate::lockable_shared_bool::LockableSharedBool;
use crate::thread_manager::{ThreadManager, ExclusiveLockGuard, SharedLockGuard};
use crate::VoidPtr;
use angelscript_bindings::{
    asALLOCFUNC_t, asAllocMem, asAtomicDec, asAtomicInc, asCreateLockableSharedBool,
    asCreateScriptEngine, asDWORD, asFREEFUNC_t, asFreeMem, asGetActiveContext,
    asGetLibraryOptions, asGetLibraryVersion, asGetThreadManager, asResetGlobalMemoryFunctions,
    asSetGlobalMemoryFunctions, ANGELSCRIPT_VERSION
};
use std::ffi::CStr;

/// Wrapper for AngelScript global functions
pub struct AngelScript;

impl AngelScript {
    // ========== ENGINE MANAGEMENT ==========

    /// Creates a new script engine
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
    /// The implementation used depends on the compile-time feature:
    /// - Default: Uses AngelScript's built-in C++ thread manager
    /// - `rust-threading`: Uses a pure Rust implementation
    pub fn prepare_multithread() -> Result<ThreadManager> {
        ThreadManager::prepare()
    }

    /// Unprepares AngelScript from multithreaded use
    pub fn unprepare_multithread() {
        ThreadManager::unprepare()
    }

    /// Gets the current thread manager
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
        ThreadManager::acquire_exclusive_lock()
    }

    /// Releases an exclusive lock
    pub fn release_exclusive_lock() {
        ThreadManager::release_exclusive_lock()
    }

    /// Acquires a shared lock for thread synchronization
    pub fn acquire_shared_lock() {
        ThreadManager::acquire_shared_lock()
    }

    /// Releases a shared lock
    pub fn release_shared_lock() {
        ThreadManager::release_shared_lock()
    }

    /// Creates an exclusive lock guard for RAII locking
    pub fn exclusive_lock() -> ExclusiveLockGuard {
        ExclusiveLockGuard::new()
    }

    /// Creates a shared lock guard for RAII locking
    pub fn shared_lock() -> SharedLockGuard {
        SharedLockGuard::new()
    }

    // ========== ATOMIC OPERATIONS ==========

    /// Atomically increments an integer value
    pub fn atomic_inc(value: &mut i32) -> i32 {
        unsafe { asAtomicInc(value as *mut i32) }
    }

    /// Atomically decrements an integer value
    pub fn atomic_dec(value: &mut i32) -> i32 {
        unsafe { asAtomicDec(value as *mut i32) }
    }

    // ========== THREAD CLEANUP ==========

    /// Performs thread-specific cleanup
    pub fn thread_cleanup() -> Result<()> {
        ThreadManager::cleanup_local_data()
    }

    // ========== MEMORY MANAGEMENT ==========

    /// Sets custom global memory allocation functions
    pub fn set_global_memory_functions(
        alloc_func: asALLOCFUNC_t,
        free_func: asFREEFUNC_t,
    ) -> Result<()> {
        unsafe { Error::from_code(asSetGlobalMemoryFunctions(alloc_func, free_func)) }
    }

    /// Resets global memory functions to default
    pub fn reset_global_memory_functions() -> Result<()> {
        unsafe { Error::from_code(asResetGlobalMemoryFunctions()) }
    }

    /// Allocates memory using AngelScript's allocator
    pub fn alloc_mem(size: usize) -> VoidPtr {
        unsafe { VoidPtr::from_mut_raw(asAllocMem(size)) }
    }

    /// Frees memory allocated by AngelScript's allocator
    pub fn free_mem(mut mem: VoidPtr) {
        unsafe {
            asFreeMem(mem.as_mut_ptr());
        }
    }

    // ========== UTILITY OBJECTS ==========

    /// Creates a new lockable shared boolean
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

    // ========== CONVENIENCE METHODS ==========

    /// Executes a closure with an exclusive lock held
    pub fn with_exclusive_lock<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = Self::exclusive_lock();
        f()
    }

    /// Executes a closure with a shared lock held
    pub fn with_shared_lock<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = Self::shared_lock();
        f()
    }

    /// Checks if AngelScript is prepared for multithreading
    pub fn is_multithreading_prepared() -> bool {
        Self::get_thread_manager().is_some()
    }

    /// Gets information about the current threading setup
    pub fn get_threading_info() -> String {
        match Self::get_thread_manager() {
            Some(manager) => manager.info(),
            None => "No thread manager (single-threaded mode)".to_string(),
        }
    }
}
