use std::ffi::NulError;
use std::fmt;
use std::fmt::{Debug, Display};
use std::str::Utf8Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    // AngelScript errors
    InvalidArg,
    NoFunction,
    NotSupported,
    InvalidName,
    NameTaken,
    InvalidDeclaration,
    InvalidObject,
    InvalidType,
    AlreadyRegistered,
    MultipleFunctions,
    NoModule,
    NoSection,
    NoConfigGroup,
    NoGlobalVar,
    InvalidConfiguration,
    InvalidInterface,
    CantBindAllFunctions,
    LowerArrayDimensionNotRegistered,
    WrongConfigGroup,
    ConfigGroupIsInUse,
    IllegalBehaviourForType,
    WrongCallingConv,
    BuildInProgress,
    InitGlobalVarsFailed,
    OutOfMemory,
    ModuleIsInUse,
    ContextActive,
    ContextNotFinished,
    ContextNotPrepared,

    // Rust-specific errors
    NullPointer,
    StringConversion(NulError),
    Utf8Conversion(Utf8Error),
    External(Box<dyn Debug>),
    Unknown(i32),
}

impl Error {
    pub fn from_code(code: i32) -> Result<()> {
        use crate::ffi::asERetCodes;

        if code >= 0 {
            return Ok(());
        }

        match unsafe { std::mem::transmute::<i32, asERetCodes>(code) } {
            asERetCodes::asCONTEXT_ACTIVE => Err(Error::ContextActive),
            asERetCodes::asCONTEXT_NOT_FINISHED => Err(Error::ContextNotFinished),
            asERetCodes::asCONTEXT_NOT_PREPARED => Err(Error::ContextNotPrepared),
            asERetCodes::asINVALID_ARG => Err(Error::InvalidArg),
            asERetCodes::asNO_FUNCTION => Err(Error::NoFunction),
            asERetCodes::asNOT_SUPPORTED => Err(Error::NotSupported),
            asERetCodes::asINVALID_NAME => Err(Error::InvalidName),
            asERetCodes::asNAME_TAKEN => Err(Error::NameTaken),
            asERetCodes::asINVALID_DECLARATION => Err(Error::InvalidDeclaration),
            asERetCodes::asINVALID_OBJECT => Err(Error::InvalidObject),
            asERetCodes::asINVALID_TYPE => Err(Error::InvalidType),
            asERetCodes::asALREADY_REGISTERED => Err(Error::AlreadyRegistered),
            asERetCodes::asMULTIPLE_FUNCTIONS => Err(Error::MultipleFunctions),
            asERetCodes::asNO_MODULE => Err(Error::NoModule),
            asERetCodes::asNO_GLOBAL_VAR => Err(Error::NoGlobalVar),
            asERetCodes::asINVALID_CONFIGURATION => Err(Error::InvalidConfiguration),
            asERetCodes::asINVALID_INTERFACE => Err(Error::InvalidInterface),
            asERetCodes::asCANT_BIND_ALL_FUNCTIONS => Err(Error::CantBindAllFunctions),
            asERetCodes::asLOWER_ARRAY_DIMENSION_NOT_REGISTERED => {
                Err(Error::LowerArrayDimensionNotRegistered)
            }
            asERetCodes::asWRONG_CONFIG_GROUP => Err(Error::WrongConfigGroup),
            asERetCodes::asCONFIG_GROUP_IS_IN_USE => Err(Error::ConfigGroupIsInUse),
            asERetCodes::asILLEGAL_BEHAVIOUR_FOR_TYPE => Err(Error::IllegalBehaviourForType),
            asERetCodes::asWRONG_CALLING_CONV => Err(Error::WrongCallingConv),
            asERetCodes::asBUILD_IN_PROGRESS => Err(Error::BuildInProgress),
            asERetCodes::asINIT_GLOBAL_VARS_FAILED => Err(Error::InitGlobalVarsFailed),
            asERetCodes::asOUT_OF_MEMORY => Err(Error::OutOfMemory),
            asERetCodes::asMODULE_IS_IN_USE => Err(Error::ModuleIsInUse),
            other => Err(Error::Unknown(other as i32)),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidArg => write!(f, "Invalid argument"),
            Error::NoFunction => write!(f, "No function"),
            Error::NotSupported => write!(f, "Not supported"),
            Error::InvalidName => write!(f, "Invalid name"),
            Error::NameTaken => write!(f, "Name taken"),
            Error::InvalidDeclaration => write!(f, "Invalid declaration"),
            Error::InvalidObject => write!(f, "Invalid object"),
            Error::InvalidType => write!(f, "Invalid type"),
            Error::AlreadyRegistered => write!(f, "Already registered"),
            Error::MultipleFunctions => write!(f, "Multiple functions"),
            Error::NoModule => write!(f, "No module"),
            Error::NoSection => write!(f, "No section"),
            Error::NoConfigGroup => write!(f, "No config group"),
            Error::NoGlobalVar => write!(f, "No global variable"),
            Error::InvalidConfiguration => write!(f, "Invalid configuration"),
            Error::InvalidInterface => write!(f, "Invalid interface"),
            Error::CantBindAllFunctions => write!(f, "Can't bind all functions"),
            Error::LowerArrayDimensionNotRegistered => {
                write!(f, "Lower array dimension not registered")
            }
            Error::WrongConfigGroup => write!(f, "Wrong config group"),
            Error::ConfigGroupIsInUse => write!(f, "Config group is in use"),
            Error::IllegalBehaviourForType => write!(f, "Illegal behaviour for type"),
            Error::WrongCallingConv => write!(f, "Wrong calling convention"),
            Error::BuildInProgress => write!(f, "Build in progress"),
            Error::InitGlobalVarsFailed => write!(f, "Init global vars failed"),
            Error::OutOfMemory => write!(f, "Out of memory"),
            Error::ModuleIsInUse => write!(f, "Module is in use"),
            Error::ContextActive => write!(f, "Context active"),
            Error::ContextNotFinished => write!(f, "Context not finished"),
            Error::ContextNotPrepared => write!(f, "Context not prepared"),
            Error::NullPointer => write!(f, "Null pointer"),
            Error::StringConversion(e) => write!(f, "String conversion error: {}", e),
            Error::Utf8Conversion(e) => write!(f, "Utf8 conversion error: {}", e),
            Error::Unknown(code) => write!(f, "Unknown error code: {}", code),
            Error::External(display) => write!(f, "External error: {:?}", display.as_ref()),
        }
    }
}

impl std::error::Error for Error {}

impl From<NulError> for Error {
    fn from(err: NulError) -> Self {
        Error::StringConversion(err)
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Error::Utf8Conversion(err)
    }
}
