use std::any::{Any, TypeId as StdTypeId};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, LazyLock, RwLock};
use std::rc::Rc;

pub type TypeId = u32;

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

pub fn allocate_type_id() -> TypeId {
    NEXT_TYPE_ID.fetch_add(1, Ordering::SeqCst)
}

#[derive(Debug, Clone)]
pub struct ObjectType {
    pub type_id: u32,
    pub name: String,
    pub flags: TypeFlags,

    pub properties: Vec<ObjectProperty>,
    pub methods: Vec<ObjectMethod>,
    pub behaviours: Vec<BehaviourInfo>,
    pub rust_type_id: Option<StdTypeId>,
}

#[derive(Debug, Clone)]
pub struct ObjectProperty {
    pub name: String,
    pub type_id: u32,
    pub is_handle: bool,
    pub is_const: bool,
    pub access: AccessSpecifier,
}

#[derive(Debug, Clone)]
pub struct ObjectMethod {
    pub name: String,
    pub return_type_id: u32,
    pub params: Vec<MethodParam>,
    pub is_const: bool,
    pub is_virtual: bool,
    pub is_final: bool,
    pub access: AccessSpecifier,
    pub function_id: u32,
}

#[derive(Debug, Clone)]
pub struct GlobalFunction {
    pub name: String,
    pub return_type_id: u32,
    pub params: Vec<MethodParam>,
    pub function_id: u32,
}

#[derive(Debug, Clone)]
pub struct BehaviourInfo {
    pub behaviour_type: BehaviourType,
    pub function_id: u32,
    pub return_type_id: u32,
    pub params: Vec<MethodParam>,
}

#[derive(Debug, Clone)]
pub struct MethodParam {
    pub name: String,
    pub type_id: u32,
    pub is_ref: bool,
    pub is_out: bool,
    pub is_const: bool,
}

#[derive(Debug, Clone)]
pub struct EnumType {
    pub type_id: u32,
    pub name: String,
    pub values: HashMap<String, i32>,
}

#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub type_id: u32,
    pub name: String,
    pub aliased_type_id: u32,
}

#[derive(Debug, Clone)]
pub struct FuncdefInfo {
    pub type_id: u32,
    pub name: String,
    pub return_type_id: u32,
    pub params: Vec<MethodParam>,
}

#[derive(Debug, Clone)]
pub struct GlobalProperty {
    pub name: String,
    pub type_id: u32,
    pub is_const: bool,
    pub is_handle: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessSpecifier {
    Public,
    Protected,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        const APP_CLASS_CONSTRUCTOR = 0x00000200;
        const APP_CLASS_DESTRUCTOR = 0x00000400;
        const APP_CLASS_ASSIGNMENT = 0x00000800;
        const APP_CLASS_COPY_CONSTRUCTOR = 0x00001000;
        const NOCOUNT = 0x00002000;
        const APP_CLASS_ALLINTS = 0x00004000;
        const APP_CLASS_ALLFLOATS = 0x00008000;
        const APP_CLASS_ALIGN8 = 0x00010000;
        const IMPLICIT_HANDLE = 0x00020000;
        const APP_CLASS_UNION = 0x00040000;
        const SCRIPT_OBJECT = 0x00080000;
        const SHARED = 0x00100000;
        const NOINHERIT = 0x00200000;
        const FUNCDEF = 0x00400000;
        const ENUM = 0x00800000;
        const TYPEDEF = 0x01000000;
        const ABSTRACT = 0x02000000;
        const APP_ALIGN16 = 0x04000000;

        const APP_CLASS_C = Self::APP_CLASS.bits() | Self::APP_CLASS_CONSTRUCTOR.bits();
        const APP_CLASS_CD = Self::APP_CLASS_C.bits() | Self::APP_CLASS_DESTRUCTOR.bits();
        const APP_CLASS_CA = Self::APP_CLASS_C.bits() | Self::APP_CLASS_ASSIGNMENT.bits();
        const APP_CLASS_CK = Self::APP_CLASS_C.bits() | Self::APP_CLASS_COPY_CONSTRUCTOR.bits();
        const APP_CLASS_CDA = Self::APP_CLASS_CD.bits() | Self::APP_CLASS_ASSIGNMENT.bits();
        const APP_CLASS_CDK = Self::APP_CLASS_CD.bits() | Self::APP_CLASS_COPY_CONSTRUCTOR.bits();
        const APP_CLASS_CAK = Self::APP_CLASS_CA.bits() | Self::APP_CLASS_COPY_CONSTRUCTOR.bits();
        const APP_CLASS_CDAK = Self::APP_CLASS_CDA.bits() | Self::APP_CLASS_COPY_CONSTRUCTOR.bits();
        const APP_CLASS_D = Self::APP_CLASS.bits() | Self::APP_CLASS_DESTRUCTOR.bits();
        const APP_CLASS_DA = Self::APP_CLASS_D.bits() | Self::APP_CLASS_ASSIGNMENT.bits();
        const APP_CLASS_DK = Self::APP_CLASS_D.bits() | Self::APP_CLASS_COPY_CONSTRUCTOR.bits();
        const APP_CLASS_DAK = Self::APP_CLASS_DA.bits() | Self::APP_CLASS_COPY_CONSTRUCTOR.bits();
        const APP_CLASS_A = Self::APP_CLASS.bits() | Self::APP_CLASS_ASSIGNMENT.bits();
        const APP_CLASS_AK = Self::APP_CLASS_A.bits() | Self::APP_CLASS_COPY_CONSTRUCTOR.bits();
        const APP_CLASS_K = Self::APP_CLASS.bits() | Self::APP_CLASS_COPY_CONSTRUCTOR.bits();
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Primitive,
    Class,
    Enum,
    Interface,
    Funcdef,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeRegistration {
    Script,
    Application,
}

/// Runtime value that can be stored in variables, on stack, etc.
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

    /// Initialization list (temporary, used during array/object construction)
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
            (Self::Dynamic(a), Self::Dynamic(b)) => { false }, // Can't compare Dynamic values
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
            ScriptValue::Dynamic(_) => true
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