// Return Codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ReturnCode {
    Success = asERetCodes_asSUCCESS,
    Error = asERetCodes_asERROR,
    ContextActive = asERetCodes_asCONTEXT_ACTIVE,
    ContextNotFinished = asERetCodes_asCONTEXT_NOT_FINISHED,
    ContextNotPrepared = asERetCodes_asCONTEXT_NOT_PREPARED,
    InvalidArg = asERetCodes_asINVALID_ARG,
    NoFunction = asERetCodes_asNO_FUNCTION,
    NotSupported = asERetCodes_asNOT_SUPPORTED,
    InvalidName = asERetCodes_asINVALID_NAME,
    NameTaken = asERetCodes_asNAME_TAKEN,
    InvalidDeclaration = asERetCodes_asINVALID_DECLARATION,
    InvalidObject = asERetCodes_asINVALID_OBJECT,
    InvalidType = asERetCodes_asINVALID_TYPE,
    AlreadyRegistered = asERetCodes_asALREADY_REGISTERED,
    MultipleFunctions = asERetCodes_asMULTIPLE_FUNCTIONS,
    NoModule = asERetCodes_asNO_MODULE,
    NoGlobalVar = asERetCodes_asNO_GLOBAL_VAR,
    InvalidConfiguration = asERetCodes_asINVALID_CONFIGURATION,
    InvalidInterface = asERetCodes_asINVALID_INTERFACE,
    CantBindAllFunctions = asERetCodes_asCANT_BIND_ALL_FUNCTIONS,
    LowerArrayDimensionNotRegistered = asERetCodes_asLOWER_ARRAY_DIMENSION_NOT_REGISTERED,
    WrongConfigGroup = asERetCodes_asWRONG_CONFIG_GROUP,
    ConfigGroupIsInUse = asERetCodes_asCONFIG_GROUP_IS_IN_USE,
    IllegalBehaviourForType = asERetCodes_asILLEGAL_BEHAVIOUR_FOR_TYPE,
    WrongCallingConv = asERetCodes_asWRONG_CALLING_CONV,
    BuildInProgress = asERetCodes_asBUILD_IN_PROGRESS,
    InitGlobalVarsFailed = asERetCodes_asINIT_GLOBAL_VARS_FAILED,
    OutOfMemory = asERetCodes_asOUT_OF_MEMORY,
    ModuleIsInUse = asERetCodes_asMODULE_IS_IN_USE,
}

impl From<asERetCodes> for ReturnCode {
    fn from(value: asERetCodes) -> Self {
        match value {
            asERetCodes_asSUCCESS => Self::Success,
            asERetCodes_asERROR => Self::Error,
            asERetCodes_asCONTEXT_ACTIVE => Self::ContextActive,
            asERetCodes_asCONTEXT_NOT_FINISHED => Self::ContextNotFinished,
            asERetCodes_asCONTEXT_NOT_PREPARED => Self::ContextNotPrepared,
            asERetCodes_asINVALID_ARG => Self::InvalidArg,
            asERetCodes_asNO_FUNCTION => Self::NoFunction,
            asERetCodes_asNOT_SUPPORTED => Self::NotSupported,
            asERetCodes_asINVALID_NAME => Self::InvalidName,
            asERetCodes_asNAME_TAKEN => Self::NameTaken,
            asERetCodes_asINVALID_DECLARATION => Self::InvalidDeclaration,
            asERetCodes_asINVALID_OBJECT => Self::InvalidObject,
            asERetCodes_asINVALID_TYPE => Self::InvalidType,
            asERetCodes_asALREADY_REGISTERED => Self::AlreadyRegistered,
            asERetCodes_asMULTIPLE_FUNCTIONS => Self::MultipleFunctions,
            asERetCodes_asNO_MODULE => Self::NoModule,
            asERetCodes_asNO_GLOBAL_VAR => Self::NoGlobalVar,
            asERetCodes_asINVALID_CONFIGURATION => Self::InvalidConfiguration,
            asERetCodes_asINVALID_INTERFACE => Self::InvalidInterface,
            asERetCodes_asCANT_BIND_ALL_FUNCTIONS => Self::CantBindAllFunctions,
            asERetCodes_asLOWER_ARRAY_DIMENSION_NOT_REGISTERED => {
                Self::LowerArrayDimensionNotRegistered
            }
            asERetCodes_asWRONG_CONFIG_GROUP => Self::WrongConfigGroup,
            asERetCodes_asCONFIG_GROUP_IS_IN_USE => Self::ConfigGroupIsInUse,
            asERetCodes_asILLEGAL_BEHAVIOUR_FOR_TYPE => Self::IllegalBehaviourForType,
            asERetCodes_asWRONG_CALLING_CONV => Self::WrongCallingConv,
            asERetCodes_asBUILD_IN_PROGRESS => Self::BuildInProgress,
            asERetCodes_asINIT_GLOBAL_VARS_FAILED => Self::InitGlobalVarsFailed,
            asERetCodes_asOUT_OF_MEMORY => Self::OutOfMemory,
            asERetCodes_asMODULE_IS_IN_USE => Self::ModuleIsInUse,
            _ => panic!("Unknown return code: {}", value),
        }
    }
}

impl From<ReturnCode> for asERetCodes {
    fn from(value: ReturnCode) -> Self {
        value as asERetCodes
    }
}

// Engine Properties
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EngineProperty {
    AllowUnsafeReferences = asEEngineProp_asEP_ALLOW_UNSAFE_REFERENCES,
    OptimizeBytecode = asEEngineProp_asEP_OPTIMIZE_BYTECODE,
    CopyScriptSections = asEEngineProp_asEP_COPY_SCRIPT_SECTIONS,
    MaxStackSize = asEEngineProp_asEP_MAX_STACK_SIZE,
    UseCharacterLiterals = asEEngineProp_asEP_USE_CHARACTER_LITERALS,
    AllowMultilineStrings = asEEngineProp_asEP_ALLOW_MULTILINE_STRINGS,
    AllowImplicitHandleTypes = asEEngineProp_asEP_ALLOW_IMPLICIT_HANDLE_TYPES,
    BuildWithoutLineCues = asEEngineProp_asEP_BUILD_WITHOUT_LINE_CUES,
    InitGlobalVarsAfterBuild = asEEngineProp_asEP_INIT_GLOBAL_VARS_AFTER_BUILD,
    RequireEnumScope = asEEngineProp_asEP_REQUIRE_ENUM_SCOPE,
    ScriptScanner = asEEngineProp_asEP_SCRIPT_SCANNER,
    IncludeJitInstructions = asEEngineProp_asEP_INCLUDE_JIT_INSTRUCTIONS,
    StringEncoding = asEEngineProp_asEP_STRING_ENCODING,
    PropertyAccessorMode = asEEngineProp_asEP_PROPERTY_ACCESSOR_MODE,
    ExpandDefArrayToTmpl = asEEngineProp_asEP_EXPAND_DEF_ARRAY_TO_TMPL,
    AutoGarbageCollect = asEEngineProp_asEP_AUTO_GARBAGE_COLLECT,
    DisallowGlobalVars = asEEngineProp_asEP_DISALLOW_GLOBAL_VARS,
    AlwaysImplDefaultConstruct = asEEngineProp_asEP_ALWAYS_IMPL_DEFAULT_CONSTRUCT,
    CompilerWarnings = asEEngineProp_asEP_COMPILER_WARNINGS,
    DisallowValueAssignForRefType = asEEngineProp_asEP_DISALLOW_VALUE_ASSIGN_FOR_REF_TYPE,
    AlterSyntaxNamedArgs = asEEngineProp_asEP_ALTER_SYNTAX_NAMED_ARGS,
    DisableIntegerDivision = asEEngineProp_asEP_DISABLE_INTEGER_DIVISION,
    DisallowEmptyListElements = asEEngineProp_asEP_DISALLOW_EMPTY_LIST_ELEMENTS,
    PrivatePropAsProtected = asEEngineProp_asEP_PRIVATE_PROP_AS_PROTECTED,
    AllowUnicodeIdentifiers = asEEngineProp_asEP_ALLOW_UNICODE_IDENTIFIERS,
    HeredocTrimMode = asEEngineProp_asEP_HEREDOC_TRIM_MODE,
    MaxNestedCalls = asEEngineProp_asEP_MAX_NESTED_CALLS,
    GenericCallMode = asEEngineProp_asEP_GENERIC_CALL_MODE,
    InitStackSize = asEEngineProp_asEP_INIT_STACK_SIZE,
    InitCallStackSize = asEEngineProp_asEP_INIT_CALL_STACK_SIZE,
    MaxCallStackSize = asEEngineProp_asEP_MAX_CALL_STACK_SIZE,
    IgnoreDuplicateSharedIntf = asEEngineProp_asEP_IGNORE_DUPLICATE_SHARED_INTF,
    NoDebugOutput = asEEngineProp_asEP_NO_DEBUG_OUTPUT,
    DisableScriptClassGc = asEEngineProp_asEP_DISABLE_SCRIPT_CLASS_GC,
    JitInterfaceVersion = asEEngineProp_asEP_JIT_INTERFACE_VERSION,
    AlwaysImplDefaultCopy = asEEngineProp_asEP_ALWAYS_IMPL_DEFAULT_COPY,
    AlwaysImplDefaultCopyConstruct = asEEngineProp_asEP_ALWAYS_IMPL_DEFAULT_COPY_CONSTRUCT,
    LastProperty = asEEngineProp_asEP_LAST_PROPERTY,
}

