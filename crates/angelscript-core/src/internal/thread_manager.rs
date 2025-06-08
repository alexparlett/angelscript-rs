//! Thread management for AngelScript integration
//!
//! This module provides a `ThreadManager` type that switches implementation
//! based on compile-time features:
//! - Default (when `rust-threads` is not enabled): Uses AngelScript's C++ manager
//! - `rust-threads`: Uses a pure Rust implementation

use crate::core::error::{ScriptError, ScriptResult};
use angelscript_sys::{
    asAcquireExclusiveLock, asAcquireSharedLock, asGetThreadManager, asIThreadManager,
    asPrepareMultithread, asReleaseExclusiveLock, asReleaseSharedLock, asThreadCleanup,
    asUnprepareMultithread,
};
use std::ptr;

// ========== C++ IMPLEMENTATION (DEFAULT) ==========

#[cfg(not(feature = "rust-threads"))]
mod cpp_impl {
    use super::*;
    use crate::types::enums::ReturnCode;

    /// Lightweight wrapper around AngelScript's C++ thread manager
    #[derive(Debug)]
    pub struct ThreadManager {
        inner: *mut asIThreadManager,
    }

    impl ThreadManager {
        /// Prepares AngelScript for multithreading and returns the manager
        pub fn prepare() -> ScriptResult<Self> {
            unsafe {
                let result = asPrepareMultithread(ptr::null_mut());
                ScriptError::from_code(result)?;

                let mgr_ptr = asGetThreadManager();
                if mgr_ptr.is_null() {
                    Err(ScriptError::AngelScriptError(ReturnCode::Error))
                } else {
                    Ok(Self { inner: mgr_ptr })
                }
            }
        }

        /// Creates a wrapper from a raw pointer
        pub(crate) fn from_raw(ptr: *mut asIThreadManager) -> Self {
            Self { inner: ptr }
        }

        /// Checks if the pointer is null
        pub fn is_null(&self) -> bool {
            self.inner.is_null()
        }

        /// Gets information about this thread manager
        pub fn info(&self) -> String {
            "AngelScript C++ thread manager".to_string()
        }

        /// Gets the implementation type
        pub fn implementation_type(&self) -> &'static str {
            "cpp"
        }

        /// Unprepares the thread manager
        pub fn unprepare() {
            unsafe {
                asUnprepareMultithread();
            }
        }

        /// Cleans up thread-local data
        pub fn cleanup_local_data() -> ScriptResult<()> {
            unsafe { ScriptError::from_code(asThreadCleanup()) }
        }

        /// Acquires exclusive lock
        pub fn acquire_exclusive_lock() {
            unsafe {
                asAcquireExclusiveLock();
            }
        }

        /// Releases exclusive lock
        pub fn release_exclusive_lock() {
            unsafe {
                asReleaseExclusiveLock();
            }
        }

        /// Acquires shared lock
        pub fn acquire_shared_lock() {
            unsafe {
                asAcquireSharedLock();
            }
        }

        /// Releases shared lock
        pub fn release_shared_lock() {
            unsafe {
                asReleaseSharedLock();
            }
        }
    }

    unsafe impl Send for ThreadManager {}
    unsafe impl Sync for ThreadManager {}
}

// ========== RUST IMPLEMENTATION ==========

