use std::any::Any;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, LazyLock, RwLock};

pub type TypeId = u32;
pub type FunctionId = u32;
pub type ModuleId = u32;

pub const TYPE_VOID: TypeId = 0;
pub const TYPE_BOOL: TypeId = 1;
pub const TYPE_INT8: TypeId = 2;
pub const TYPE_INT16: TypeId = 3;
pub const TYPE_INT32: TypeId = 4;
pub const TYPE_INT64: TypeId = 5;
pub const TYPE_UINT8: TypeId = 6;
pub const TYPE_UINT16: TypeId = 7;
pub const TYPE_UINT32: TypeId = 8;
pub const TYPE_UINT64: TypeId = 9;
pub const TYPE_FLOAT: TypeId = 10;
pub const TYPE_DOUBLE: TypeId = 11;
pub const TYPE_STRING: TypeId = 12;
pub const TYPE_AUTO: TypeId = 13;

static NEXT_TYPE_ID: LazyLock<AtomicU32> = LazyLock::new(|| AtomicU32::new(100));
static NEXT_FUNCTION_ID: LazyLock<AtomicU32> = LazyLock::new(|| AtomicU32::new(1000));

pub fn allocate_type_id() -> TypeId {
    NEXT_TYPE_ID.fetch_add(1, Ordering::SeqCst)
}

pub fn allocate_function_id() -> FunctionId {
    NEXT_FUNCTION_ID.fetch_add(1, Ordering::SeqCst)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    Primitive,
    Class,
    Enum,
    Interface,
    Funcdef,
    Typedef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeRegistration {
    Script,
    Application,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TypeFlags: u32 {
        const REF_TYPE = 0x00000001;
        const VALUE_TYPE = 0x00000002;
        const GC_TYPE = 0x00000004;
        const POD_TYPE = 0x00000008;
        const NOHANDLE = 0x00000010;
        const SCOPED = 0x00000020;
        const TEMPLATE = 0x00000040;
        const ASHANDLE = 0x00000080;
        const APP_CLASS = 0x00000100;
        const NOCOUNT = 0x00002000;
        const IMPLICIT_HANDLE = 0x00020000;
        const SCRIPT_OBJECT = 0x00080000;
        const SHARED = 0x00100000;
        const NOINHERIT = 0x00200000;
        const FUNCDEF = 0x00400000;
        const ENUM = 0x00800000;
        const TYPEDEF = 0x01000000;
        const ABSTRACT = 0x02000000;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessSpecifier {
    Public,
    Protected,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BehaviourType {
    Construct,
    ListConstruct,
    Destruct,
    ListFactory,
    AddRef,
    Release,
    GetWeakRefFlag,
    TemplateCallback,
    GetRefCount,
    SetGCFlag,
    GetGCFlag,
    EnumRefs,
    ReleaseRefs,
}

#[derive(Debug)]
pub enum ScriptValue {
    Void,
    Bool(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float(f32),
    Double(f64),
    String(String),
    ObjectHandle(u64),
    InitList(Vec<ScriptValue>),
    Dynamic(Arc<RwLock<Box<dyn Any + Send + Sync>>>),
    Null,
}

impl Clone for ScriptValue {
    fn clone(&self) -> Self {
        match self {
            Self::Void => Self::Void,
            Self::Bool(b) => Self::Bool(*b),
            Self::Int8(n) => Self::Int8(*n),
            Self::Int16(n) => Self::Int16(*n),
            Self::Int32(n) => Self::Int32(*n),
            Self::Int64(n) => Self::Int64(*n),
            Self::UInt8(n) => Self::UInt8(*n),
            Self::UInt16(n) => Self::UInt16(*n),
            Self::UInt32(n) => Self::UInt32(*n),
            Self::UInt64(n) => Self::UInt64(*n),
            Self::Float(f) => Self::Float(*f),
            Self::Double(d) => Self::Double(*d),
            Self::String(s) => Self::String(s.clone()),
            Self::ObjectHandle(h) => Self::ObjectHandle(*h),
            Self::InitList(list) => Self::InitList(list.clone()),
            Self::Dynamic(dynamic) => Self::Dynamic(dynamic.clone()),
            Self::Null => Self::Null,
        }
    }
}

impl PartialEq for ScriptValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Void, Self::Void) => true,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Int8(a), Self::Int8(b)) => a == b,
            (Self::Int16(a), Self::Int16(b)) => a == b,
            (Self::Int32(a), Self::Int32(b)) => a == b,
            (Self::Int64(a), Self::Int64(b)) => a == b,
            (Self::UInt8(a), Self::UInt8(b)) => a == b,
            (Self::UInt16(a), Self::UInt16(b)) => a == b,
            (Self::UInt32(a), Self::UInt32(b)) => a == b,
            (Self::UInt64(a), Self::UInt64(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::Double(a), Self::Double(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::ObjectHandle(a), Self::ObjectHandle(b)) => a == b,
            (Self::InitList(a), Self::InitList(b)) => a == b,
            (Self::Dynamic(_), Self::Dynamic(_)) => false,
            (Self::Null, Self::Null) => true,
            _ => false,
        }
    }
}

impl ScriptValue {
    pub fn is_truthy(&self) -> bool {
        match self {
            ScriptValue::Void | ScriptValue::Null => false,
            ScriptValue::Bool(b) => *b,
            ScriptValue::Int8(n) => *n != 0,
            ScriptValue::Int16(n) => *n != 0,
            ScriptValue::Int32(n) => *n != 0,
            ScriptValue::Int64(n) => *n != 0,
            ScriptValue::UInt8(n) => *n != 0,
            ScriptValue::UInt16(n) => *n != 0,
            ScriptValue::UInt32(n) => *n != 0,
            ScriptValue::UInt64(n) => *n != 0,
            ScriptValue::Float(f) => *f != 0.0,
            ScriptValue::Double(d) => *d != 0.0,
            ScriptValue::String(s) => !s.is_empty(),
            ScriptValue::ObjectHandle(h) => *h != 0,
            ScriptValue::InitList(list) => !list.is_empty(),
            ScriptValue::Dynamic(_) => true,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self {
            ScriptValue::Int32(n) => Some(*n),
            ScriptValue::Int8(n) => Some(*n as i32),
            ScriptValue::Int16(n) => Some(*n as i32),
            ScriptValue::UInt8(n) => Some(*n as i32),
            ScriptValue::UInt16(n) => Some(*n as i32),
            _ => None,
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match self {
            ScriptValue::Float(f) => Some(*f),
            ScriptValue::Int32(n) => Some(*n as f32),
            _ => None,
        }
    }

    pub fn as_object_handle(&self) -> Option<u64> {
        match self {
            ScriptValue::ObjectHandle(h) => Some(*h),
            _ => None,
        }
    }

    pub fn as_init_list(&self) -> Option<&Vec<ScriptValue>> {
        match self {
            ScriptValue::InitList(list) => Some(list),
            _ => None,
        }
    }
}