impl From<asEEngineProp> for EngineProperty {
    fn from(value: asEEngineProp) -> Self {
        match value {
            asEEngineProp_asEP_ALLOW_UNSAFE_REFERENCES => Self::AllowUnsafeReferences,
            asEEngineProp_asEP_OPTIMIZE_BYTECODE => Self::OptimizeBytecode,
            asEEngineProp_asEP_COPY_SCRIPT_SECTIONS => Self::CopyScriptSections,
            asEEngineProp_asEP_MAX_STACK_SIZE => Self::MaxStackSize,
            asEEngineProp_asEP_USE_CHARACTER_LITERALS => Self::UseCharacterLiterals,
            asEEngineProp_asEP_ALLOW_MULTILINE_STRINGS => Self::AllowMultilineStrings,
            asEEngineProp_asEP_ALLOW_IMPLICIT_HANDLE_TYPES => Self::AllowImplicitHandleTypes,
            asEEngineProp_asEP_BUILD_WITHOUT_LINE_CUES => Self::BuildWithoutLineCues,
            asEEngineProp_asEP_INIT_GLOBAL_VARS_AFTER_BUILD => Self::InitGlobalVarsAfterBuild,
            asEEngineProp_asEP_REQUIRE_ENUM_SCOPE => Self::RequireEnumScope,
            asEEngineProp_asEP_SCRIPT_SCANNER => Self::ScriptScanner,
            asEEngineProp_asEP_INCLUDE_JIT_INSTRUCTIONS => Self::IncludeJitInstructions,
            asEEngineProp_asEP_STRING_ENCODING => Self::StringEncoding,
            asEEngineProp_asEP_PROPERTY_ACCESSOR_MODE => Self::PropertyAccessorMode,
            asEEngineProp_asEP_EXPAND_DEF_ARRAY_TO_TMPL => Self::ExpandDefArrayToTmpl,
            asEEngineProp_asEP_AUTO_GARBAGE_COLLECT => Self::AutoGarbageCollect,
            asEEngineProp_asEP_DISALLOW_GLOBAL_VARS => Self::DisallowGlobalVars,
            asEEngineProp_asEP_ALWAYS_IMPL_DEFAULT_CONSTRUCT => Self::AlwaysImplDefaultConstruct,
            asEEngineProp_asEP_COMPILER_WARNINGS => Self::CompilerWarnings,
            asEEngineProp_asEP_DISALLOW_VALUE_ASSIGN_FOR_REF_TYPE => {
                Self::DisallowValueAssignForRefType
            }
            asEEngineProp_asEP_ALTER_SYNTAX_NAMED_ARGS => Self::AlterSyntaxNamedArgs,
            asEEngineProp_asEP_DISABLE_INTEGER_DIVISION => Self::DisableIntegerDivision,
            asEEngineProp_asEP_DISALLOW_EMPTY_LIST_ELEMENTS => Self::DisallowEmptyListElements,
            asEEngineProp_asEP_PRIVATE_PROP_AS_PROTECTED => Self::PrivatePropAsProtected,
            asEEngineProp_asEP_ALLOW_UNICODE_IDENTIFIERS => Self::AllowUnicodeIdentifiers,
            asEEngineProp_asEP_HEREDOC_TRIM_MODE => Self::HeredocTrimMode,
            asEEngineProp_asEP_MAX_NESTED_CALLS => Self::MaxNestedCalls,
            asEEngineProp_asEP_GENERIC_CALL_MODE => Self::GenericCallMode,
            asEEngineProp_asEP_INIT_STACK_SIZE => Self::InitStackSize,
            asEEngineProp_asEP_INIT_CALL_STACK_SIZE => Self::InitCallStackSize,
            asEEngineProp_asEP_MAX_CALL_STACK_SIZE => Self::MaxCallStackSize,
            asEEngineProp_asEP_IGNORE_DUPLICATE_SHARED_INTF => Self::IgnoreDuplicateSharedIntf,
            asEEngineProp_asEP_NO_DEBUG_OUTPUT => Self::NoDebugOutput,
            asEEngineProp_asEP_DISABLE_SCRIPT_CLASS_GC => Self::DisableScriptClassGc,
            asEEngineProp_asEP_JIT_INTERFACE_VERSION => Self::JitInterfaceVersion,
            asEEngineProp_asEP_ALWAYS_IMPL_DEFAULT_COPY => Self::AlwaysImplDefaultCopy,
            asEEngineProp_asEP_ALWAYS_IMPL_DEFAULT_COPY_CONSTRUCT => {
                Self::AlwaysImplDefaultCopyConstruct
            }
            asEEngineProp_asEP_LAST_PROPERTY => Self::LastProperty,
            _ => panic!("Unknown engine property: {}", value),
        }
    }
}

impl From<EngineProperty> for asEEngineProp {
    fn from(value: EngineProperty) -> Self {
        value as asEEngineProp
    }
}

// Calling Convention Types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CallingConvention {
    Cdecl = asECallConvTypes_asCALL_CDECL,
    Stdcall = asECallConvTypes_asCALL_STDCALL,
    ThiscallAsGlobal = asECallConvTypes_asCALL_THISCALL_ASGLOBAL,
    Thiscall = asECallConvTypes_asCALL_THISCALL,
    CdeclObjLast = asECallConvTypes_asCALL_CDECL_OBJLAST,
    CdeclObjFirst = asECallConvTypes_asCALL_CDECL_OBJFIRST,
    Generic = asECallConvTypes_asCALL_GENERIC,
    ThiscallObjLast = asECallConvTypes_asCALL_THISCALL_OBJLAST,
    ThiscallObjFirst = asECallConvTypes_asCALL_THISCALL_OBJFIRST,
}

impl From<asECallConvTypes> for CallingConvention {
    fn from(value: asECallConvTypes) -> Self {
        match value {
            asECallConvTypes_asCALL_CDECL => Self::Cdecl,
            asECallConvTypes_asCALL_STDCALL => Self::Stdcall,
            asECallConvTypes_asCALL_THISCALL_ASGLOBAL => Self::ThiscallAsGlobal,
            asECallConvTypes_asCALL_THISCALL => Self::Thiscall,
            asECallConvTypes_asCALL_CDECL_OBJLAST => Self::CdeclObjLast,
            asECallConvTypes_asCALL_CDECL_OBJFIRST => Self::CdeclObjFirst,
            asECallConvTypes_asCALL_GENERIC => Self::Generic,
            asECallConvTypes_asCALL_THISCALL_OBJLAST => Self::ThiscallObjLast,
            asECallConvTypes_asCALL_THISCALL_OBJFIRST => Self::ThiscallObjFirst,
            _ => panic!("Unknown calling convention: {}", value),
        }
    }
}

impl From<CallingConvention> for asECallConvTypes {
    fn from(value: CallingConvention) -> Self {
        value as asECallConvTypes
    }
}

