use angelscript_sys::asIJITCompilerAbstract;

pub struct JITCompiler {
    ptr: *mut asIJITCompilerAbstract,
}

impl JITCompiler {
    pub(crate) fn from_raw(ptr: *mut asIJITCompilerAbstract) -> Self {
        Self { ptr }
    }

    pub(crate) fn as_ptr(&self) -> *mut asIJITCompilerAbstract {
        self.ptr
    }
}

unsafe impl Send for JITCompiler {}
unsafe impl Sync for JITCompiler {}
