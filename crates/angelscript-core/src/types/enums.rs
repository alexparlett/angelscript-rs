#![allow(non_upper_case_globals)]

use angelscript_sys::*;
use bitflags::bitflags;
use std::hash::Hash;

/// AngelScript return codes indicating the result of operations.
///
/// These codes are returned by most AngelScript functions to indicate
/// success, failure, or specific error conditions. They correspond directly
/// to the AngelScript C API return codes.
///
/// # Usage
///
/// Return codes are typically used with [`ScriptError::from_code`] to convert
/// them into Rust-style error handling:
///
/// ```rust
/// use angelscript_rs::{ReturnCode, ScriptError};
///
/// let result = some_angelscript_operation();
/// match ScriptError::from_code(result) {
///     Ok(()) => println!("Operation succeeded"),
///     Err(ScriptError::AngelScriptError(ReturnCode::InvalidName)) => {
///         println!("Invalid name provided");
///     }
///     Err(e) => println!("Other error: {}", e),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ReturnCode {
    /// Operation completed successfully.
    Success = asERetCodes_asSUCCESS,
    /// Generic error occurred.
    Error = asERetCodes_asERROR,
    /// Context is currently active and cannot be modified.
    ContextActive = asERetCodes_asCONTEXT_ACTIVE,
    /// Context execution has not finished.
    ContextNotFinished = asERetCodes_asCONTEXT_NOT_FINISHED,
    /// Context has not been prepared for execution.
    ContextNotPrepared = asERetCodes_asCONTEXT_NOT_PREPARED,
    /// Invalid argument provided to function.
    InvalidArg = asERetCodes_asINVALID_ARG,
    /// Function not found.
    NoFunction = asERetCodes_asNO_FUNCTION,
    /// Operation not supported.
    NotSupported = asERetCodes_asNOT_SUPPORTED,
    /// Invalid name provided (e.g., contains illegal characters).
    InvalidName = asERetCodes_asINVALID_NAME,
    /// Name is already taken.
    NameTaken = asERetCodes_asNAME_TAKEN,
    /// Invalid declaration syntax.
    InvalidDeclaration = asERetCodes_asINVALID_DECLARATION,
    /// Invalid object reference.
    InvalidObject = asERetCodes_asINVALID_OBJECT,
    /// Invalid type specified.
    InvalidType = asERetCodes_asINVALID_TYPE,
    /// Item is already registered.
    AlreadyRegistered = asERetCodes_asALREADY_REGISTERED,
    /// Multiple functions match the criteria.
    MultipleFunctions = asERetCodes_asMULTIPLE_FUNCTIONS,
    /// Module not found.
    NoModule = asERetCodes_asNO_MODULE,
    /// Global variable not found.
    NoGlobalVar = asERetCodes_asNO_GLOBAL_VAR,
    /// Invalid engine configuration.
    InvalidConfiguration = asERetCodes_asINVALID_CONFIGURATION,
    /// Invalid interface definition.
    InvalidInterface = asERetCodes_asINVALID_INTERFACE,
    /// Cannot bind all imported functions.
    CantBindAllFunctions = asERetCodes_asCANT_BIND_ALL_FUNCTIONS,
    /// Lower array dimension not registered.
    LowerArrayDimensionNotRegistered = asERetCodes_asLOWER_ARRAY_DIMENSION_NOT_REGISTERED,
    /// Wrong configuration group.
    WrongConfigGroup = asERetCodes_asWRONG_CONFIG_GROUP,
    /// Configuration group is currently in use.
    ConfigGroupIsInUse = asERetCodes_asCONFIG_GROUP_IS_IN_USE,
    /// Illegal behaviour for this type.
    IllegalBehaviourForType = asERetCodes_asILLEGAL_BEHAVIOUR_FOR_TYPE,
    /// Wrong calling convention specified.
    WrongCallingConv = asERetCodes_asWRONG_CALLING_CONV,
    /// Build operation is already in progress.
    BuildInProgress = asERetCodes_asBUILD_IN_PROGRESS,
    /// Failed to initialize global variables.
    InitGlobalVarsFailed = asERetCodes_asINIT_GLOBAL_VARS_FAILED,
    /// Out of memory.
    OutOfMemory = asERetCodes_asOUT_OF_MEMORY,
    /// Module is currently in use and cannot be modified.
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

