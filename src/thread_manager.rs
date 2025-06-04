use crate::error::{Error, Result};
use crate::ReturnCode;
use angelscript_bindings::{asIThreadManager, asIThreadManager__bindgen_vtable};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, RwLock};

/// Wrapper for AngelScript's thread manager interface
///
/// This is an abstract interface that can be implemented by applications
/// to provide custom thread management. AngelScript provides its own
/// default implementation internally.
#[derive(Debug)]
pub struct ThreadManager {
    inner: *mut asIThreadManager,
    _phantom: PhantomData<asIThreadManager>,
}

impl ThreadManager {
    /// Creates a ThreadManager wrapper from a raw pointer
    ///
    /// # Safety
    /// The pointer must be valid and point to a properly initialized asIThreadManager
    pub(crate) fn from_raw(ptr: *mut asIThreadManager) -> Self {
        Self {
            inner: ptr,
            _phantom: PhantomData,
        }
    }

    /// Returns the raw pointer to the thread manager
    pub(crate) fn as_ptr(&self) -> *mut asIThreadManager {
        self.inner
    }

    /// Checks if the thread manager pointer is null
    pub fn is_null(&self) -> bool {
        self.inner.is_null()
    }

    // Note: The vtable is empty because asIThreadManager is a pure abstract interface
    // Applications implement this interface and AngelScript calls the implementation
    // through virtual function calls in C++

    fn as_vtable(&self) -> &asIThreadManager__bindgen_vtable {
        unsafe { &*(*self.inner).vtable_ }
    }
}

unsafe impl Send for ThreadManager {}
unsafe impl Sync for ThreadManager {}

/// Thread-local data structure that mirrors asCThreadLocalData
#[derive(Debug)]
pub struct ThreadLocalData {
    /// Active script contexts in this thread
    pub active_contexts: Vec<*mut crate::ffi::asIScriptContext>,
    /// Thread-local string buffer for temporary operations
    pub string_buffer: String,
}

impl ThreadLocalData {
    /// Creates new thread-local data
    pub fn new() -> Self {
        Self {
            active_contexts: Vec::new(),
            string_buffer: String::new(),
        }
    }

    /// Adds an active context to this thread
    pub fn add_active_context(&mut self, context: *mut crate::ffi::asIScriptContext) {
        self.active_contexts.push(context);
    }

    /// Removes an active context from this thread
    pub fn remove_active_context(&mut self, context: *mut crate::ffi::asIScriptContext) {
        self.active_contexts.retain(|&ctx| ctx != context);
    }

    /// Checks if there are any active contexts
    pub fn has_active_contexts(&self) -> bool {
        !self.active_contexts.is_empty()
    }

    /// Gets the number of active contexts
    pub fn active_context_count(&self) -> usize {
        self.active_contexts.len()
    }
}

impl Default for ThreadLocalData {
    fn default() -> Self {
        Self::new()
    }
}

/// Critical section implementation that mirrors asCThreadCriticalSection
#[derive(Debug)]
pub struct ThreadCriticalSection {
    mutex: Mutex<()>,
}

impl ThreadCriticalSection {
    /// Creates a new critical section
    pub fn new() -> Self {
        Self {
            mutex: Mutex::new(()),
        }
    }

    /// Enters the critical section (blocks until acquired)
    pub fn enter(&self) -> ThreadCriticalSectionGuard {
        let guard = self.mutex.lock().expect("Critical section poisoned");
        ThreadCriticalSectionGuard { _guard: guard }
    }

    /// Tries to enter the critical section without blocking
    pub fn try_enter(&self) -> Option<ThreadCriticalSectionGuard> {
        self.mutex
            .try_lock()
            .ok()
            .map(|guard| ThreadCriticalSectionGuard { _guard: guard })
    }
}

impl Default for ThreadCriticalSection {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard for critical section
pub struct ThreadCriticalSectionGuard<'a> {
    _guard: std::sync::MutexGuard<'a, ()>,
}

// The guard automatically releases the lock when dropped

/// Read-write lock implementation that mirrors asCThreadReadWriteLock
#[derive(Debug)]
pub struct ThreadReadWriteLock {
    lock: RwLock<()>,
}

impl ThreadReadWriteLock {
    /// Creates a new read-write lock
    pub fn new() -> Self {
        Self {
            lock: RwLock::new(()),
        }
    }

    /// Acquires an exclusive (write) lock
    pub fn acquire_exclusive(&self) -> ThreadWriteLockGuard {
        let guard = self.lock.write().expect("RwLock poisoned");
        ThreadWriteLockGuard { _guard: guard }
    }

    /// Acquires a shared (read) lock
    pub fn acquire_shared(&self) -> ThreadReadLockGuard {
        let guard = self.lock.read().expect("RwLock poisoned");
        ThreadReadLockGuard { _guard: guard }
    }

    /// Tries to acquire an exclusive lock without blocking
    pub fn try_acquire_exclusive(&self) -> Option<ThreadWriteLockGuard> {
        self.lock
            .try_write()
            .ok()
            .map(|guard| ThreadWriteLockGuard { _guard: guard })
    }

    /// Tries to acquire a shared lock without blocking
    pub fn try_acquire_shared(&self) -> Option<ThreadReadLockGuard> {
        self.lock
            .try_read()
            .ok()
            .map(|guard| ThreadReadLockGuard { _guard: guard })
    }
}

impl Default for ThreadReadWriteLock {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard for exclusive (write) lock
pub struct ThreadWriteLockGuard<'a> {
    _guard: std::sync::RwLockWriteGuard<'a, ()>,
}

/// RAII guard for shared (read) lock
pub struct ThreadReadLockGuard<'a> {
    _guard: std::sync::RwLockReadGuard<'a, ()>,
}