// Object Type Flags (using bitflags for this one since it's a flag enum)
use angelscript_sys::*;
use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ObjectTypeFlags: asEObjTypeFlags {
        const REF = asEObjTypeFlags_asOBJ_REF;
        const VALUE = asEObjTypeFlags_asOBJ_VALUE;
        const GC = asEObjTypeFlags_asOBJ_GC;
        const POD = asEObjTypeFlags_asOBJ_POD;
        const NOHANDLE = asEObjTypeFlags_asOBJ_NOHANDLE;
        const SCOPED = asEObjTypeFlags_asOBJ_SCOPED;
        const TEMPLATE = asEObjTypeFlags_asOBJ_TEMPLATE;
        const ASHANDLE = asEObjTypeFlags_asOBJ_ASHANDLE;
        const APP_CLASS = asEObjTypeFlags_asOBJ_APP_CLASS;
        const APP_CLASS_CONSTRUCTOR = asEObjTypeFlags_asOBJ_APP_CLASS_CONSTRUCTOR;
        const APP_CLASS_DESTRUCTOR = asEObjTypeFlags_asOBJ_APP_CLASS_DESTRUCTOR;
        const APP_CLdASS_ASSIGNMENT = asEObjTypeFlags_asOBJ_APP_CLASS_ASSIGNMENT;
        const APP_CLASS_COPY_CONSTRUCTOR = asEObjTypeFlags_asOBJ_APP_CLASS_COPY_CONSTRUCTOR;
        const APP_CLASS_C = asEObjTypeFlags_asOBJ_APP_CLASS_C;
        const APP_CLASS_CD = asEObjTypeFlags_asOBJ_APP_CLASS_CD;
        const APP_CLASS_CA = asEObjTypeFlags_asOBJ_APP_CLASS_CA;
        const APP_CLASS_CK = asEObjTypeFlags_asOBJ_APP_CLASS_CK;
        const APP_CLASS_CDA = asEObjTypeFlags_asOBJ_APP_CLASS_CDA;
        const APP_CLASS_CDK = asEObjTypeFlags_asOBJ_APP_CLASS_CDK;
        const APP_CLASS_CAK = asEObjTypeFlags_asOBJ_APP_CLASS_CAK;
        const APP_CLASS_CDAK = asEObjTypeFlags_asOBJ_APP_CLASS_CDAK;
        const APP_CLASS_D = asEObjTypeFlags_asOBJ_APP_CLASS_D;
        const APP_CLASS_DA = asEObjTypeFlags_asOBJ_APP_CLASS_DA;
        const APP_CLASS_DK = asEObjTypeFlags_asOBJ_APP_CLASS_DK;
        const APP_CLASS_DAK = asEObjTypeFlags_asOBJ_APP_CLASS_DAK;
        const APP_CLASS_A = asEObjTypeFlags_asOBJ_APP_CLASS_A;
        const APP_CLASS_AK = asEObjTypeFlags_asOBJ_APP_CLASS_AK;
        const APP_CLASS_K = asEObjTypeFlags_asOBJ_APP_CLASS_K;
        const APP_CLASS_MORE_CONSTRUCTORS = asEObjTypeFlags_asOBJ_APP_CLASS_MORE_CONSTRUCTORS;
        const APP_PRIMITIVE = asEObjTypeFlags_asOBJ_APP_PRIMITIVE;
        const APP_FLOAT = asEObjTypeFlags_asOBJ_APP_FLOAT;
        const APP_ARRAY = asEObjTypeFlags_asOBJ_APP_ARRAY;
        const APP_CLASS_ALLINTS = asEObjTypeFlags_asOBJ_APP_CLASS_ALLINTS;
        const APP_CLASS_ALLFLOATS = asEObjTypeFlags_asOBJ_APP_CLASS_ALLFLOATS;
        const NOCOUNT = asEObjTypeFlags_asOBJ_NOCOUNT;
        const APP_CLASS_ALIGN8 = asEObjTypeFlags_asOBJ_APP_CLASS_ALIGN8;
        const IMPLICIT_HANDLE = asEObjTypeFlags_asOBJ_IMPLICIT_HANDLE;
        const APP_CLASS_UNION = asEObjTypeFlags_asOBJ_APP_CLASS_UNION;
        const MASK_VALID_FLAGS = asEObjTypeFlags_asOBJ_MASK_VALID_FLAGS;
        const SCRIPT_OBJECT = asEObjTypeFlags_asOBJ_SCRIPT_OBJECT;
        const SHARED = asEObjTypeFlags_asOBJ_SHARED;
        const NOINHERIT = asEObjTypeFlags_asOBJ_NOINHERIT;
        const FUNCDEF = asEObjTypeFlags_asOBJ_FUNCDEF;
        const LIST_PATTERN = asEObjTypeFlags_asOBJ_LIST_PATTERN;
        const ENUM = asEObjTypeFlags_asOBJ_ENUM;
        const TEMPLATE_SUBTYPE = asEObjTypeFlags_asOBJ_TEMPLATE_SUBTYPE;
        const TYPEDEF = asEObjTypeFlags_asOBJ_TYPEDEF;
        const ABSTRACT = asEObjTypeFlags_asOBJ_ABSTRACT;
        const APP_ALIGN16 = asEObjTypeFlags_asOBJ_APP_ALIGN16;
    }
}

impl From<asEObjTypeFlags> for ObjectTypeFlags {
    fn from(value: asEObjTypeFlags) -> Self {
        ObjectTypeFlags::from_bits_truncate(value)
    }
}

impl From<ObjectTypeFlags> for asEObjTypeFlags {
    fn from(value: ObjectTypeFlags) -> Self {
        value.bits()
    }
}

// Behaviours
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Behaviour {
    Construct = asEBehaviours_asBEHAVE_CONSTRUCT,
    ListConstruct = asEBehaviours_asBEHAVE_LIST_CONSTRUCT,
    Destruct = asEBehaviours_asBEHAVE_DESTRUCT,
    Factory = asEBehaviours_asBEHAVE_FACTORY,
    ListFactory = asEBehaviours_asBEHAVE_LIST_FACTORY,
    AddRef = asEBehaviours_asBEHAVE_ADDREF,
    Release = asEBehaviours_asBEHAVE_RELEASE,
    GetWeakRefFlag = asEBehaviours_asBEHAVE_GET_WEAKREF_FLAG,
    TemplateCallback = asEBehaviours_asBEHAVE_TEMPLATE_CALLBACK,
    GetRefCount = asEBehaviours_asBEHAVE_GETREFCOUNT,
    SetGcFlag = asEBehaviours_asBEHAVE_SETGCFLAG,
    GetGcFlag = asEBehaviours_asBEHAVE_GETGCFLAG,
    EnumRefs = asEBehaviours_asBEHAVE_ENUMREFS,
    ReleaseRefs = asEBehaviours_asBEHAVE_RELEASEREFS,
    Max = asEBehaviours_asBEHAVE_MAX,
}

impl From<asEBehaviours> for Behaviour {
    fn from(value: asEBehaviours) -> Self {
        match value {
            asEBehaviours_asBEHAVE_CONSTRUCT => Self::Construct,
            asEBehaviours_asBEHAVE_LIST_CONSTRUCT => Self::ListConstruct,
            asEBehaviours_asBEHAVE_DESTRUCT => Self::Destruct,
            asEBehaviours_asBEHAVE_FACTORY => Self::Factory,
            asEBehaviours_asBEHAVE_LIST_FACTORY => Self::ListFactory,
            asEBehaviours_asBEHAVE_ADDREF => Self::AddRef,
            asEBehaviours_asBEHAVE_RELEASE => Self::Release,
            asEBehaviours_asBEHAVE_GET_WEAKREF_FLAG => Self::GetWeakRefFlag,
            asEBehaviours_asBEHAVE_TEMPLATE_CALLBACK => Self::TemplateCallback,
            asEBehaviours_asBEHAVE_GETREFCOUNT => Self::GetRefCount,
            asEBehaviours_asBEHAVE_SETGCFLAG => Self::SetGcFlag,
            asEBehaviours_asBEHAVE_GETGCFLAG => Self::GetGcFlag,
            asEBehaviours_asBEHAVE_ENUMREFS => Self::EnumRefs,
            asEBehaviours_asBEHAVE_RELEASEREFS => Self::ReleaseRefs,
            asEBehaviours_asBEHAVE_MAX => Self::Max,
            _ => panic!("Unknown behaviour: {}", value),
        }
    }
}

impl From<Behaviour> for asEBehaviours {
    fn from(value: Behaviour) -> Self {
        value as asEBehaviours
    }
}

