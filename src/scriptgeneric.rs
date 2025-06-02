use angelscript_bindings::asIScriptGeneric;

#[repr(C)]
pub struct ScriptGeneric {
    generic: *mut asIScriptGeneric
}

impl ScriptGeneric {
    pub(crate) fn from_raw(generic: *mut asIScriptGeneric) -> Self {
        Self { generic }
    }

    pub(crate) fn as_ptr(&self) -> *mut asIScriptGeneric {
        self.generic
    }
}