/// Configuration properties for the AngelScript engine.
///
/// These properties control various aspects of the engine's behavior,
/// compilation settings, and runtime characteristics.
///
/// # Usage
///
/// ```rust
/// use angelscript_rs::{Engine, EngineProperty};
///
/// let engine = Engine::create()?;
///
/// // Enable optimizations
/// engine.set_engine_property(EngineProperty::OptimizeBytecode, 1)?;
///
/// // Set maximum stack size
/// engine.set_engine_property(EngineProperty::MaxStackSize, 1024 * 1024)?;
///
/// // Enable automatic garbage collection
/// engine.set_engine_property(EngineProperty::AutoGarbageCollect, 1)?;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EngineProperty {
    /// Allow unsafe references (default: false).
    AllowUnsafeReferences = asEEngineProp_asEP_ALLOW_UNSAFE_REFERENCES,
    /// Optimize bytecode during compilation (default: true).
    OptimizeBytecode = asEEngineProp_asEP_OPTIMIZE_BYTECODE,
    /// Copy script sections to allow reuse (default: true).
    CopyScriptSections = asEEngineProp_asEP_COPY_SCRIPT_SECTIONS,
    /// Maximum stack size in bytes (default: 0 = no limit).
    MaxStackSize = asEEngineProp_asEP_MAX_STACK_SIZE,
    /// Use character literals (default: false).
    UseCharacterLiterals = asEEngineProp_asEP_USE_CHARACTER_LITERALS,
    /// Allow multiline strings (default: false).
    AllowMultilineStrings = asEEngineProp_asEP_ALLOW_MULTILINE_STRINGS,
    /// Allow implicit handle types (default: true).
    AllowImplicitHandleTypes = asEEngineProp_asEP_ALLOW_IMPLICIT_HANDLE_TYPES,
    /// Build without line cues for smaller bytecode (default: false).
    BuildWithoutLineCues = asEEngineProp_asEP_BUILD_WITHOUT_LINE_CUES,
    /// Initialize global variables after build (default: true).
    InitGlobalVarsAfterBuild = asEEngineProp_asEP_INIT_GLOBAL_VARS_AFTER_BUILD,
    /// Require enum scope (default: false).
    RequireEnumScope = asEEngineProp_asEP_REQUIRE_ENUM_SCOPE,
    /// Script scanner mode (default: 0).
    ScriptScanner = asEEngineProp_asEP_SCRIPT_SCANNER,
    /// Include JIT instructions in bytecode (default: false).
    IncludeJitInstructions = asEEngineProp_asEP_INCLUDE_JIT_INSTRUCTIONS,
    /// String encoding (default: 0 = UTF8).
    StringEncoding = asEEngineProp_asEP_STRING_ENCODING,
    /// Property accessor mode (default: 0).
    PropertyAccessorMode = asEEngineProp_asEP_PROPERTY_ACCESSOR_MODE,
    /// Expand default array to template (default: false).
    ExpandDefArrayToTmpl = asEEngineProp_asEP_EXPAND_DEF_ARRAY_TO_TMPL,
    /// Automatic garbage collection (default: false).
    AutoGarbageCollect = asEEngineProp_asEP_AUTO_GARBAGE_COLLECT,
    /// Disallow global variables (default: false).
    DisallowGlobalVars = asEEngineProp_asEP_DISALLOW_GLOBAL_VARS,
    /// Always implement default constructor (default: false).
    AlwaysImplDefaultConstruct = asEEngineProp_asEP_ALWAYS_IMPL_DEFAULT_CONSTRUCT,
    /// Compiler warnings level (default: 0).
    CompilerWarnings = asEEngineProp_asEP_COMPILER_WARNINGS,
    /// Disallow value assignment for reference types (default: false).
    DisallowValueAssignForRefType = asEEngineProp_asEP_DISALLOW_VALUE_ASSIGN_FOR_REF_TYPE,
    /// Alter syntax for named arguments (default: 0).
    AlterSyntaxNamedArgs = asEEngineProp_asEP_ALTER_SYNTAX_NAMED_ARGS,
    /// Disable integer division (default: false).
    DisableIntegerDivision = asEEngineProp_asEP_DISABLE_INTEGER_DIVISION,
    /// Disallow empty list elements (default: false).
    DisallowEmptyListElements = asEEngineProp_asEP_DISALLOW_EMPTY_LIST_ELEMENTS,
    /// Treat private properties as protected (default: false).
    PrivatePropAsProtected = asEEngineProp_asEP_PRIVATE_PROP_AS_PROTECTED,
    /// Allow Unicode identifiers (default: false).
    AllowUnicodeIdentifiers = asEEngineProp_asEP_ALLOW_UNICODE_IDENTIFIERS,
    /// Heredoc trim mode (default: 0).
    HeredocTrimMode = asEEngineProp_asEP_HEREDOC_TRIM_MODE,
    /// Maximum nested calls (default: 100).
    MaxNestedCalls = asEEngineProp_asEP_MAX_NESTED_CALLS,
    /// Generic call mode (default: 0).
    GenericCallMode = asEEngineProp_asEP_GENERIC_CALL_MODE,
    /// Initial stack size (default: 4096).
    InitStackSize = asEEngineProp_asEP_INIT_STACK_SIZE,
    /// Initial call stack size (default: 10).
    InitCallStackSize = asEEngineProp_asEP_INIT_CALL_STACK_SIZE,
    /// Maximum call stack size (default: 0 = no limit).
    MaxCallStackSize = asEEngineProp_asEP_MAX_CALL_STACK_SIZE,
    /// Ignore duplicate shared interfaces (default: false).
    IgnoreDuplicateSharedIntf = asEEngineProp_asEP_IGNORE_DUPLICATE_SHARED_INTF,
    /// Disable debug output (default: false).
    NoDebugOutput = asEEngineProp_asEP_NO_DEBUG_OUTPUT,
    /// Disable script class garbage collection (default: false).
    DisableScriptClassGc = asEEngineProp_asEP_DISABLE_SCRIPT_CLASS_GC,
    /// JIT interface version (read-only).
    JitInterfaceVersion = asEEngineProp_asEP_JIT_INTERFACE_VERSION,
    /// Always implement default copy (default: false).
    AlwaysImplDefaultCopy = asEEngineProp_asEP_ALWAYS_IMPL_DEFAULT_COPY,
    /// Always implement default copy constructor (default: false).
    AlwaysImplDefaultCopyConstruct = asEEngineProp_asEP_ALWAYS_IMPL_DEFAULT_COPY_CONSTRUCT,
    /// Last property marker (internal use).
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

/// Calling conventions for registered functions.
///
/// These specify how arguments are passed and how the stack is managed
/// when calling functions registered with AngelScript.
///
/// # Platform Support
///
/// Not all calling conventions are supported on all platforms:
/// - `Cdecl`: Supported on all platforms
/// - `Stdcall`: Windows only
/// - `Thiscall`: Windows only (for class methods)
/// - `Generic`: Supported on all platforms (recommended for cross-platform code)
///
/// # Usage
///
/// ```rust
/// use angelscript_rs::{Engine, CallingConvention};
///
/// let engine = Engine::create()?;
///
/// // Register a function with cdecl calling convention
/// engine.register_global_function(
///     "void myFunction(int)",
///     my_function_ptr,
///     CallingConvention::Cdecl,
///     None
/// )?;
///
/// // Generic calling convention works on all platforms
/// engine.register_global_function(
///     "void genericFunction(int)",
///     generic_function_wrapper,
///     CallingConvention::Generic,
///     None
/// )?;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CallingConvention {
    /// C declaration calling convention (caller cleans stack).
    Cdecl = asECallConvTypes_asCALL_CDECL,
    /// Standard calling convention (callee cleans stack, Windows only).
    Stdcall = asECallConvTypes_asCALL_STDCALL,
    /// This-call as global function (Windows only).
    ThiscallAsGlobal = asECallConvTypes_asCALL_THISCALL_ASGLOBAL,
    /// This-call for class methods (Windows only).
    Thiscall = asECallConvTypes_asCALL_THISCALL,
    /// Cdecl with object as last parameter.
    CdeclObjLast = asECallConvTypes_asCALL_CDECL_OBJLAST,
    /// Cdecl with object as first parameter.
    CdeclObjFirst = asECallConvTypes_asCALL_CDECL_OBJFIRST,
    /// Generic calling convention (cross-platform).
    Generic = asECallConvTypes_asCALL_GENERIC,
    /// This-call with object as last parameter.
    ThiscallObjLast = asECallConvTypes_asCALL_THISCALL_OBJLAST,
    /// This-call with object as first parameter.
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