// Context State
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ContextState {
    Finished = asEContextState_asEXECUTION_FINISHED,
    Suspended = asEContextState_asEXECUTION_SUSPENDED,
    Aborted = asEContextState_asEXECUTION_ABORTED,
    Exception = asEContextState_asEXECUTION_EXCEPTION,
    Prepared = asEContextState_asEXECUTION_PREPARED,
    Uninitialized = asEContextState_asEXECUTION_UNINITIALIZED,
    Active = asEContextState_asEXECUTION_ACTIVE,
    Error = asEContextState_asEXECUTION_ERROR,
    Deserialization = asEContextState_asEXECUTION_DESERIALIZATION,
}

impl From<asEContextState> for ContextState {
    fn from(value: asEContextState) -> Self {
        match value {
            asEContextState_asEXECUTION_FINISHED => Self::Finished,
            asEContextState_asEXECUTION_SUSPENDED => Self::Suspended,
            asEContextState_asEXECUTION_ABORTED => Self::Aborted,
            asEContextState_asEXECUTION_EXCEPTION => Self::Exception,
            asEContextState_asEXECUTION_PREPARED => Self::Prepared,
            asEContextState_asEXECUTION_UNINITIALIZED => Self::Uninitialized,
            asEContextState_asEXECUTION_ACTIVE => Self::Active,
            asEContextState_asEXECUTION_ERROR => Self::Error,
            asEContextState_asEXECUTION_DESERIALIZATION => Self::Deserialization,
            _ => panic!("Unknown context state: {}", value),
        }
    }
}

impl From<ContextState> for asEContextState {
    fn from(value: ContextState) -> Self {
        value as asEContextState
    }
}

// Message Type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MessageType {
    Error = asEMsgType_asMSGTYPE_ERROR,
    Warning = asEMsgType_asMSGTYPE_WARNING,
    Information = asEMsgType_asMSGTYPE_INFORMATION,
}

impl From<asEMsgType> for MessageType {
    fn from(value: asEMsgType) -> Self {
        match value {
            asEMsgType_asMSGTYPE_ERROR => Self::Error,
            asEMsgType_asMSGTYPE_WARNING => Self::Warning,
            asEMsgType_asMSGTYPE_INFORMATION => Self::Information,
            _ => panic!("Unknown message type: {}", value),
        }
    }
}

impl From<MessageType> for asEMsgType {
    fn from(value: MessageType) -> Self {
        value as asEMsgType
    }
}

// GC Flags
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct GCFlags: u32 {
        const FULL_CYCLE = asEGCFlags_asGC_FULL_CYCLE;
        const ONE_STEP = asEGCFlags_asGC_ONE_STEP;
        const DESTROY_GARBAGE = asEGCFlags_asGC_DESTROY_GARBAGE;
        const DETECT_GARBAGE = asEGCFlags_asGC_DETECT_GARBAGE;
    }
}

impl From<asEGCFlags> for GCFlags {
    fn from(value: asEGCFlags) -> Self {
        GCFlags::from_bits_truncate(value)
    }
}

impl From<GCFlags> for asEGCFlags {
    fn from(value: GCFlags) -> Self {
        value.bits()
    }
}

// Token Class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TokenClass {
    Unknown = asETokenClass_asTC_UNKNOWN,
    Keyword = asETokenClass_asTC_KEYWORD,
    Value = asETokenClass_asTC_VALUE,
    Identifier = asETokenClass_asTC_IDENTIFIER,
    Comment = asETokenClass_asTC_COMMENT,
    Whitespace = asETokenClass_asTC_WHITESPACE,
}

impl From<asETokenClass> for TokenClass {
    fn from(value: asETokenClass) -> Self {
        match value {
            asETokenClass_asTC_UNKNOWN => Self::Unknown,
            asETokenClass_asTC_KEYWORD => Self::Keyword,
            asETokenClass_asTC_VALUE => Self::Value,
            asETokenClass_asTC_IDENTIFIER => Self::Identifier,
            asETokenClass_asTC_COMMENT => Self::Comment,
            asETokenClass_asTC_WHITESPACE => Self::Whitespace,
            _ => panic!("Unknown token class: {}", value),
        }
    }
}

impl From<TokenClass> for asETokenClass {
    fn from(value: TokenClass) -> Self {
        value as asETokenClass
    }
}

// Type ID Flags
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TypeIdFlags: u32 {
        const VOID = asETypeIdFlags_asTYPEID_VOID;
        const BOOL = asETypeIdFlags_asTYPEID_BOOL;
        const INT8 = asETypeIdFlags_asTYPEID_INT8;
        const INT16 = asETypeIdFlags_asTYPEID_INT16;
        const INT32 = asETypeIdFlags_asTYPEID_INT32;
        const INT64 = asETypeIdFlags_asTYPEID_INT64;
        const UINT8 = asETypeIdFlags_asTYPEID_UINT8;
        const UINT16 = asETypeIdFlags_asTYPEID_UINT16;
        const UINT32 = asETypeIdFlags_asTYPEID_UINT32;
        const UINT64 = asETypeIdFlags_asTYPEID_UINT64;
        const FLOAT = asETypeIdFlags_asTYPEID_FLOAT;
        const DOUBLE = asETypeIdFlags_asTYPEID_DOUBLE;
        const OBJHANDLE = asETypeIdFlags_asTYPEID_OBJHANDLE;
        const HANDLETOCONST = asETypeIdFlags_asTYPEID_HANDLETOCONST;
        const MASK_OBJECT = asETypeIdFlags_asTYPEID_MASK_OBJECT;
        const APPOBJECT = asETypeIdFlags_asTYPEID_APPOBJECT;
        const SCRIPTOBJECT = asETypeIdFlags_asTYPEID_SCRIPTOBJECT;
        const TEMPLATE = asETypeIdFlags_asTYPEID_TEMPLATE;
        const MASK_SEQNBR = asETypeIdFlags_asTYPEID_MASK_SEQNBR;
    }
}

impl From<asETypeIdFlags> for TypeIdFlags {
    fn from(value: asETypeIdFlags) -> Self {
        TypeIdFlags::from_bits_truncate(value)
    }
}

impl From<TypeIdFlags> for asETypeIdFlags {
    fn from(value: TypeIdFlags) -> Self {
        value.bits()
    }
}

// Type Modifiers
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TypeModifiers: u32 {
        const NONE = asETypeModifiers_asTM_NONE;
        const INREF = asETypeModifiers_asTM_INREF;
        const OUTREF = asETypeModifiers_asTM_OUTREF;
        const INOUTREF = asETypeModifiers_asTM_INOUTREF;
        const CONST = asETypeModifiers_asTM_CONST;
    }
}

impl From<asETypeModifiers> for TypeModifiers {
    fn from(value: asETypeModifiers) -> Self {
        TypeModifiers::from_bits_truncate(value)
    }
}

impl From<TypeModifiers> for asETypeModifiers {
    fn from(value: TypeModifiers) -> Self {
        value.bits()
    }
}

// Get Module Flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GetModuleFlags {
    OnlyIfExists = asEGMFlags_asGM_ONLY_IF_EXISTS,
    CreateIfNotExists = asEGMFlags_asGM_CREATE_IF_NOT_EXISTS,
    AlwaysCreate = asEGMFlags_asGM_ALWAYS_CREATE,
}

impl From<asEGMFlags> for GetModuleFlags {
    fn from(value: asEGMFlags) -> Self {
        match value {
            asEGMFlags_asGM_ONLY_IF_EXISTS => Self::OnlyIfExists,
            asEGMFlags_asGM_CREATE_IF_NOT_EXISTS => Self::CreateIfNotExists,
            asEGMFlags_asGM_ALWAYS_CREATE => Self::AlwaysCreate,
            _ => panic!("Unknown get module flag: {}", value),
        }
    }
}

impl From<GetModuleFlags> for asEGMFlags {
    fn from(value: GetModuleFlags) -> Self {
        value as asEGMFlags
    }
}

// Compile Flags
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CompileFlags: u32 {
        const ADD_TO_MODULE = asECompileFlags_asCOMP_ADD_TO_MODULE;
    }
}

impl From<asECompileFlags> for CompileFlags {
    fn from(value: asECompileFlags) -> Self {
        CompileFlags::from_bits_truncate(value)
    }
}

impl From<CompileFlags> for asECompileFlags {
    fn from(value: CompileFlags) -> Self {
        value.bits()
    }
}