#[cfg(feature = "rust-threads")]
mod rust_impl {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Mutex, RwLock};
    use std::thread::{self, ThreadId};
    use angelscript_sys::{asIScriptContext, asIThreadManager__bindgen_vtable};

    /// Thread-local data structure that mirrors asCThreadLocalData
    #[derive(Debug, Clone)]
    pub struct ThreadLocalData {
        pub active_contexts: Vec<*mut asIScriptContext>,
        pub string_buffer: String,
    }

    impl ThreadLocalData {
        pub fn new() -> Self {
            Self {
                active_contexts: Vec::new(),
                string_buffer: String::new(),
            }
        }

        pub fn add_active_context(&mut self, context: *mut asIScriptContext) {
            if !context.is_null() {
                self.active_contexts.push(context);
            }
        }

        pub fn remove_active_context(&mut self, context: *mut asIScriptContext) {
            self.active_contexts.retain(|&ctx| ctx != context);
        }

        pub fn has_active_contexts(&self) -> bool {
            !self.active_contexts.is_empty()
        }

        pub fn active_context_count(&self) -> usize {
            self.active_contexts.len()
        }

        pub fn clear_string_buffer(&mut self) {
            self.string_buffer.clear();
        }
    }

    impl Default for ThreadLocalData {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Critical section implementation using std::sync::Mutex
    #[derive(Debug)]
    pub struct ThreadCriticalSection {
        mutex: Mutex<()>,
    }

    impl ThreadCriticalSection {
        pub fn new() -> Self {
            Self {
                mutex: Mutex::new(()),
            }
        }

        pub fn enter(&self) -> ThreadCriticalSectionGuard {
            let guard = self.mutex.lock().expect("Critical section poisoned");
            ThreadCriticalSectionGuard { _guard: guard }
        }

        pub fn try_enter(&self) -> Option<ThreadCriticalSectionGuard> {
            self.mutex
                .try_lock()
                .ok()
                .map(|guard| ThreadCriticalSectionGuard { _guard: guard })
        }
    }

    pub struct ThreadCriticalSectionGuard<'a> {
        _guard: std::sync::MutexGuard<'a, ()>,
    }

    /// Read-write lock implementation using std::sync::RwLock
    #[derive(Debug)]
    pub struct ThreadReadWriteLock {
        lock: RwLock<()>,
    }

    impl ThreadReadWriteLock {
        pub fn new() -> Self {
            Self {
                lock: RwLock::new(()),
            }
        }

        pub fn acquire_exclusive(&self) -> ThreadWriteLockGuard {
            let guard = self.lock.write().expect("RwLock poisoned");
            ThreadWriteLockGuard { _guard: guard }
        }

        pub fn acquire_shared(&self) -> ThreadReadLockGuard {
            let guard = self.lock.read().expect("RwLock poisoned");
            ThreadReadLockGuard { _guard: guard }
        }

        pub fn try_acquire_exclusive(&self) -> Option<ThreadWriteLockGuard> {
            self.lock
                .try_write()
                .ok()
                .map(|guard| ThreadWriteLockGuard { _guard: guard })
        }

        pub fn try_acquire_shared(&self) -> Option<ThreadReadLockGuard> {
            self.lock
                .try_read()
                .ok()
                .map(|guard| ThreadReadLockGuard { _guard: guard })
        }
    }

    pub struct ThreadWriteLockGuard<'a> {
        _guard: std::sync::RwLockWriteGuard<'a, ()>,
    }

    pub struct ThreadReadLockGuard<'a> {
        _guard: std::sync::RwLockReadGuard<'a, ()>,
    }

    /// Thread-local storage manager
    #[derive(Debug)]
    struct ThreadLocalStorage {
        storage: Mutex<HashMap<ThreadId, ThreadLocalData>>,
    }

    impl ThreadLocalStorage {
        fn new() -> Self {
            Self {
                storage: Mutex::new(HashMap::new()),
            }
        }

        fn get_local_data(&self) -> ScriptResult<ThreadLocalData> {
            let thread_id = thread::current().id();
            let mut storage = self.storage.lock().map_err(|_| {
                ScriptError::External(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Thread storage lock poisoned",
                )))
            })?;

            Ok(storage
                .entry(thread_id)
                .or_insert_with(ThreadLocalData::new)
                .clone())
        }

        fn update_local_data<F>(&self, f: F) -> ScriptResult<()>
        where
            F: FnOnce(&mut ThreadLocalData),
        {
            let thread_id = thread::current().id();
            let mut storage = self.storage.lock().map_err(|_| {
                ScriptError::External(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Thread storage lock poisoned",
                )))
            })?;

            let data = storage
                .entry(thread_id)
                .or_insert_with(ThreadLocalData::new);
            f(data);
            Ok(())
        }

        fn cleanup_local_data(&self) -> ScriptResult<()> {
            let thread_id = thread::current().id();
            let mut storage = self.storage.lock().map_err(|_| {
                ScriptError::External(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Thread storage lock poisoned",
                )))
            })?;

            if let Some(data) = storage.get(&thread_id) {
                if data.has_active_contexts() {
                    return Err(ScriptError::AngelScriptError(ReturnCode::ContextActive));
                }
            }

            storage.remove(&thread_id);
            Ok(())
        }
    }

    /// Core Rust thread manager implementation
    #[derive(Debug)]
    struct RustThreadManagerCore {
        ref_count: Mutex<i32>,
        thread_local_storage: ThreadLocalStorage,
        app_rw_lock: ThreadReadWriteLock,
        critical_section: ThreadCriticalSection,
    }

    impl RustThreadManagerCore {
        fn new() -> Self {
            Self {
                ref_count: Mutex::new(1),
                thread_local_storage: ThreadLocalStorage::new(),
                app_rw_lock: ThreadReadWriteLock::new(),
                critical_section: ThreadCriticalSection::new(),
            }
        }
    }

    /// FFI-compatible thread manager that implements asIThreadManager interface
    #[repr(C)]
    struct RustThreadManagerFFI {
        vtable: *const asIThreadManager__bindgen_vtable,
        ref_count: i32,
        core: Box<RustThreadManagerCore>,
    }

    impl RustThreadManagerFFI {
        fn new() -> Box<Self> {
            Box::new(Self {
                vtable: &RUST_THREAD_MANAGER_VTABLE,
                ref_count: 1,
                core: Box::new(RustThreadManagerCore::new()),
            })
        }

        fn as_interface_ptr(boxed: Box<Self>) -> *mut asIThreadManager {
            Box::into_raw(boxed) as *mut asIThreadManager
        }

        unsafe fn from_interface_ptr(ptr: *mut asIThreadManager) -> Box<Self> {
            Box::from_raw(ptr as *mut Self)
        }
    }

    static RUST_THREAD_MANAGER_VTABLE: asIThreadManager__bindgen_vtable =
        asIThreadManager__bindgen_vtable {};

    /// Pure Rust thread manager implementation
    #[derive(Debug)]
    pub struct ThreadManager {
        /// The FFI interface pointer for AngelScript
        interface_ptr: *mut asIThreadManager,
        /// Reference to the core implementation (for safe access)
        core: *const RustThreadManagerCore,
    }

    impl ThreadManager {
        /// Prepares AngelScript for multithreading with Rust implementation
        pub fn prepare() -> ScriptResult<Self> {
            let ffi_manager = RustThreadManagerFFI::new();
            let core_ptr = ffi_manager.core.as_ref() as *const RustThreadManagerCore;
            let interface_ptr = RustThreadManagerFFI::as_interface_ptr(ffi_manager);

            unsafe {
                let result = asPrepareMultithread(interface_ptr);
                if result != 0 {
                    // Clean up on failure
                    let _recovered = RustThreadManagerFFI::from_interface_ptr(interface_ptr);
                    return Err(ScriptError::from_code(result));
                }
            }

            Ok(Self {
                interface_ptr,
                core: core_ptr,
            })
        }

        /// Creates a wrapper from a raw pointer (for compatibility)
        pub(crate) fn from_raw(ptr: *mut asIThreadManager) -> Self {
            // For Rust implementation, we assume this is our own pointer
            unsafe {
                let ffi_manager = &*(ptr as *const RustThreadManagerFFI);
                let core_ptr = ffi_manager.core.as_ref() as *const RustThreadManagerCore;

                Self {
                    interface_ptr: ptr,
                    core: core_ptr,
                }
            }
        }

        /// Returns the raw pointer for AngelScript
        pub(crate) fn as_ptr(&self) -> *mut asIThreadManager {
            self.interface_ptr
        }

        /// Checks if the pointer is null
        pub fn is_null(&self) -> bool {
            self.interface_ptr.is_null()
        }

        /// Gets the core implementation safely
        fn core(&self) -> &RustThreadManagerCore {
            unsafe { &*self.core }
        }

        /// Gets thread-local data for the current thread
        pub fn get_local_data(&self) -> ScriptResult<ThreadLocalData> {
            self.core().thread_local_storage.get_local_data()
        }

        /// Updates thread-local data for the current thread
        pub fn update_local_data<F>(&self, f: F) -> ScriptResult<()>
        where
            F: FnOnce(&mut ThreadLocalData),
        {
            self.core().thread_local_storage.update_local_data(f)
        }

        /// Gets the application read-write lock
        pub fn app_rw_lock(&self) -> &ThreadReadWriteLock {
            &self.core().app_rw_lock
        }

        /// Gets the critical section
        pub fn critical_section(&self) -> &ThreadCriticalSection {
            &self.core().critical_section
        }

        /// Gets information about this thread manager
        pub fn info(&self) -> String {
            format!(
                "Rust thread manager (active threads: {})",
                self.core()
                    .thread_local_storage
                    .storage
                    .lock()
                    .map(|s| s.len())
                    .unwrap_or(0)
            )
        }

        /// Gets the implementation type
        pub fn implementation_type(&self) -> &'static str {
            "rust"
        }

        /// Unprepares the thread manager
        pub fn unprepare() {
            unsafe {
                asUnprepareMultithread();
            }
        }

        /// Cleans up thread-local data
        pub fn cleanup_local_data() -> ScriptResult<()> {
            // For Rust implementation, we need to clean up both AngelScript and our data
            unsafe {
                ScriptError::from_code(asThreadCleanup())?;
            }

            // Additional Rust-specific cleanup could go here if needed
            Ok(())
        }

        /// Acquires exclusive lock (delegates to AngelScript's global lock)
        pub fn acquire_exclusive_lock() {
            unsafe {
                asAcquireExclusiveLock();
            }
        }

        /// Releases exclusive lock
        pub fn release_exclusive_lock() {
            unsafe {
                asReleaseExclusiveLock();
            }
        }

        /// Acquires shared lock
        pub fn acquire_shared_lock() {
            unsafe {
                asAcquireSharedLock();
            }
        }

        /// Releases shared lock
        pub fn release_shared_lock() {
            unsafe {
                asReleaseSharedLock();
            }
        }
    }

    impl Drop for ThreadManager {
        fn drop(&mut self) {
            // Clean up the FFI manager when the Rust wrapper is dropped
            if !self.interface_ptr.is_null() {
                unsafe {
                    let _ffi_manager = RustThreadManagerFFI::from_interface_ptr(self.interface_ptr);
                    // Box will be dropped automatically
                }
            }
        }
    }

    unsafe impl Send for ThreadManager {}
    unsafe impl Sync for ThreadManager {}

    // Re-export types that are only available with rust-threads
    use crate::prelude::ReturnCode;
    pub use ThreadCriticalSection;
    pub use ThreadLocalData;
    pub use ThreadReadLockGuard;
    pub use ThreadReadWriteLock;
    pub use ThreadWriteLockGuard;
    use crate::types::enums::ReturnCode;
}

// ========== CONDITIONAL EXPORTS ==========

#[cfg(not(feature = "rust-threads"))]
pub use cpp_impl::ThreadManager;

#[cfg(feature = "rust-threads")]
pub use rust_impl::*;

/// RAII guard for AngelScript's exclusive lock
pub struct ExclusiveLockGuard;

impl Default for ExclusiveLockGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl ExclusiveLockGuard {
    pub fn new() -> Self {
        ThreadManager::acquire_exclusive_lock();
        Self
    }
}

impl Drop for ExclusiveLockGuard {
    fn drop(&mut self) {
        ThreadManager::release_exclusive_lock();
    }
}

/// RAII guard for AngelScript's shared lock
pub struct SharedLockGuard;

impl Default for SharedLockGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedLockGuard {
    pub fn new() -> Self {
        ThreadManager::acquire_shared_lock();
        Self
    }
}

impl Drop for SharedLockGuard {
    fn drop(&mut self) {
        ThreadManager::release_shared_lock();
    }
}