bitflags! {
    /// Flags that control object type registration and behavior.
    ///
    /// These flags specify how AngelScript should handle objects of a particular type,
    /// including memory management, garbage collection, and calling conventions.
    ///
    /// # Common Combinations
    ///
    /// ```rust
    /// use angelscript_rs::ObjectTypeFlags;
    ///
    /// // Reference type with garbage collection
    /// let ref_gc = ObjectTypeFlags::REF | ObjectTypeFlags::GC;
    ///
    /// // Value type (POD - Plain Old Data)
    /// let value_pod = ObjectTypeFlags::VALUE | ObjectTypeFlags::POD;
    ///
    /// // Application class with constructor and destructor
    /// let app_class = ObjectTypeFlags::REF |
    ///                 ObjectTypeFlags::APP_CLASS_CD;
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ObjectTypeFlags: asEObjTypeFlags {
        /// Reference type (allocated on heap).
        const REF = asEObjTypeFlags_asOBJ_REF;
        /// Value type (allocated on stack or embedded).
        const VALUE = asEObjTypeFlags_asOBJ_VALUE;
        /// Participates in garbage collection.
        const GC = asEObjTypeFlags_asOBJ_GC;
        /// Plain Old Data (no constructor/destructor needed).
        const POD = asEObjTypeFlags_asOBJ_POD;
        /// Cannot be used as handle type.
        const NOHANDLE = asEObjTypeFlags_asOBJ_NOHANDLE;
        /// Scoped reference (automatically released).
        const SCOPED = asEObjTypeFlags_asOBJ_SCOPED;
        /// Template type.
        const TEMPLATE = asEObjTypeFlags_asOBJ_TEMPLATE;
        /// Use as handle type.
        const ASHANDLE = asEObjTypeFlags_asOBJ_ASHANDLE;
        /// Application class.
        const APP_CLASS = asEObjTypeFlags_asOBJ_APP_CLASS;
        /// Has constructor.
        const APP_CLASS_CONSTRUCTOR = asEObjTypeFlags_asOBJ_APP_CLASS_CONSTRUCTOR;
        /// Has destructor.
        const APP_CLASS_DESTRUCTOR = asEObjTypeFlags_asOBJ_APP_CLASS_DESTRUCTOR;
        /// Has assignment operator.
        const APP_CLdASS_ASSIGNMENT = asEObjTypeFlags_asOBJ_APP_CLASS_ASSIGNMENT;
        /// Has copy constructor.
        const APP_CLASS_COPY_CONSTRUCTOR = asEObjTypeFlags_asOBJ_APP_CLASS_COPY_CONSTRUCTOR;
        /// Constructor only.
        const APP_CLASS_C = asEObjTypeFlags_asOBJ_APP_CLASS_C;
        /// Constructor and destructor.
        const APP_CLASS_CD = asEObjTypeFlags_asOBJ_APP_CLASS_CD;
        /// Constructor and assignment.
        const APP_CLASS_CA = asEObjTypeFlags_asOBJ_APP_CLASS_CA;
        /// Constructor and copy constructor.
        const APP_CLASS_CK = asEObjTypeFlags_asOBJ_APP_CLASS_CK;
        /// Constructor, destructor, and assignment.
        const APP_CLASS_CDA = asEObjTypeFlags_asOBJ_APP_CLASS_CDA;
        /// Constructor, destructor, and copy constructor.
        const APP_CLASS_CDK = asEObjTypeFlags_asOBJ_APP_CLASS_CDK;
        /// Constructor, assignment, and copy constructor.
        const APP_CLASS_CAK = asEObjTypeFlags_asOBJ_APP_CLASS_CAK;
        /// All: constructor, destructor, assignment, and copy constructor.
        const APP_CLASS_CDAK = asEObjTypeFlags_asOBJ_APP_CLASS_CDAK;
        /// Destructor only.
        const APP_CLASS_D = asEObjTypeFlags_asOBJ_APP_CLASS_D;
        /// Destructor and assignment.
        const APP_CLASS_DA = asEObjTypeFlags_asOBJ_APP_CLASS_DA;
        /// Destructor and copy constructor.
        const APP_CLASS_DK = asEObjTypeFlags_asOBJ_APP_CLASS_DK;
        /// Destructor, assignment, and copy constructor.
        const APP_CLASS_DAK = asEObjTypeFlags_asOBJ_APP_CLASS_DAK;
        /// Assignment only.
        const APP_CLASS_A = asEObjTypeFlags_asOBJ_APP_CLASS_A;
        /// Assignment and copy constructor.
        const APP_CLASS_AK = asEObjTypeFlags_asOBJ_APP_CLASS_AK;
        /// Copy constructor only.
        const APP_CLASS_K = asEObjTypeFlags_asOBJ_APP_CLASS_K;
        /// Has additional constructors.
        const APP_CLASS_MORE_CONSTRUCTORS = asEObjTypeFlags_asOBJ_APP_CLASS_MORE_CONSTRUCTORS;
        /// Application primitive type.
        const APP_PRIMITIVE = asEObjTypeFlags_asOBJ_APP_PRIMITIVE;
        /// Application floating point type.
        const APP_FLOAT = asEObjTypeFlags_asOBJ_APP_FLOAT;
        /// Application array type.
        const APP_ARRAY = asEObjTypeFlags_asOBJ_APP_ARRAY;
        /// All integer types.
        const APP_CLASS_ALLINTS = asEObjTypeFlags_asOBJ_APP_CLASS_ALLINTS;
        /// All floating point types.
        const APP_CLASS_ALLFLOATS = asEObjTypeFlags_asOBJ_APP_CLASS_ALLFLOATS;
        /// No reference counting.
        const NOCOUNT = asEObjTypeFlags_asOBJ_NOCOUNT;
        /// 8-byte alignment.
        const APP_CLASS_ALIGN8 = asEObjTypeFlags_asOBJ_APP_CLASS_ALIGN8;
        /// Implicit handle type.
        const IMPLICIT_HANDLE = asEObjTypeFlags_asOBJ_IMPLICIT_HANDLE;
        /// Union type.
        const APP_CLASS_UNION = asEObjTypeFlags_asOBJ_APP_CLASS_UNION;
        /// Mask for valid flags.
        const MASK_VALID_FLAGS = asEObjTypeFlags_asOBJ_MASK_VALID_FLAGS;
        /// Script object.
        const SCRIPT_OBJECT = asEObjTypeFlags_asOBJ_SCRIPT_OBJECT;
        /// Shared between modules.
        const SHARED = asEObjTypeFlags_asOBJ_SHARED;
        /// Cannot be inherited.
        const NOINHERIT = asEObjTypeFlags_asOBJ_NOINHERIT;
        /// Function definition.
        const FUNCDEF = asEObjTypeFlags_asOBJ_FUNCDEF;
        /// List pattern.
        const LIST_PATTERN = asEObjTypeFlags_asOBJ_LIST_PATTERN;
        /// Enumeration.
        const ENUM = asEObjTypeFlags_asOBJ_ENUM;
        /// Template subtype.
        const TEMPLATE_SUBTYPE = asEObjTypeFlags_asOBJ_TEMPLATE_SUBTYPE;
        /// Type definition.
        const TYPEDEF = asEObjTypeFlags_asOBJ_TYPEDEF;
        /// Abstract class.
        const ABSTRACT = asEObjTypeFlags_asOBJ_ABSTRACT;
        /// 16-byte alignment.
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

/// Object behaviors that can be registered with AngelScript.
///
/// Behaviors define how objects are created, destroyed, and managed
/// by the AngelScript engine and garbage collector.
///
/// # Usage
///
/// ```rust
/// use angelscript_rs::{Engine, Behaviour};
///
/// let engine = Engine::create()?;
///
/// // Register constructor
/// engine.register_object_behaviour(
///     "MyClass",
///     Behaviour::Construct,
///     "void f()",
///     constructor_function,
///     None, None, None
/// )?;
///
/// // Register destructor
/// engine.register_object_behaviour(
///     "MyClass",
///     Behaviour::Destruct,
///     "void f()",
///     destructor_function,
///     None, None, None
/// )?;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Behaviour {
    /// Object constructor.
    Construct = asEBehaviours_asBEHAVE_CONSTRUCT,
    /// List constructor (for initialization lists).
    ListConstruct = asEBehaviours_asBEHAVE_LIST_CONSTRUCT,
    /// Object destructor.
    Destruct = asEBehaviours_asBEHAVE_DESTRUCT,
    /// Object factory function.
    Factory = asEBehaviours_asBEHAVE_FACTORY,
    /// List factory function.
    ListFactory = asEBehaviours_asBEHAVE_LIST_FACTORY,
    /// Add reference (for reference counting).
    AddRef = asEBehaviours_asBEHAVE_ADDREF,
    /// Release reference (for reference counting).
    Release = asEBehaviours_asBEHAVE_RELEASE,
    /// Get weak reference flag.
    GetWeakRefFlag = asEBehaviours_asBEHAVE_GET_WEAKREF_FLAG,
    /// Template callback.
    TemplateCallback = asEBehaviours_asBEHAVE_TEMPLATE_CALLBACK,
    /// Get reference count.
    GetRefCount = asEBehaviours_asBEHAVE_GETREFCOUNT,
    /// Set garbage collection flag.
    SetGcFlag = asEBehaviours_asBEHAVE_SETGCFLAG,
    /// Get garbage collection flag.
    GetGcFlag = asEBehaviours_asBEHAVE_GETGCFLAG,
    /// Enumerate references (for garbage collection).
    EnumRefs = asEBehaviours_asBEHAVE_ENUMREFS,
    /// Release references (for garbage collection).
    ReleaseRefs = asEBehaviours_asBEHAVE_RELEASEREFS,
    /// Maximum behavior value.
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

/// Execution state of a script context.
///
/// These states indicate the current status of script execution
/// and help determine what operations are valid.
///
/// # State Transitions
///
/// ```text
/// Uninitialized -> Prepared -> Active -> Finished
///                           -> Suspended -> Active
///                           -> Aborted
///                           -> Exception
/// ```
///
/// # Usage
///
/// ```rust
/// use angelscript_rs::{Context, ContextState};
///
/// let context = engine.create_context()?;
/// let function = module.get_function_by_name("myFunction")?;
///
/// context.prepare(&function)?;
/// assert_eq!(context.get_state(), ContextState::Prepared);
///
/// match context.execute()? {
///     ContextState::Finished => println!("Execution completed"),
///     ContextState::Suspended => println!("Execution suspended"),
///     ContextState::Exception => println!("Exception occurred"),
///     _ => println!("Other state"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ContextState {
    /// Execution completed successfully.
    Finished = asEContextState_asEXECUTION_FINISHED,
    /// Execution was suspended.
    Suspended = asEContextState_asEXECUTION_SUSPENDED,
    /// Execution was aborted.
    Aborted = asEContextState_asEXECUTION_ABORTED,
    /// An exception occurred during execution.
    Exception = asEContextState_asEXECUTION_EXCEPTION,
    /// Context is prepared for execution.
    Prepared = asEContextState_asEXECUTION_PREPARED,
    /// Context is uninitialized.
    Uninitialized = asEContextState_asEXECUTION_UNINITIALIZED,
    /// Context is currently executing.
    Active = asEContextState_asEXECUTION_ACTIVE,
    /// An error occurred.
    Error = asEContextState_asEXECUTION_ERROR,
    /// Context is in deserialization mode.
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

/// Types of messages that can be generated during compilation.
///
/// These are used in message callbacks to categorize the severity
/// of compilation messages.
///
/// # Usage
///
/// ```rust
/// use angelscript_rs::{Engine, MessageType};
///
/// let mut engine = Engine::create()?;
///
/// engine.set_message_callback(|msg_type, section, row, col, message| {
///     match msg_type {
///         MessageType::Error => eprintln!("Error: {}", message),
///         MessageType::Warning => println!("Warning: {}", message),
///         MessageType::Information => println!("Info: {}", message),
///     }
/// })?;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MessageType {
    /// Error message (compilation will fail).
    Error = asEMsgType_asMSGTYPE_ERROR,
    /// Warning message (compilation will succeed).
    Warning = asEMsgType_asMSGTYPE_WARNING,
    /// Informational message.
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

bitflags! {
    /// Flags controlling garbage collection behavior.
    ///
    /// These flags can be combined to control how the garbage collector
    /// operates when called.
    ///
    /// # Usage
    ///
    /// ```rust
    /// use angelscript_rs::{Engine, GCFlags};
    ///
    /// let engine = Engine::create()?;
    ///
    /// // Perform a full garbage collection cycle
    /// engine.garbage_collect(GCFlags::FULL_CYCLE.bits(), 0)?;
    ///
    /// // Detect garbage without destroying it
    /// engine.garbage_collect(GCFlags::DETECT_GARBAGE.bits(), 0)?;
    ///
    /// // Destroy previously detected garbage
    /// engine.garbage_collect(GCFlags::DESTROY_GARBAGE.bits(), 0)?;
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct GCFlags: u32 {
        /// Perform a full garbage collection cycle.
        const FULL_CYCLE = asEGCFlags_asGC_FULL_CYCLE;
        /// Perform one step of garbage collection.
        const ONE_STEP = asEGCFlags_asGC_ONE_STEP;
        /// Destroy detected garbage objects.
        const DESTROY_GARBAGE = asEGCFlags_asGC_DESTROY_GARBAGE;
        /// Detect garbage objects without destroying them.
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

/// Classification of tokens in AngelScript source code.
///
/// Used by the token parser to categorize different elements
/// of the script syntax.
///
/// # Usage
///
/// ```rust
/// use angelscript_rs::{Engine, TokenClass};
///
/// let engine = Engine::create()?;
/// let (token_class, length) = engine.parse_token("function");
///
/// match token_class {
///     TokenClass::Keyword => println!("Found keyword"),
///     TokenClass::Identifier => println!("Found identifier"),
///     TokenClass::Value => println!("Found value"),
///     _ => println!("Other token type"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TokenClass {
    /// Unknown token type.
    Unknown = asETokenClass_asTC_UNKNOWN,
    /// Language keyword.
    Keyword = asETokenClass_asTC_KEYWORD,
    /// Literal value.
    Value = asETokenClass_asTC_VALUE,
    /// Identifier (variable, function name, etc.).
    Identifier = asETokenClass_asTC_IDENTIFIER,
    /// Comment.
    Comment = asETokenClass_asTC_COMMENT,
    /// Whitespace.
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

bitflags! {
    /// Type identifiers for built-in and custom types.
    ///
    /// These identify the different data types that AngelScript can work with,
    /// including primitive types and custom object types.
    ///
    /// # Usage
    ///
    /// ```rust
    /// let engine = Engine::create()?;
    ///
    /// // Get type info for built-in types
    /// if let Some(type_info) = engine.get_type_info_by_id(TypeId::Int32) {
    ///     println!("Found int32 type");
    /// }
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TypeId: asETypeIdFlags {
        /// Void type.
        const Void = asETypeIdFlags_asTYPEID_VOID;
        /// Boolean type.
        const Bool = asETypeIdFlags_asTYPEID_BOOL;
        /// 8-bit signed integer.
        const Int8 = asETypeIdFlags_asTYPEID_INT8;
        /// 16-bit signed integer.
        const Int16 = asETypeIdFlags_asTYPEID_INT16;
        /// 32-bit signed integer.
        const Int32 = asETypeIdFlags_asTYPEID_INT32;
        /// 64-bit signed integer.
        const  Int64 = asETypeIdFlags_asTYPEID_INT64;
        /// 8-bit unsigned integer.
        const Uint8 = asETypeIdFlags_asTYPEID_UINT8;
        /// 16-bit unsigned integer.
        const Uint16 = asETypeIdFlags_asTYPEID_UINT16;
        /// 32-bit unsigned integer.
        const Uint32 = asETypeIdFlags_asTYPEID_UINT32;
        /// 64-bit unsigned integer.
        const Uint64 = asETypeIdFlags_asTYPEID_UINT64;
        /// 32-bit floating point.
        const Float = asETypeIdFlags_asTYPEID_FLOAT;
        /// 64-bit floating point.
        const Double = asETypeIdFlags_asTYPEID_DOUBLE;
        /// Object handle.
        const ObjHandle = asETypeIdFlags_asTYPEID_OBJHANDLE;
        /// Handle to const object.
        const HandleToConst = asETypeIdFlags_asTYPEID_HANDLETOCONST;
        /// Object mask.
        const MaskObject = asETypeIdFlags_asTYPEID_MASK_OBJECT;
        /// Application object.
        const AppObject = asETypeIdFlags_asTYPEID_APPOBJECT;
        /// Script object.
        const ScriptObject = asETypeIdFlags_asTYPEID_SCRIPTOBJECT;
        /// Template type.
        const Template = asETypeIdFlags_asTYPEID_TEMPLATE;
        /// Sequence number mask.
        const MaskSeqnr = asETypeIdFlags_asTYPEID_MASK_SEQNBR;
    }
}

impl From<asETypeIdFlags> for TypeId {
    fn from(value: asETypeIdFlags) -> Self {
        Self::from_bits_truncate(value)
    }
}

impl From<TypeId> for asETypeIdFlags {
    fn from(value: TypeId) -> Self {
        value.bits()
    }
}

bitflags! {
    /// Type modifiers for function parameters and variables.
    ///
    /// These flags specify how parameters are passed and what
    /// access restrictions apply to them.
    ///
    /// # Usage
    ///
    /// ```rust
    /// use angelscript_rs::TypeModifiers;
    ///
    /// // Check if a parameter is const
    /// if modifiers.contains(TypeModifiers::CONST) {
    ///     println!("Parameter is const");
    /// }
    ///
    /// // Check if it's an output reference
    /// if modifiers.contains(TypeModifiers::OUTREF) {
    ///     println!("Parameter is an output reference");
    /// }
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TypeModifiers: u32 {
        /// No modifiers.
        const NONE = asETypeModifiers_asTM_NONE;
        /// Input reference.
        const INREF = asETypeModifiers_asTM_INREF;
        /// Output reference.
        const OUTREF = asETypeModifiers_asTM_OUTREF;
        /// Input/output reference.
        const INOUTREF = asETypeModifiers_asTM_INOUTREF;
        /// Const modifier.
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

/// Flags controlling module retrieval behavior.
///
/// These flags determine how [`Engine::get_module`] behaves when
/// requesting a module by name.
///
/// # Usage
///
/// ```rust
/// use angelscript_rs::{Engine, GetModuleFlags};
///
/// let engine = Engine::create()?;
///
/// // Get existing module or fail
/// if let Ok(module) = engine.get_module("MyModule", GetModuleFlags::OnlyIfExists) {
///     println!("Module exists");
/// }
///
/// // Get existing module or create new one
/// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
///
/// // Always create a new module (discarding existing)
/// let module = engine.get_module("MyModule", GetModuleFlags::AlwaysCreate)?;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GetModuleFlags {
    /// Only return the module if it already exists.
    OnlyIfExists = asEGMFlags_asGM_ONLY_IF_EXISTS,
    /// Create the module if it doesn't exist.
    CreateIfNotExists = asEGMFlags_asGM_CREATE_IF_NOT_EXISTS,
    /// Always create a new module (discard existing).
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

bitflags! {
    /// Flags controlling function compilation behavior.
    ///
    /// These flags affect how functions are compiled and integrated
    /// into modules.
    ///
    /// # Usage
    ///
    /// ```rust
    /// use angelscript_rs::{Module, CompileFlags};
    ///
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    ///
    /// // Compile function and add to module
    /// let function = module.compile_function(
    ///     "test",
    ///     "int add(int a, int b) { return a + b; }",
    ///     0,
    ///     CompileFlags::ADD_TO_MODULE.bits()
    /// )?;
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CompileFlags: u32 {
        /// Add the compiled function to the module.
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

/// Types of functions in AngelScript.
///
/// This enum categorizes the different kinds of functions that
/// can exist in the AngelScript environment.
///
/// # Usage
///
/// ```rust
/// use angelscript_rs::{Function, FunctionType};
///
/// let function = module.get_function_by_name("myFunction")?;
///
/// match function.get_func_type() {
///     FunctionType::Script => println!("Script function"),
///     FunctionType::System => println!("Registered system function"),
///     FunctionType::Interface => println!("Interface method"),
///     FunctionType::Delegate => println!("Delegate function"),
///     _ => println!("Other function type"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum FunctionType {
    /// Dummy function (placeholder).
    Dummy = asEFuncType_asFUNC_DUMMY,
    /// System function (registered from application).
    System = asEFuncType_asFUNC_SYSTEM,
    /// Script function (written in AngelScript).
    Script = asEFuncType_asFUNC_SCRIPT,
    /// Interface method.
    Interface = asEFuncType_asFUNC_INTERFACE,
    /// Virtual method.
    Virtual = asEFuncType_asFUNC_VIRTUAL,
    /// Function definition.
    Funcdef = asEFuncType_asFUNC_FUNCDEF,
    /// Imported function.
    Imported = asEFuncType_asFUNC_IMPORTED,
    /// Delegate function.
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

// Note: BCInstr and BCType enums are included but not documented here
// as they are primarily for internal use and bytecode analysis.
// The existing implementations remain unchanged.

/// AngelScript bytecode instructions.
///
/// These represent the individual operations in compiled AngelScript bytecode.
/// They are primarily used for debugging, analysis, and JIT compilation.
///
/// **Note**: This enum is mainly for internal use and advanced debugging.
/// Most users won't need to work with bytecode instructions directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BCInstr {
    /// Pop pointer from stack.
    PopPtr = asEBCInstr_asBC_PopPtr,
    /// Push global pointer.
    PshGPtr = asEBCInstr_asBC_PshGPtr,
    /// Push constant 4-byte value.
    PshC4 = asEBCInstr_asBC_PshC4,
    /// Push variable 4-byte value.
    PshV4 = asEBCInstr_asBC_PshV4,
    /// Push stack frame.
    PSF = asEBCInstr_asBC_PSF,
    /// Swap pointers.
    SwapPtr = asEBCInstr_asBC_SwapPtr,
    /// Logical NOT.
    NOT = asEBCInstr_asBC_NOT,
    /// Push global 4-byte value.
    PshG4 = asEBCInstr_asBC_PshG4,
    /// Load global, read register 4.
    LdGRdR4 = asEBCInstr_asBC_LdGRdR4,
    /// Function call.
    CALL = asEBCInstr_asBC_CALL,
    /// Return from function.
    RET = asEBCInstr_asBC_RET,
    /// Unconditional jump.
    JMP = asEBCInstr_asBC_JMP,
    /// Jump if zero.
    JZ = asEBCInstr_asBC_JZ,
    /// Jump if not zero.
    JNZ = asEBCInstr_asBC_JNZ,
    /// Jump if sign.
    JS = asEBCInstr_asBC_JS,
    /// Jump if not sign.
    JNS = asEBCInstr_asBC_JNS,
    /// Jump if positive.
    JP = asEBCInstr_asBC_JP,
    /// Jump if not positive.
    JNP = asEBCInstr_asBC_JNP,
    /// Test zero.
    TZ = asEBCInstr_asBC_TZ,
    /// Test not zero.
    TNZ = asEBCInstr_asBC_TNZ,
    /// Test sign.
    TS = asEBCInstr_asBC_TS,
    /// Test not sign.
    TNS = asEBCInstr_asBC_TNS,
    /// Test positive.
    TP = asEBCInstr_asBC_TP,
    /// Test not positive.
    TNP = asEBCInstr_asBC_TNP,
    /// Negate integer.
    NEGi = asEBCInstr_asBC_NEGi,
    /// Negate float.
    NEGf = asEBCInstr_asBC_NEGf,
    /// Negate double.
    NEGd = asEBCInstr_asBC_NEGd,
    /// Increment 16-bit integer.
    INCi16 = asEBCInstr_asBC_INCi16,
    /// Increment 8-bit integer.
    INCi8 = asEBCInstr_asBC_INCi8,
    /// Decrement 16-bit integer.
    DECi16 = asEBCInstr_asBC_DECi16,
    /// Decrement 8-bit integer.
    DECi8 = asEBCInstr_asBC_DECi8,
    /// Increment integer.
    INCi = asEBCInstr_asBC_INCi,
    /// Decrement integer.
    DECi = asEBCInstr_asBC_DECi,
    /// Increment float.
    INCf = asEBCInstr_asBC_INCf,
    /// Decrement float.
    DECf = asEBCInstr_asBC_DECf,
    /// Increment double.
    INCd = asEBCInstr_asBC_INCd,
    /// Decrement double.
    DECd = asEBCInstr_asBC_DECd,
    /// Increment variable integer.
    IncVi = asEBCInstr_asBC_IncVi,
    /// Decrement variable integer.
    DecVi = asEBCInstr_asBC_DecVi,
    /// Bitwise NOT.
    BNOT = asEBCInstr_asBC_BNOT,
    /// Bitwise AND.
    BAND = asEBCInstr_asBC_BAND,
    /// Bitwise OR.
    BOR = asEBCInstr_asBC_BOR,
    /// Bitwise XOR.
    BXOR = asEBCInstr_asBC_BXOR,
    /// Bitwise shift left logical.
    BSLL = asEBCInstr_asBC_BSLL,
    /// Bitwise shift right logical.
    BSRL = asEBCInstr_asBC_BSRL,
    /// Bitwise shift right arithmetic.
    BSRA = asEBCInstr_asBC_BSRA,
    /// Copy memory.
    COPY = asEBCInstr_asBC_COPY,
    /// Push constant 8-byte value.
    PshC8 = asEBCInstr_asBC_PshC8,
    /// Push variable pointer.
    PshVPtr = asEBCInstr_asBC_PshVPtr,
    /// Read dereference stack pointer.
    RDSPtr = asEBCInstr_asBC_RDSPtr,
    /// Compare double.
    CMPd = asEBCInstr_asBC_CMPd,
    /// Compare unsigned.
    CMPu = asEBCInstr_asBC_CMPu,
    /// Compare float.
    CMPf = asEBCInstr_asBC_CMPf,
    /// Compare integer.
    CMPi = asEBCInstr_asBC_CMPi,
    /// Compare immediate integer.
    CMPIi = asEBCInstr_asBC_CMPIi,
    /// Compare immediate float.
    CMPIf = asEBCInstr_asBC_CMPIf,
    /// Compare immediate unsigned.
    CMPIu = asEBCInstr_asBC_CMPIu,
    /// Jump to pointer.
    JMPP = asEBCInstr_asBC_JMPP,
    /// Pop reference pointer.
    PopRPtr = asEBCInstr_asBC_PopRPtr,
    /// Push reference pointer.
    PshRPtr = asEBCInstr_asBC_PshRPtr,
    /// String operation.
    STR = asEBCInstr_asBC_STR,
    /// Call system function.
    CALLSYS = asEBCInstr_asBC_CALLSYS,
    /// Call bound function.
    CALLBND = asEBCInstr_asBC_CALLBND,
    /// Suspend execution.
    SUSPEND = asEBCInstr_asBC_SUSPEND,
    /// Allocate memory.
    ALLOC = asEBCInstr_asBC_ALLOC,
    /// Free memory.
    FREE = asEBCInstr_asBC_FREE,
    /// Load object.
    LOADOBJ = asEBCInstr_asBC_LOADOBJ,
    /// Store object.
    STOREOBJ = asEBCInstr_asBC_STOREOBJ,
    /// Get object.
    GETOBJ = asEBCInstr_asBC_GETOBJ,
    /// Reference copy.
    REFCPY = asEBCInstr_asBC_REFCPY,
    /// Check reference.
    CHKREF = asEBCInstr_asBC_CHKREF,
    /// Get object reference.
    GETOBJREF = asEBCInstr_asBC_GETOBJREF,
    /// Get reference.
    GETREF = asEBCInstr_asBC_GETREF,
    /// Push null.
    PshNull = asEBCInstr_asBC_PshNull,
    /// Clear variable pointer.
    ClrVPtr = asEBCInstr_asBC_ClrVPtr,
    /// Object type.
    OBJTYPE = asEBCInstr_asBC_OBJTYPE,
    /// Type ID.
    TYPEID = asEBCInstr_asBC_TYPEID,
    /// Set variable 4-byte.
    SetV4 = asEBCInstr_asBC_SetV4,
    /// Set variable 8-byte.
    SetV8 = asEBCInstr_asBC_SetV8,
    /// Add stack integer.
    ADDSi = asEBCInstr_asBC_ADDSi,
    /// Copy variable to variable 4-byte.
    CpyVtoV4 = asEBCInstr_asBC_CpyVtoV4,
    /// Copy variable to variable 8-byte.
    CpyVtoV8 = asEBCInstr_asBC_CpyVtoV8,
    /// Copy variable to register 4-byte.
    CpyVtoR4 = asEBCInstr_asBC_CpyVtoR4,
    /// Copy variable to register 8-byte.
    CpyVtoR8 = asEBCInstr_asBC_CpyVtoR8,
    /// Copy variable to global 4-byte.
    CpyVtoG4 = asEBCInstr_asBC_CpyVtoG4,
    /// Copy register to variable 4-byte.
    CpyRtoV4 = asEBCInstr_asBC_CpyRtoV4,
    /// Copy register to variable 8-byte.
    CpyRtoV8 = asEBCInstr_asBC_CpyRtoV8,
    /// Copy global to variable 4-byte.
    CpyGtoV4 = asEBCInstr_asBC_CpyGtoV4,
    /// Write variable 1-byte.
    WRTV1 = asEBCInstr_asBC_WRTV1,
    /// Write variable 2-byte.
    WRTV2 = asEBCInstr_asBC_WRTV2,
    /// Write variable 4-byte.
    WRTV4 = asEBCInstr_asBC_WRTV4,
    /// Write variable 8-byte.
    WRTV8 = asEBCInstr_asBC_WRTV8,
    /// Read register 1-byte.
    RDR1 = asEBCInstr_asBC_RDR1,
    /// Read register 2-byte.
    RDR2 = asEBCInstr_asBC_RDR2,
    /// Read register 4-byte.
    RDR4 = asEBCInstr_asBC_RDR4,
    /// Read register 8-byte.
    RDR8 = asEBCInstr_asBC_RDR8,
    /// Load global.
    LDG = asEBCInstr_asBC_LDG,
    /// Load variable.
    LDV = asEBCInstr_asBC_LDV,
    /// Push global address.
    PGA = asEBCInstr_asBC_PGA,
    /// Compare pointer.
    CmpPtr = asEBCInstr_asBC_CmpPtr,
    /// Variable.
    VAR = asEBCInstr_asBC_VAR,
    /// Integer to float.
    Itof = asEBCInstr_asBC_iTOf,
    /// Float to integer.
    Ftoi = asEBCInstr_asBC_fTOi,
    /// Unsigned to float.
    Utof = asEBCInstr_asBC_uTOf,
    /// Float to unsigned.
    Ftou = asEBCInstr_asBC_fTOu,
    /// Signed byte to integer.
    SbToi = asEBCInstr_asBC_sbTOi,
    /// Signed word to integer.
    SwToi = asEBCInstr_asBC_swTOi,
    /// Unsigned byte to integer.
    UbToi = asEBCInstr_asBC_ubTOi,
    /// Unsigned word to integer.
    UwToi = asEBCInstr_asBC_uwTOi,
    /// Double to integer.
    Dtoi = asEBCInstr_asBC_dTOi,
    /// Double to unsigned.
    Dtou = asEBCInstr_asBC_dTOu,
    /// Double to float.
    Dtof = asEBCInstr_asBC_dTOf,
    /// Integer to double.
    Itod = asEBCInstr_asBC_iTOd,
    /// Unsigned to double.
    Utod = asEBCInstr_asBC_uTOd,
    /// Float to double.
    Ftod = asEBCInstr_asBC_fTOd,
    /// Add integer.
    ADDi = asEBCInstr_asBC_ADDi,
    /// Subtract integer.
    SUBi = asEBCInstr_asBC_SUBi,
    /// Multiply integer.
    MULi = asEBCInstr_asBC_MULi,
    /// Divide integer.
    DIVi = asEBCInstr_asBC_DIVi,
    /// Modulo integer.
    MODi = asEBCInstr_asBC_MODi,
    /// Add float.
    ADDf = asEBCInstr_asBC_ADDf,
    /// Subtract float.
    SUBf = asEBCInstr_asBC_SUBf,
    /// Multiply float.
    MULf = asEBCInstr_asBC_MULf,
    /// Divide float.
    DIVf = asEBCInstr_asBC_DIVf,
    /// Modulo float.
    MODf = asEBCInstr_asBC_MODf,
    /// Add double.
    ADDd = asEBCInstr_asBC_ADDd,
    /// Subtract double.
    SUBd = asEBCInstr_asBC_SUBd,
    /// Multiply double.
    MULd = asEBCInstr_asBC_MULd,
    /// Divide double.
    DIVd = asEBCInstr_asBC_DIVd,
    /// Modulo double.
    MODd = asEBCInstr_asBC_MODd,
    /// Add immediate integer.
    ADDIi = asEBCInstr_asBC_ADDIi,
    /// Subtract immediate integer.
    SUBIi = asEBCInstr_asBC_SUBIi,
    /// Multiply immediate integer.
    MULIi = asEBCInstr_asBC_MULIi,
    /// Add immediate float.
    ADDIf = asEBCInstr_asBC_ADDIf,
    /// Subtract immediate float.
    SUBIf = asEBCInstr_asBC_SUBIf,
    /// Multiply immediate float.
    MULIf = asEBCInstr_asBC_MULIf,
    /// Set global 4-byte.
    SetG4 = asEBCInstr_asBC_SetG4,
    /// Check reference stack.
    ChkRefS = asEBCInstr_asBC_ChkRefS,
    /// Check null variable.
    ChkNullV = asEBCInstr_asBC_ChkNullV,
    /// Call interface.
    CALLINTF = asEBCInstr_asBC_CALLINTF,
    /// Integer to byte.
    Itob = asEBCInstr_asBC_iTOb,
    /// Integer to word.
    Itow = asEBCInstr_asBC_iTOw,
    /// Set variable 1-byte.
    SetV1 = asEBCInstr_asBC_SetV1,
    /// Set variable 2-byte.
    SetV2 = asEBCInstr_asBC_SetV2,
    /// Cast operation.
    Cast = asEBCInstr_asBC_Cast,
    /// 64-bit integer to integer.
    I64toi = asEBCInstr_asBC_i64TOi,
    /// Unsigned to 64-bit integer.
    Utoi64 = asEBCInstr_asBC_uTOi64,
    /// Integer to 64-bit integer.
    Itoi64 = asEBCInstr_asBC_iTOi64,
    /// Float to 64-bit integer.
    Ftoi64 = asEBCInstr_asBC_fTOi64,
    /// Double to 64-bit integer.
    Dtoi64 = asEBCInstr_asBC_dTOi64,
    /// Float to 64-bit unsigned.
    Ftou64 = asEBCInstr_asBC_fTOu64,
    /// Double to 64-bit unsigned.
    Dtou64 = asEBCInstr_asBC_dTOu64,
    /// 64-bit integer to float.
    I64tof = asEBCInstr_asBC_i64TOf,
    /// 64-bit unsigned to float.
    U64tof = asEBCInstr_asBC_u64TOf,
    /// 64-bit integer to double.
    I64tod = asEBCInstr_asBC_i64TOd,
    /// 64-bit unsigned to double.
    U64tod = asEBCInstr_asBC_u64TOd,
    /// Negate 64-bit integer.
    NEGi64 = asEBCInstr_asBC_NEGi64,
    /// Increment 64-bit integer.
    INCi64 = asEBCInstr_asBC_INCi64,
    /// Decrement 64-bit integer.
    DECi64 = asEBCInstr_asBC_DECi64,
    /// Bitwise NOT 64-bit.
    BNOT64 = asEBCInstr_asBC_BNOT64,
    /// Add 64-bit integer.
    ADDi64 = asEBCInstr_asBC_ADDi64,
    /// Subtract 64-bit integer.
    SUBi64 = asEBCInstr_asBC_SUBi64,
    /// Multiply 64-bit integer.
    MULi64 = asEBCInstr_asBC_MULi64,
    /// Divide 64-bit integer.
    DIVi64 = asEBCInstr_asBC_DIVi64,
    /// Modulo 64-bit integer.
    MODi64 = asEBCInstr_asBC_MODi64,
    /// Bitwise AND 64-bit.
    BAND64 = asEBCInstr_asBC_BAND64,
    /// Bitwise OR 64-bit.
    BOR64 = asEBCInstr_asBC_BOR64,
    /// Bitwise XOR 64-bit.
    BXOR64 = asEBCInstr_asBC_BXOR64,
    /// Bitwise shift left logical 64-bit.
    BSLL64 = asEBCInstr_asBC_BSLL64,
    /// Bitwise shift right logical 64-bit.
    BSRL64 = asEBCInstr_asBC_BSRL64,
    /// Bitwise shift right arithmetic 64-bit.
    BSRA64 = asEBCInstr_asBC_BSRA64,
    /// Compare 64-bit integer.
    CMPi64 = asEBCInstr_asBC_CMPi64,
    /// Compare 64-bit unsigned.
    CMPu64 = asEBCInstr_asBC_CMPu64,
    /// Check null stack.
    ChkNullS = asEBCInstr_asBC_ChkNullS,
    /// Clear high bits.
    ClrHi = asEBCInstr_asBC_ClrHi,
    /// JIT entry point.
    JitEntry = asEBCInstr_asBC_JitEntry,
    /// Call pointer.
    CallPtr = asEBCInstr_asBC_CallPtr,
    /// Function pointer.
    FuncPtr = asEBCInstr_asBC_FuncPtr,
    /// Load this register.
    LoadThisR = asEBCInstr_asBC_LoadThisR,
    /// Push variable 8-byte.
    PshV8 = asEBCInstr_asBC_PshV8,
    /// Divide unsigned.
    DIVu = asEBCInstr_asBC_DIVu,
    /// Modulo unsigned.
    MODu = asEBCInstr_asBC_MODu,
    /// Divide 64-bit unsigned.
    DIVu64 = asEBCInstr_asBC_DIVu64,
    /// Modulo 64-bit unsigned.
    MODu64 = asEBCInstr_asBC_MODu64,
    /// Load reference object register.
    LoadRObjR = asEBCInstr_asBC_LoadRObjR,
    /// Load variable object register.
    LoadVObjR = asEBCInstr_asBC_LoadVObjR,
    /// Reference copy variable.
    RefCpyV = asEBCInstr_asBC_RefCpyV,
    /// Jump if low zero.
    JLowZ = asEBCInstr_asBC_JLowZ,
    /// Jump if low not zero.
    JLowNZ = asEBCInstr_asBC_JLowNZ,
    /// Allocate memory.
    AllocMem = asEBCInstr_asBC_AllocMem,
    /// Set list size.
    SetListSize = asEBCInstr_asBC_SetListSize,
    /// Push list element.
    PshListElmnt = asEBCInstr_asBC_PshListElmnt,
    /// Set list type.
    SetListType = asEBCInstr_asBC_SetListType,
    /// Power integer.
    POWi = asEBCInstr_asBC_POWi,
    /// Power unsigned.
    POWu = asEBCInstr_asBC_POWu,
    /// Power float.
    POWf = asEBCInstr_asBC_POWf,
    /// Power double.
    POWd = asEBCInstr_asBC_POWd,
    /// Power double integer.
    POWdi = asEBCInstr_asBC_POWdi,
    /// Power 64-bit integer.
    POWi64 = asEBCInstr_asBC_POWi64,
    /// Power 64-bit unsigned.
    POWu64 = asEBCInstr_asBC_POWu64,
    /// This-call 1.
    Thiscall1 = asEBCInstr_asBC_Thiscall1,
    /// Maximum bytecode.
    MaxByteCode = asEBCInstr_asBC_MAXBYTECODE,
    /// Try block.
    TryBlock = asEBCInstr_asBC_TryBlock,
    /// Variable declaration.
    VarDecl = asEBCInstr_asBC_VarDecl,
    /// Block.
    Block = asEBCInstr_asBC_Block,
    /// Object info.
    ObjInfo = asEBCInstr_asBC_ObjInfo,
    /// Line number.
    Line = asEBCInstr_asBC_LINE,
    /// Label.
    Label = asEBCInstr_asBC_LABEL,
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
            asEBCInstr_asBC_iTOf => Self::Itof,
            asEBCInstr_asBC_fTOi => Self::Ftoi,
            asEBCInstr_asBC_uTOf => Self::Utof,
            asEBCInstr_asBC_fTOu => Self::Ftou,
            asEBCInstr_asBC_sbTOi => Self::SbToi,
            asEBCInstr_asBC_swTOi => Self::SwToi,
            asEBCInstr_asBC_ubTOi => Self::UbToi,
            asEBCInstr_asBC_uwTOi => Self::UwToi,
            asEBCInstr_asBC_dTOi => Self::Dtoi,
            asEBCInstr_asBC_dTOu => Self::Dtou,
            asEBCInstr_asBC_dTOf => Self::Dtof,
            asEBCInstr_asBC_iTOd => Self::Itod,
            asEBCInstr_asBC_uTOd => Self::Utod,
            asEBCInstr_asBC_fTOd => Self::Ftod,
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
            asEBCInstr_asBC_iTOb => Self::Itob,
            asEBCInstr_asBC_iTOw => Self::Itow,
            asEBCInstr_asBC_SetV1 => Self::SetV1,
            asEBCInstr_asBC_SetV2 => Self::SetV2,
            asEBCInstr_asBC_Cast => Self::Cast,
            asEBCInstr_asBC_i64TOi => Self::I64toi,
            asEBCInstr_asBC_uTOi64 => Self::Utoi64,
            asEBCInstr_asBC_iTOi64 => Self::Itoi64,
            asEBCInstr_asBC_fTOi64 => Self::Ftoi64,
            asEBCInstr_asBC_dTOi64 => Self::Dtoi64,
            asEBCInstr_asBC_fTOu64 => Self::Ftou64,
            asEBCInstr_asBC_dTOu64 => Self::Dtou64,
            asEBCInstr_asBC_i64TOf => Self::I64tof,
            asEBCInstr_asBC_u64TOf => Self::U64tof,
            asEBCInstr_asBC_i64TOd => Self::I64tod,
            asEBCInstr_asBC_u64TOd => Self::U64tod,
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
            asEBCInstr_asBC_MAXBYTECODE => Self::MaxByteCode,
            asEBCInstr_asBC_TryBlock => Self::TryBlock,
            asEBCInstr_asBC_VarDecl => Self::VarDecl,
            asEBCInstr_asBC_Block => Self::Block,
            asEBCInstr_asBC_ObjInfo => Self::ObjInfo,
            asEBCInstr_asBC_LINE => Self::Line,
            asEBCInstr_asBC_LABEL => Self::Label,
            _ => panic!("Unknown bytecode instruction: {}", value),
        }
    }
}

impl From<BCInstr> for asEBCInstr {
    fn from(value: BCInstr) -> Self {
        value as asEBCInstr
    }
}

/// Bytecode instruction argument types.
///
/// These specify the format of arguments that bytecode instructions expect.
/// Used primarily for bytecode analysis and debugging tools.
///
/// **Note**: This enum is mainly for internal use and bytecode analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BCType {
    /// Information type.
    Info = asEBCType_asBCTYPE_INFO,
    /// No argument.
    NoArg = asEBCType_asBCTYPE_NO_ARG,
    /// Word argument.
    WArg = asEBCType_asBCTYPE_W_ARG,
    /// Word-word argument.
    WwArg = asEBCType_asBCTYPE_wW_ARG,
    /// Double word argument.
    DwArg = asEBCType_asBCTYPE_DW_ARG,
    /// Read word, double word argument.
    RwDwArg = asEBCType_asBCTYPE_rW_DW_ARG,
    /// Quad word argument.
    QwArg = asEBCType_asBCTYPE_QW_ARG,
    /// Double word, double word argument.
    DwDwArg = asEBCType_asBCTYPE_DW_DW_ARG,
    /// Word-word, read word, read word argument.
    WwRwRwArg = asEBCType_asBCTYPE_wW_rW_rW_ARG,
    /// Word-word, quad word argument.
    WwQwArg = asEBCType_asBCTYPE_wW_QW_ARG,
    /// Word-word, read word argument.
    WwRwArg = asEBCType_asBCTYPE_wW_rW_ARG,
    /// Read word argument.
    RwArg = asEBCType_asBCTYPE_rW_ARG,
    /// Word-word, double word argument.
    WwDwArg = asEBCType_asBCTYPE_wW_DW_ARG,
    /// Word-word, read word, double word argument.
    WwRwDwArg = asEBCType_asBCTYPE_wW_rW_DW_ARG,
    /// Read word, read word argument.
    RwRwArg = asEBCType_asBCTYPE_rW_rW_ARG,
    /// Word-word, word argument.
    WwWArg = asEBCType_asBCTYPE_wW_W_ARG,
    /// Quad word, double word argument.
    QwDwArg = asEBCType_asBCTYPE_QW_DW_ARG,
    /// Read word, quad word argument.
    RwQwArg = asEBCType_asBCTYPE_rW_QW_ARG,
    /// Word, double word argument.
    WDwArg = asEBCType_asBCTYPE_W_DW_ARG,
    /// Read word, word, double word argument.
    RwWDwArg = asEBCType_asBCTYPE_rW_W_DW_ARG,
    /// Read word, double word, double word argument.
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
