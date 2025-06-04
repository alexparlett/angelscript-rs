use angelscript_bindings::asPWORD;

pub trait UserData {
    const TypeId: asPWORD;
}
