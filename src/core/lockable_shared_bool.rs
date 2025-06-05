use crate::core::error::{ScriptError, ScriptResult};
use angelscript_sys::{asILockableSharedBool, asILockableSharedBool__bindgen_vtable};

#[derive(Debug)]
pub struct LockableSharedBool {
    inner: *mut asILockableSharedBool,
}

impl LockableSharedBool {
    pub(crate) fn from_raw(ptr: *mut asILockableSharedBool) -> Self {
        let wrapper = Self { inner: ptr };
        wrapper
            .add_ref()
            .expect("Failed to add reference to LockableSharedBool");
        wrapper
    }

    pub(crate) fn as_ptr(&self) -> *mut asILockableSharedBool {
        self.inner
    }

    // ========== VTABLE ORDER (matches asILockableSharedBool__bindgen_vtable) ==========

    // 1. AddRef
    pub fn add_ref(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asILockableSharedBool_AddRef)(self.inner)) }
    }

    // 2. Release
    pub fn release(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asILockableSharedBool_Release)(self.inner)) }
    }

    // 3. Get
    pub fn get(&self) -> bool {
        unsafe { (self.as_vtable().asILockableSharedBool_Get)(self.inner) }
    }

    // 4. Set
    pub fn set(&self, value: bool) {
        unsafe { (self.as_vtable().asILockableSharedBool_Set)(self.inner, value) }
    }

    // 5. Lock
    pub fn lock(&self) {
        unsafe { (self.as_vtable().asILockableSharedBool_Lock)(self.inner) }
    }

    // 6. Unlock
    pub fn unlock(&self) {
        unsafe { (self.as_vtable().asILockableSharedBool_Unlock)(self.inner) }
    }

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

// Convenience RAII lock guard
pub struct LockableSharedBoolGuard<'a> {
    bool_ref: &'a LockableSharedBool,
}

impl<'a> LockableSharedBoolGuard<'a> {
    pub fn new(bool_ref: &'a LockableSharedBool) -> Self {
        bool_ref.lock();
        Self { bool_ref }
    }

    pub fn get(&self) -> bool {
        self.bool_ref.get()
    }

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
    /// Creates a RAII guard that locks the boolean and automatically unlocks on drop
    pub fn lock_guard(&self) -> LockableSharedBoolGuard {
        LockableSharedBoolGuard::new(self)
    }
}