// Function Type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum FunctionType {
    Dummy = asEFuncType_asFUNC_DUMMY,
    System = asEFuncType_asFUNC_SYSTEM,
    Script = asEFuncType_asFUNC_SCRIPT,
    Interface = asEFuncType_asFUNC_INTERFACE,
    Virtual = asEFuncType_asFUNC_VIRTUAL,
    Funcdef = asEFuncType_asFUNC_FUNCDEF,
    Imported = asEFuncType_asFUNC_IMPORTED,
    Delegate = asEFuncType_asFUNC_DELEGATE,
}

impl From<asEFuncType> for FunctionType {
    fn from(value: asEFuncType) -> Self {
        match value {
            asEFuncType_asFUNC_DUMMY => Self::Dummy,
            asEFuncType_asFUNC_SYSTEM => Self::System,
            asEFuncType_asFUNC_SCRIPT => Self::Script,
            asEFuncType_asFUNC_INTERFACE => Self::Interface,
            asEFuncType_asFUNC_VIRTUAL => Self::Virtual,
            asEFuncType_asFUNC_FUNCDEF => Self::Funcdef,
            asEFuncType_asFUNC_IMPORTED => Self::Imported,
            asEFuncType_asFUNC_DELEGATE => Self::Delegate,
            _ => panic!("Unknown function type: {}", value),
        }
    }
}

impl From<FunctionType> for asEFuncType {
    fn from(value: FunctionType) -> Self {
        value as asEFuncType
    }
}

// Bytecode Instructions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BCInstr {
    PopPtr = asEBCInstr_asBC_PopPtr,
    PshGPtr = asEBCInstr_asBC_PshGPtr,
    PshC4 = asEBCInstr_asBC_PshC4,
    PshV4 = asEBCInstr_asBC_PshV4,
    PSF = asEBCInstr_asBC_PSF,
    SwapPtr = asEBCInstr_asBC_SwapPtr,
    NOT = asEBCInstr_asBC_NOT,
    PshG4 = asEBCInstr_asBC_PshG4,
    LdGRdR4 = asEBCInstr_asBC_LdGRdR4,
    CALL = asEBCInstr_asBC_CALL,
    RET = asEBCInstr_asBC_RET,
    JMP = asEBCInstr_asBC_JMP,
    JZ = asEBCInstr_asBC_JZ,
    JNZ = asEBCInstr_asBC_JNZ,
    JS = asEBCInstr_asBC_JS,
    JNS = asEBCInstr_asBC_JNS,
    JP = asEBCInstr_asBC_JP,
    JNP = asEBCInstr_asBC_JNP,
    TZ = asEBCInstr_asBC_TZ,
    TNZ = asEBCInstr_asBC_TNZ,
    TS = asEBCInstr_asBC_TS,
    TNS = asEBCInstr_asBC_TNS,
    TP = asEBCInstr_asBC_TP,
    TNP = asEBCInstr_asBC_TNP,
    NEGi = asEBCInstr_asBC_NEGi,
    NEGf = asEBCInstr_asBC_NEGf,
    NEGd = asEBCInstr_asBC_NEGd,
    INCi16 = asEBCInstr_asBC_INCi16,
    INCi8 = asEBCInstr_asBC_INCi8,
    DECi16 = asEBCInstr_asBC_DECi16,
    DECi8 = asEBCInstr_asBC_DECi8,
    INCi = asEBCInstr_asBC_INCi,
    DECi = asEBCInstr_asBC_DECi,
    INCf = asEBCInstr_asBC_INCf,
    DECf = asEBCInstr_asBC_DECf,
    INCd = asEBCInstr_asBC_INCd,
    DECd = asEBCInstr_asBC_DECd,
    IncVi = asEBCInstr_asBC_IncVi,
    DecVi = asEBCInstr_asBC_DecVi,
    BNOT = asEBCInstr_asBC_BNOT,
    BAND = asEBCInstr_asBC_BAND,
    BOR = asEBCInstr_asBC_BOR,
    BXOR = asEBCInstr_asBC_BXOR,
    BSLL = asEBCInstr_asBC_BSLL,
    BSRL = asEBCInstr_asBC_BSRL,
    BSRA = asEBCInstr_asBC_BSRA,
    COPY = asEBCInstr_asBC_COPY,
    PshC8 = asEBCInstr_asBC_PshC8,
    PshVPtr = asEBCInstr_asBC_PshVPtr,
    RDSPtr = asEBCInstr_asBC_RDSPtr,
    CMPd = asEBCInstr_asBC_CMPd,
    CMPu = asEBCInstr_asBC_CMPu,
    CMPf = asEBCInstr_asBC_CMPf,
    CMPi = asEBCInstr_asBC_CMPi,
    CMPIi = asEBCInstr_asBC_CMPIi,
    CMPIf = asEBCInstr_asBC_CMPIf,
    CMPIu = asEBCInstr_asBC_CMPIu,
    JMPP = asEBCInstr_asBC_JMPP,
    PopRPtr = asEBCInstr_asBC_PopRPtr,
    PshRPtr = asEBCInstr_asBC_PshRPtr,
    STR = asEBCInstr_asBC_STR,
    CALLSYS = asEBCInstr_asBC_CALLSYS,
    CALLBND = asEBCInstr_asBC_CALLBND,
    SUSPEND = asEBCInstr_asBC_SUSPEND,
    ALLOC = asEBCInstr_asBC_ALLOC,
    FREE = asEBCInstr_asBC_FREE,
    LOADOBJ = asEBCInstr_asBC_LOADOBJ,
    STOREOBJ = asEBCInstr_asBC_STOREOBJ,
    GETOBJ = asEBCInstr_asBC_GETOBJ,
    REFCPY = asEBCInstr_asBC_REFCPY,
    CHKREF = asEBCInstr_asBC_CHKREF,
    GETOBJREF = asEBCInstr_asBC_GETOBJREF,
    GETREF = asEBCInstr_asBC_GETREF,
    PshNull = asEBCInstr_asBC_PshNull,
    ClrVPtr = asEBCInstr_asBC_ClrVPtr,
    OBJTYPE = asEBCInstr_asBC_OBJTYPE,
    TYPEID = asEBCInstr_asBC_TYPEID,
    SetV4 = asEBCInstr_asBC_SetV4,
    SetV8 = asEBCInstr_asBC_SetV8,
    ADDSi = asEBCInstr_asBC_ADDSi,
    CpyVtoV4 = asEBCInstr_asBC_CpyVtoV4,
    CpyVtoV8 = asEBCInstr_asBC_CpyVtoV8,
    CpyVtoR4 = asEBCInstr_asBC_CpyVtoR4,
    CpyVtoR8 = asEBCInstr_asBC_CpyVtoR8,
    CpyVtoG4 = asEBCInstr_asBC_CpyVtoG4,
    CpyRtoV4 = asEBCInstr_asBC_CpyRtoV4,
    CpyRtoV8 = asEBCInstr_asBC_CpyRtoV8,
    CpyGtoV4 = asEBCInstr_asBC_CpyGtoV4,
    WRTV1 = asEBCInstr_asBC_WRTV1,
    WRTV2 = asEBCInstr_asBC_WRTV2,
    WRTV4 = asEBCInstr_asBC_WRTV4,
    WRTV8 = asEBCInstr_asBC_WRTV8,
    RDR1 = asEBCInstr_asBC_RDR1,
    RDR2 = asEBCInstr_asBC_RDR2,
    RDR4 = asEBCInstr_asBC_RDR4,
    RDR8 = asEBCInstr_asBC_RDR8,
    LDG = asEBCInstr_asBC_LDG,
    LDV = asEBCInstr_asBC_LDV,
    PGA = asEBCInstr_asBC_PGA,
    CmpPtr = asEBCInstr_asBC_CmpPtr,
    VAR = asEBCInstr_asBC_VAR,
    iTOf = asEBCInstr_asBC_iTOf,
    fTOi = asEBCInstr_asBC_fTOi,
    uTOf = asEBCInstr_asBC_uTOf,
    fTOu = asEBCInstr_asBC_fTOu,
    sbTOi = asEBCInstr_asBC_sbTOi,
    swTOi = asEBCInstr_asBC_swTOi,
    ubTOi = asEBCInstr_asBC_ubTOi,
    uwTOi = asEBCInstr_asBC_uwTOi,
    dTOi = asEBCInstr_asBC_dTOi,
    dTOu = asEBCInstr_asBC_dTOu,
    dTOf = asEBCInstr_asBC_dTOf,
    iTOd = asEBCInstr_asBC_iTOd,
    uTOd = asEBCInstr_asBC_uTOd,
    fTOd = asEBCInstr_asBC_fTOd,
    ADDi = asEBCInstr_asBC_ADDi,
    SUBi = asEBCInstr_asBC_SUBi,
    MULi = asEBCInstr_asBC_MULi,
    DIVi = asEBCInstr_asBC_DIVi,
    MODi = asEBCInstr_asBC_MODi,
    ADDf = asEBCInstr_asBC_ADDf,
    SUBf = asEBCInstr_asBC_SUBf,
    MULf = asEBCInstr_asBC_MULf,
    DIVf = asEBCInstr_asBC_DIVf,
    MODf = asEBCInstr_asBC_MODf,
    ADDd = asEBCInstr_asBC_ADDd,
    SUBd = asEBCInstr_asBC_SUBd,
    MULd = asEBCInstr_asBC_MULd,
    DIVd = asEBCInstr_asBC_DIVd,
    MODd = asEBCInstr_asBC_MODd,
    ADDIi = asEBCInstr_asBC_ADDIi,
    SUBIi = asEBCInstr_asBC_SUBIi,
    MULIi = asEBCInstr_asBC_MULIi,
    ADDIf = asEBCInstr_asBC_ADDIf,
    SUBIf = asEBCInstr_asBC_SUBIf,
    MULIf = asEBCInstr_asBC_MULIf,
    SetG4 = asEBCInstr_asBC_SetG4,
    ChkRefS = asEBCInstr_asBC_ChkRefS,
    ChkNullV = asEBCInstr_asBC_ChkNullV,
    CALLINTF = asEBCInstr_asBC_CALLINTF,
    iTOb = asEBCInstr_asBC_iTOb,
    iTOw = asEBCInstr_asBC_iTOw,
    SetV1 = asEBCInstr_asBC_SetV1,
    SetV2 = asEBCInstr_asBC_SetV2,
    Cast = asEBCInstr_asBC_Cast,
    i64TOi = asEBCInstr_asBC_i64TOi,
    uTOi64 = asEBCInstr_asBC_uTOi64,
    iTOi64 = asEBCInstr_asBC_iTOi64,
    fTOi64 = asEBCInstr_asBC_fTOi64,
    dTOi64 = asEBCInstr_asBC_dTOi64,
    fTOu64 = asEBCInstr_asBC_fTOu64,
    dTOu64 = asEBCInstr_asBC_dTOu64,
    i64TOf = asEBCInstr_asBC_i64TOf,
    u64TOf = asEBCInstr_asBC_u64TOf,
    i64TOd = asEBCInstr_asBC_i64TOd,
    u64TOd = asEBCInstr_asBC_u64TOd,
    NEGi64 = asEBCInstr_asBC_NEGi64,
    INCi64 = asEBCInstr_asBC_INCi64,
    DECi64 = asEBCInstr_asBC_DECi64,
    BNOT64 = asEBCInstr_asBC_BNOT64,
    ADDi64 = asEBCInstr_asBC_ADDi64,
    SUBi64 = asEBCInstr_asBC_SUBi64,
    MULi64 = asEBCInstr_asBC_MULi64,
    DIVi64 = asEBCInstr_asBC_DIVi64,
    MODi64 = asEBCInstr_asBC_MODi64,
    BAND64 = asEBCInstr_asBC_BAND64,
    BOR64 = asEBCInstr_asBC_BOR64,
    BXOR64 = asEBCInstr_asBC_BXOR64,
    BSLL64 = asEBCInstr_asBC_BSLL64,
    BSRL64 = asEBCInstr_asBC_BSRL64,
    BSRA64 = asEBCInstr_asBC_BSRA64,
    CMPi64 = asEBCInstr_asBC_CMPi64,
    CMPu64 = asEBCInstr_asBC_CMPu64,
    ChkNullS = asEBCInstr_asBC_ChkNullS,
    ClrHi = asEBCInstr_asBC_ClrHi,
    JitEntry = asEBCInstr_asBC_JitEntry,
    CallPtr = asEBCInstr_asBC_CallPtr,
    FuncPtr = asEBCInstr_asBC_FuncPtr,
    LoadThisR = asEBCInstr_asBC_LoadThisR,
    PshV8 = asEBCInstr_asBC_PshV8,
    DIVu = asEBCInstr_asBC_DIVu,
    MODu = asEBCInstr_asBC_MODu,
    DIVu64 = asEBCInstr_asBC_DIVu64,
    MODu64 = asEBCInstr_asBC_MODu64,
    LoadRObjR = asEBCInstr_asBC_LoadRObjR,
    LoadVObjR = asEBCInstr_asBC_LoadVObjR,
    RefCpyV = asEBCInstr_asBC_RefCpyV,
    JLowZ = asEBCInstr_asBC_JLowZ,
    JLowNZ = asEBCInstr_asBC_JLowNZ,
    AllocMem = asEBCInstr_asBC_AllocMem,
    SetListSize = asEBCInstr_asBC_SetListSize,
    PshListElmnt = asEBCInstr_asBC_PshListElmnt,
    SetListType = asEBCInstr_asBC_SetListType,
    POWi = asEBCInstr_asBC_POWi,
    POWu = asEBCInstr_asBC_POWu,
    POWf = asEBCInstr_asBC_POWf,
    POWd = asEBCInstr_asBC_POWd,
    POWdi = asEBCInstr_asBC_POWdi,
    POWi64 = asEBCInstr_asBC_POWi64,
    POWu64 = asEBCInstr_asBC_POWu64,
    Thiscall1 = asEBCInstr_asBC_Thiscall1,
    MAXBYTECODE = asEBCInstr_asBC_MAXBYTECODE,
    TryBlock = asEBCInstr_asBC_TryBlock,
    VarDecl = asEBCInstr_asBC_VarDecl,
    Block = asEBCInstr_asBC_Block,
    ObjInfo = asEBCInstr_asBC_ObjInfo,
    LINE = asEBCInstr_asBC_LINE,
    LABEL = asEBCInstr_asBC_LABEL,
}