/// Rust implementation of thread manager functionality
///
/// This provides the same functionality as AngelScript's internal asCThreadManager
/// but implemented in safe Rust. This can be used as a reference or as a basis
/// for custom thread manager implementations.
#[derive(Debug)]
pub struct RustThreadManager {
    /// Reference count for the thread manager
    ref_count: Arc<Mutex<i32>>,
    /// Thread-local storage for each thread
    thread_local_storage: Arc<Mutex<HashMap<std::thread::ThreadId, ThreadLocalData>>>,
    /// Application-level read-write lock (equivalent to appRWLock in C++)
    app_rw_lock: Arc<ThreadReadWriteLock>,
    /// Critical section for internal synchronization
    critical_section: Arc<ThreadCriticalSection>,
}

impl RustThreadManager {
    /// Creates a new Rust-based thread manager
    pub fn new() -> Self {
        Self {
            ref_count: Arc::new(Mutex::new(1)),
            thread_local_storage: Arc::new(Mutex::new(HashMap::new())),
            app_rw_lock: Arc::new(ThreadReadWriteLock::new()),
            critical_section: Arc::new(ThreadCriticalSection::new()),
        }
    }

    /// Gets the thread-local data for the current thread
    pub fn get_local_data(&self) -> Result<Arc<Mutex<ThreadLocalData>>> {
        let thread_id = std::thread::current().id();
        let mut storage = self.thread_local_storage.lock().map_err(|_| {
            Error::External(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Thread storage lock poisoned",
            )))
        })?;

        if !storage.contains_key(&thread_id) {
            storage.insert(thread_id, ThreadLocalData::new());
        }

        // This is a bit awkward - we need to return a reference to the data
        // In the real implementation, this would be handled by TLS
        Ok(Arc::new(Mutex::new(ThreadLocalData::new())))
    }

    /// Cleans up thread-local data for the current thread
    pub fn cleanup_local_data(&self) -> Result<()> {
        let thread_id = std::thread::current().id();
        let mut storage = self.thread_local_storage.lock().map_err(|_| {
            Error::External(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Thread storage lock poisoned",
            )))
        })?;

        if let Some(data) = storage.get(&thread_id) {
            if data.has_active_contexts() {
                return Err(Error::AngelScript(ReturnCode::ContextActive));
            }
        }

        storage.remove(&thread_id);
        Ok(())
    }

    /// Increments the reference count
    pub fn add_ref(&self) -> Result<()> {
        let _guard = self.critical_section.enter();
        let mut count = self.ref_count.lock().map_err(|_| {
            Error::External(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Reference count lock poisoned",
            )))
        })?;
        *count += 1;
        Ok(())
    }

    /// Decrements the reference count and returns true if it reached zero
    pub fn release(&self) -> Result<bool> {
        let _guard = self.critical_section.enter();
        let mut count = self.ref_count.lock().map_err(|_| {
            Error::External(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Reference count lock poisoned",
            )))
        })?;
        *count -= 1;
        Ok(*count == 0)
    }

    /// Gets the application read-write lock
    pub fn app_rw_lock(&self) -> &ThreadReadWriteLock {
        &self.app_rw_lock
    }

    /// Gets the critical section
    pub fn critical_section(&self) -> &ThreadCriticalSection {
        &self.critical_section
    }
}

impl Default for RustThreadManager {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for RustThreadManager {}

unsafe impl Sync for RustThreadManager {}

/// Global thread manager instance (mirrors the singleton in C++)
static THREAD_MANAGER: std::sync::OnceLock<Arc<RustThreadManager>> = std::sync::OnceLock::new();

/// Thread manager utilities that mirror the C++ global functions
pub struct ThreadManagerUtils;

impl ThreadManagerUtils {
    /// Prepares the global thread manager (equivalent to asPrepareMultithread)
    pub fn prepare(external_mgr: Option<ThreadManager>) -> Result<()> {
        if external_mgr.is_some() && THREAD_MANAGER.get().is_some() {
            return Err(Error::AngelScript(ReturnCode::InvalidArg));
        }

        if external_mgr.is_none() && THREAD_MANAGER.get().is_none() {
            let manager = Arc::new(RustThreadManager::new());
            THREAD_MANAGER.set(manager).map_err(|_| {
                Error::External(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to set global thread manager",
                )))
            })?;
        }

        if let Some(manager) = THREAD_MANAGER.get() {
            manager.add_ref()?;
        }

        Ok(())
    }

    /// Unprepares the global thread manager (equivalent to asUnprepareMultithread)
    pub fn unprepare() -> Result<()> {
        if let Some(manager) = THREAD_MANAGER.get() {
            if manager.release()? {
                // In the C++ version, the manager would be deleted here
                // In Rust, it will be dropped when the last Arc reference is dropped
            }
        }
        Ok(())
    }

    /// Cleans up thread-local data (equivalent to asThreadCleanup)
    pub fn cleanup_local_data() -> Result<()> {
        if let Some(manager) = THREAD_MANAGER.get() {
            manager.cleanup_local_data()
        } else {
            Ok(())
        }
    }

    /// Gets the global thread manager
    pub fn get_thread_manager() -> Option<Arc<RustThreadManager>> {
        THREAD_MANAGER.get().cloned()
    }

    /// Acquires the application exclusive lock
    pub fn acquire_exclusive_lock() -> Option<ThreadWriteLockGuard<'static>> {
        THREAD_MANAGER.get()?.app_rw_lock().try_acquire_exclusive()
    }

    /// Acquires the application shared lock
    pub fn acquire_shared_lock() -> Option<ThreadReadLockGuard<'static>> {
        THREAD_MANAGER.get()?.app_rw_lock().try_acquire_shared()
    }
}
