use angelscript_sys::asPWORD;

pub trait UserData {
    const TypeId: asPWORD;
}