impl From<asEBCInstr> for BCInstr {
    fn from(value: asEBCInstr) -> Self {
        match value {
            asEBCInstr_asBC_PopPtr => Self::PopPtr,
            asEBCInstr_asBC_PshGPtr => Self::PshGPtr,
            asEBCInstr_asBC_PshC4 => Self::PshC4,
            asEBCInstr_asBC_PshV4 => Self::PshV4,
            asEBCInstr_asBC_PSF => Self::PSF,
            asEBCInstr_asBC_SwapPtr => Self::SwapPtr,
            asEBCInstr_asBC_NOT => Self::NOT,
            asEBCInstr_asBC_PshG4 => Self::PshG4,
            asEBCInstr_asBC_LdGRdR4 => Self::LdGRdR4,
            asEBCInstr_asBC_CALL => Self::CALL,
            asEBCInstr_asBC_RET => Self::RET,
            asEBCInstr_asBC_JMP => Self::JMP,
            asEBCInstr_asBC_JZ => Self::JZ,
            asEBCInstr_asBC_JNZ => Self::JNZ,
            asEBCInstr_asBC_JS => Self::JS,
            asEBCInstr_asBC_JNS => Self::JNS,
            asEBCInstr_asBC_JP => Self::JP,
            asEBCInstr_asBC_JNP => Self::JNP,
            asEBCInstr_asBC_TZ => Self::TZ,
            asEBCInstr_asBC_TNZ => Self::TNZ,
            asEBCInstr_asBC_TS => Self::TS,
            asEBCInstr_asBC_TNS => Self::TNS,
            asEBCInstr_asBC_TP => Self::TP,
            asEBCInstr_asBC_TNP => Self::TNP,
            asEBCInstr_asBC_NEGi => Self::NEGi,
            asEBCInstr_asBC_NEGf => Self::NEGf,
            asEBCInstr_asBC_NEGd => Self::NEGd,
            asEBCInstr_asBC_INCi16 => Self::INCi16,
            asEBCInstr_asBC_INCi8 => Self::INCi8,
            asEBCInstr_asBC_DECi16 => Self::DECi16,
            asEBCInstr_asBC_DECi8 => Self::DECi8,
            asEBCInstr_asBC_INCi => Self::INCi,
            asEBCInstr_asBC_DECi => Self::DECi,
            asEBCInstr_asBC_INCf => Self::INCf,
            asEBCInstr_asBC_DECf => Self::DECf,
            asEBCInstr_asBC_INCd => Self::INCd,
            asEBCInstr_asBC_DECd => Self::DECd,
            asEBCInstr_asBC_IncVi => Self::IncVi,
            asEBCInstr_asBC_DecVi => Self::DecVi,
            asEBCInstr_asBC_BNOT => Self::BNOT,
            asEBCInstr_asBC_BAND => Self::BAND,
            asEBCInstr_asBC_BOR => Self::BOR,
            asEBCInstr_asBC_BXOR => Self::BXOR,
            asEBCInstr_asBC_BSLL => Self::BSLL,
            asEBCInstr_asBC_BSRL => Self::BSRL,
            asEBCInstr_asBC_BSRA => Self::BSRA,
            asEBCInstr_asBC_COPY => Self::COPY,
            asEBCInstr_asBC_PshC8 => Self::PshC8,
            asEBCInstr_asBC_PshVPtr => Self::PshVPtr,
            asEBCInstr_asBC_RDSPtr => Self::RDSPtr,
            asEBCInstr_asBC_CMPd => Self::CMPd,
            asEBCInstr_asBC_CMPu => Self::CMPu,
            asEBCInstr_asBC_CMPf => Self::CMPf,
            asEBCInstr_asBC_CMPi => Self::CMPi,
            asEBCInstr_asBC_CMPIi => Self::CMPIi,
            asEBCInstr_asBC_CMPIf => Self::CMPIf,
            asEBCInstr_asBC_CMPIu => Self::CMPIu,
            asEBCInstr_asBC_JMPP => Self::JMPP,
            asEBCInstr_asBC_PopRPtr => Self::PopRPtr,
            asEBCInstr_asBC_PshRPtr => Self::PshRPtr,
            asEBCInstr_asBC_STR => Self::STR,
            asEBCInstr_asBC_CALLSYS => Self::CALLSYS,
            asEBCInstr_asBC_CALLBND => Self::CALLBND,
            asEBCInstr_asBC_SUSPEND => Self::SUSPEND,
            asEBCInstr_asBC_ALLOC => Self::ALLOC,
            asEBCInstr_asBC_FREE => Self::FREE,
            asEBCInstr_asBC_LOADOBJ => Self::LOADOBJ,
            asEBCInstr_asBC_STOREOBJ => Self::STOREOBJ,
            asEBCInstr_asBC_GETOBJ => Self::GETOBJ,
            asEBCInstr_asBC_REFCPY => Self::REFCPY,
            asEBCInstr_asBC_CHKREF => Self::CHKREF,
            asEBCInstr_asBC_GETOBJREF => Self::GETOBJREF,
            asEBCInstr_asBC_GETREF => Self::GETREF,
            asEBCInstr_asBC_PshNull => Self::PshNull,
            asEBCInstr_asBC_ClrVPtr => Self::ClrVPtr,
            asEBCInstr_asBC_OBJTYPE => Self::OBJTYPE,
            asEBCInstr_asBC_TYPEID => Self::TYPEID,
            asEBCInstr_asBC_SetV4 => Self::SetV4,
            asEBCInstr_asBC_SetV8 => Self::SetV8,
            asEBCInstr_asBC_ADDSi => Self::ADDSi,
            asEBCInstr_asBC_CpyVtoV4 => Self::CpyVtoV4,
            asEBCInstr_asBC_CpyVtoV8 => Self::CpyVtoV8,
            asEBCInstr_asBC_CpyVtoR4 => Self::CpyVtoR4,
            asEBCInstr_asBC_CpyVtoR8 => Self::CpyVtoR8,
            asEBCInstr_asBC_CpyVtoG4 => Self::CpyVtoG4,
            asEBCInstr_asBC_CpyRtoV4 => Self::CpyRtoV4,
            asEBCInstr_asBC_CpyRtoV8 => Self::CpyRtoV8,
            asEBCInstr_asBC_CpyGtoV4 => Self::CpyGtoV4,
            asEBCInstr_asBC_WRTV1 => Self::WRTV1,
            asEBCInstr_asBC_WRTV2 => Self::WRTV2,
            asEBCInstr_asBC_WRTV4 => Self::WRTV4,
            asEBCInstr_asBC_WRTV8 => Self::WRTV8,
            asEBCInstr_asBC_RDR1 => Self::RDR1,
            asEBCInstr_asBC_RDR2 => Self::RDR2,
            asEBCInstr_asBC_RDR4 => Self::RDR4,
            asEBCInstr_asBC_RDR8 => Self::RDR8,
            asEBCInstr_asBC_LDG => Self::LDG,
            asEBCInstr_asBC_LDV => Self::LDV,
            asEBCInstr_asBC_PGA => Self::PGA,
            asEBCInstr_asBC_CmpPtr => Self::CmpPtr,
            asEBCInstr_asBC_VAR => Self::VAR,
            asEBCInstr_asBC_iTOf => Self::iTOf,
            asEBCInstr_asBC_fTOi => Self::fTOi,
            asEBCInstr_asBC_uTOf => Self::uTOf,
            asEBCInstr_asBC_fTOu => Self::fTOu,
            asEBCInstr_asBC_sbTOi => Self::sbTOi,
            asEBCInstr_asBC_swTOi => Self::swTOi,
            asEBCInstr_asBC_ubTOi => Self::ubTOi,
            asEBCInstr_asBC_uwTOi => Self::uwTOi,
            asEBCInstr_asBC_dTOi => Self::dTOi,
            asEBCInstr_asBC_dTOu => Self::dTOu,
            asEBCInstr_asBC_dTOf => Self::dTOf,
            asEBCInstr_asBC_iTOd => Self::iTOd,
            asEBCInstr_asBC_uTOd => Self::uTOd,
            asEBCInstr_asBC_fTOd => Self::fTOd,
            asEBCInstr_asBC_ADDi => Self::ADDi,
            asEBCInstr_asBC_SUBi => Self::SUBi,
            asEBCInstr_asBC_MULi => Self::MULi,
            asEBCInstr_asBC_DIVi => Self::DIVi,
            asEBCInstr_asBC_MODi => Self::MODi,
            asEBCInstr_asBC_ADDf => Self::ADDf,
            asEBCInstr_asBC_SUBf => Self::SUBf,
            asEBCInstr_asBC_MULf => Self::MULf,
            asEBCInstr_asBC_DIVf => Self::DIVf,
            asEBCInstr_asBC_MODf => Self::MODf,
            asEBCInstr_asBC_ADDd => Self::ADDd,
            asEBCInstr_asBC_SUBd => Self::SUBd,
            asEBCInstr_asBC_MULd => Self::MULd,
            asEBCInstr_asBC_DIVd => Self::DIVd,
            asEBCInstr_asBC_MODd => Self::MODd,
            asEBCInstr_asBC_ADDIi => Self::ADDIi,
            asEBCInstr_asBC_SUBIi => Self::SUBIi,
            asEBCInstr_asBC_MULIi => Self::MULIi,
            asEBCInstr_asBC_ADDIf => Self::ADDIf,
            asEBCInstr_asBC_SUBIf => Self::SUBIf,
            asEBCInstr_asBC_MULIf => Self::MULIf,
            asEBCInstr_asBC_SetG4 => Self::SetG4,
            asEBCInstr_asBC_ChkRefS => Self::ChkRefS,
            asEBCInstr_asBC_ChkNullV => Self::ChkNullV,
            asEBCInstr_asBC_CALLINTF => Self::CALLINTF,
            asEBCInstr_asBC_iTOb => Self::iTOb,
            asEBCInstr_asBC_iTOw => Self::iTOw,
            asEBCInstr_asBC_SetV1 => Self::SetV1,
            asEBCInstr_asBC_SetV2 => Self::SetV2,
            asEBCInstr_asBC_Cast => Self::Cast,
            asEBCInstr_asBC_i64TOi => Self::i64TOi,
            asEBCInstr_asBC_uTOi64 => Self::uTOi64,
            asEBCInstr_asBC_iTOi64 => Self::iTOi64,
            asEBCInstr_asBC_fTOi64 => Self::fTOi64,
            asEBCInstr_asBC_dTOi64 => Self::dTOi64,
            asEBCInstr_asBC_fTOu64 => Self::fTOu64,
            asEBCInstr_asBC_dTOu64 => Self::dTOu64,
            asEBCInstr_asBC_i64TOf => Self::i64TOf,
            asEBCInstr_asBC_u64TOf => Self::u64TOf,
            asEBCInstr_asBC_i64TOd => Self::i64TOd,
            asEBCInstr_asBC_u64TOd => Self::u64TOd,
            asEBCInstr_asBC_NEGi64 => Self::NEGi64,
            asEBCInstr_asBC_INCi64 => Self::INCi64,
            asEBCInstr_asBC_DECi64 => Self::DECi64,
            asEBCInstr_asBC_BNOT64 => Self::BNOT64,
            asEBCInstr_asBC_ADDi64 => Self::ADDi64,
            asEBCInstr_asBC_SUBi64 => Self::SUBi64,
            asEBCInstr_asBC_MULi64 => Self::MULi64,
            asEBCInstr_asBC_DIVi64 => Self::DIVi64,
            asEBCInstr_asBC_MODi64 => Self::MODi64,
            asEBCInstr_asBC_BAND64 => Self::BAND64,
            asEBCInstr_asBC_BOR64 => Self::BOR64,
            asEBCInstr_asBC_BXOR64 => Self::BXOR64,
            asEBCInstr_asBC_BSLL64 => Self::BSLL64,
            asEBCInstr_asBC_BSRL64 => Self::BSRL64,
            asEBCInstr_asBC_BSRA64 => Self::BSRA64,
            asEBCInstr_asBC_CMPi64 => Self::CMPi64,
            asEBCInstr_asBC_CMPu64 => Self::CMPu64,
            asEBCInstr_asBC_ChkNullS => Self::ChkNullS,
            asEBCInstr_asBC_ClrHi => Self::ClrHi,
            asEBCInstr_asBC_JitEntry => Self::JitEntry,
            asEBCInstr_asBC_CallPtr => Self::CallPtr,
            asEBCInstr_asBC_FuncPtr => Self::FuncPtr,
            asEBCInstr_asBC_LoadThisR => Self::LoadThisR,
            asEBCInstr_asBC_PshV8 => Self::PshV8,
            asEBCInstr_asBC_DIVu => Self::DIVu,
            asEBCInstr_asBC_MODu => Self::MODu,
            asEBCInstr_asBC_DIVu64 => Self::DIVu64,
            asEBCInstr_asBC_MODu64 => Self::MODu64,
            asEBCInstr_asBC_LoadRObjR => Self::LoadRObjR,
            asEBCInstr_asBC_LoadVObjR => Self::LoadVObjR,
            asEBCInstr_asBC_RefCpyV => Self::RefCpyV,
            asEBCInstr_asBC_JLowZ => Self::JLowZ,
            asEBCInstr_asBC_JLowNZ => Self::JLowNZ,
            asEBCInstr_asBC_AllocMem => Self::AllocMem,
            asEBCInstr_asBC_SetListSize => Self::SetListSize,
            asEBCInstr_asBC_PshListElmnt => Self::PshListElmnt,
            asEBCInstr_asBC_SetListType => Self::SetListType,
            asEBCInstr_asBC_POWi => Self::POWi,
            asEBCInstr_asBC_POWu => Self::POWu,
            asEBCInstr_asBC_POWf => Self::POWf,
            asEBCInstr_asBC_POWd => Self::POWd,
            asEBCInstr_asBC_POWdi => Self::POWdi,
            asEBCInstr_asBC_POWi64 => Self::POWi64,
            asEBCInstr_asBC_POWu64 => Self::POWu64,
            asEBCInstr_asBC_Thiscall1 => Self::Thiscall1,
            asEBCInstr_asBC_MAXBYTECODE => Self::MAXBYTECODE,
            asEBCInstr_asBC_TryBlock => Self::TryBlock,
            asEBCInstr_asBC_VarDecl => Self::VarDecl,
            asEBCInstr_asBC_Block => Self::Block,
            asEBCInstr_asBC_ObjInfo => Self::ObjInfo,
            asEBCInstr_asBC_LINE => Self::LINE,
            asEBCInstr_asBC_LABEL => Self::LABEL,
            _ => panic!("Unknown bytecode instruction: {}", value),
        }
    }
}

impl From<BCInstr> for asEBCInstr {
    fn from(value: BCInstr) -> Self {
        value as asEBCInstr
    }
}

// Bytecode Type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BCType {
    Info = asEBCType_asBCTYPE_INFO,
    NoArg = asEBCType_asBCTYPE_NO_ARG,
    WArg = asEBCType_asBCTYPE_W_ARG,
    WwArg = asEBCType_asBCTYPE_wW_ARG,
    DwArg = asEBCType_asBCTYPE_DW_ARG,
    RwDwArg = asEBCType_asBCTYPE_rW_DW_ARG,
    QwArg = asEBCType_asBCTYPE_QW_ARG,
    DwDwArg = asEBCType_asBCTYPE_DW_DW_ARG,
    WwRwRwArg = asEBCType_asBCTYPE_wW_rW_rW_ARG,
    WwQwArg = asEBCType_asBCTYPE_wW_QW_ARG,
    WwRwArg = asEBCType_asBCTYPE_wW_rW_ARG,
    RwArg = asEBCType_asBCTYPE_rW_ARG,
    WwDwArg = asEBCType_asBCTYPE_wW_DW_ARG,
    WwRwDwArg = asEBCType_asBCTYPE_wW_rW_DW_ARG,
    RwRwArg = asEBCType_asBCTYPE_rW_rW_ARG,
    WwWArg = asEBCType_asBCTYPE_wW_W_ARG,
    QwDwArg = asEBCType_asBCTYPE_QW_DW_ARG,
    RwQwArg = asEBCType_asBCTYPE_rW_QW_ARG,
    WDwArg = asEBCType_asBCTYPE_W_DW_ARG,
    RwWDwArg = asEBCType_asBCTYPE_rW_W_DW_ARG,
    RwDwDwArg = asEBCType_asBCTYPE_rW_DW_DW_ARG,
}

impl From<asEBCType> for BCType {
    fn from(value: asEBCType) -> Self {
        match value {
            asEBCType_asBCTYPE_INFO => Self::Info,
            asEBCType_asBCTYPE_NO_ARG => Self::NoArg,
            asEBCType_asBCTYPE_W_ARG => Self::WArg,
            asEBCType_asBCTYPE_wW_ARG => Self::WwArg,
            asEBCType_asBCTYPE_DW_ARG => Self::DwArg,
            asEBCType_asBCTYPE_rW_DW_ARG => Self::RwDwArg,
            asEBCType_asBCTYPE_QW_ARG => Self::QwArg,
            asEBCType_asBCTYPE_DW_DW_ARG => Self::DwDwArg,
            asEBCType_asBCTYPE_wW_rW_rW_ARG => Self::WwRwRwArg,
            asEBCType_asBCTYPE_wW_QW_ARG => Self::WwQwArg,
            asEBCType_asBCTYPE_wW_rW_ARG => Self::WwRwArg,
            asEBCType_asBCTYPE_rW_ARG => Self::RwArg,
            asEBCType_asBCTYPE_wW_DW_ARG => Self::WwDwArg,
            asEBCType_asBCTYPE_wW_rW_DW_ARG => Self::WwRwDwArg,
            asEBCType_asBCTYPE_rW_rW_ARG => Self::RwRwArg,
            asEBCType_asBCTYPE_wW_W_ARG => Self::WwWArg,
            asEBCType_asBCTYPE_QW_DW_ARG => Self::QwDwArg,
            asEBCType_asBCTYPE_rW_QW_ARG => Self::RwQwArg,
            asEBCType_asBCTYPE_W_DW_ARG => Self::WDwArg,
            asEBCType_asBCTYPE_rW_W_DW_ARG => Self::RwWDwArg,
            asEBCType_asBCTYPE_rW_DW_DW_ARG => Self::RwDwDwArg,
            _ => panic!("Unknown bytecode type: {}", value),
        }
    }
}

impl From<BCType> for asEBCType {
    fn from(value: BCType) -> Self {
        value as asEBCType
    }
}
