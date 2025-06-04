use std::ffi::NulError;
use std::fmt;
use std::fmt::Debug;
use std::str::Utf8Error;
use std::sync::{MutexGuard, PoisonError};

use crate::enums::ReturnCode;
use crate::ffi::asERetCodes;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    // AngelScript errors - now using the enum directly
    AngelScript(ReturnCode),

    // Rust-specific errors
    NullPointer,
    StringConversion(NulError),
    Utf8Conversion(Utf8Error),
    External(Box<dyn Debug>),
    Unknown(i32),
    FailedToCreateEngine,
    MutexPoisoned,
}

impl Error {
    pub fn from_code(code: i32) -> Result<()> {
        if code >= 0 {
            return Ok(());
        }

        // Convert the raw FFI code to our ReturnCode enum
        let return_code = ReturnCode::from(code);

        // Only return error for actual error codes (negative values)
        match return_code {
            ReturnCode::Success => Ok(()),
            error_code => Err(Error::AngelScript(error_code)),
        }
    }

    /// Convenience method to check if this is a specific AngelScript error
    pub fn is_angelscript_error(&self, code: ReturnCode) -> bool {
        matches!(self, Error::AngelScript(err_code) if *err_code == code)
    }

    /// Get the underlying ReturnCode if this is an AngelScript error
    pub fn as_angelscript_error(&self) -> Option<ReturnCode> {
        match self {
            Error::AngelScript(code) => Some(*code),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::AngelScript(code) => {
                match code {
                    ReturnCode::Error => write!(f, "AngelScript error"),
                    ReturnCode::ContextActive => write!(f, "Context is active"),
                    ReturnCode::ContextNotFinished => write!(f, "Context not finished"),
                    ReturnCode::ContextNotPrepared => write!(f, "Context not prepared"),
                    ReturnCode::InvalidArg => write!(f, "Invalid argument"),
                    ReturnCode::NoFunction => write!(f, "No function found"),
                    ReturnCode::NotSupported => write!(f, "Operation not supported"),
                    ReturnCode::InvalidName => write!(f, "Invalid name"),
                    ReturnCode::NameTaken => write!(f, "Name already taken"),
                    ReturnCode::InvalidDeclaration => write!(f, "Invalid declaration"),
                    ReturnCode::InvalidObject => write!(f, "Invalid object"),
                    ReturnCode::InvalidType => write!(f, "Invalid type"),
                    ReturnCode::AlreadyRegistered => write!(f, "Already registered"),
                    ReturnCode::MultipleFunctions => write!(f, "Multiple functions found"),
                    ReturnCode::NoModule => write!(f, "No module found"),
                    ReturnCode::NoGlobalVar => write!(f, "No global variable found"),
                    ReturnCode::InvalidConfiguration => write!(f, "Invalid configuration"),
                    ReturnCode::InvalidInterface => write!(f, "Invalid interface"),
                    ReturnCode::CantBindAllFunctions => write!(f, "Cannot bind all functions"),
                    ReturnCode::LowerArrayDimensionNotRegistered => {
                        write!(f, "Lower array dimension not registered")
                    }
                    ReturnCode::WrongConfigGroup => write!(f, "Wrong configuration group"),
                    ReturnCode::ConfigGroupIsInUse => write!(f, "Configuration group is in use"),
                    ReturnCode::IllegalBehaviourForType => write!(f, "Illegal behaviour for type"),
                    ReturnCode::WrongCallingConv => write!(f, "Wrong calling convention"),
                    ReturnCode::BuildInProgress => write!(f, "Build in progress"),
                    ReturnCode::InitGlobalVarsFailed => write!(f, "Failed to initialize global variables"),
                    ReturnCode::OutOfMemory => write!(f, "Out of memory"),
                    ReturnCode::ModuleIsInUse => write!(f, "Module is in use"),
                    ReturnCode::Success => write!(f, "Success (this shouldn't be an error)"),
                }
            }
            Error::NullPointer => write!(f, "Null pointer encountered"),
            Error::StringConversion(e) => write!(f, "String conversion error: {}", e),
            Error::Utf8Conversion(e) => write!(f, "UTF-8 conversion error: {}", e),
            Error::Unknown(code) => write!(f, "Unknown error code: {}", code),
            Error::External(display) => write!(f, "External error: {:?}", display.as_ref()),
            Error::FailedToCreateEngine => write!(f, "Failed to create AngelScript engine"),
            Error::MutexPoisoned => write!(f, "Mutex poisoned"),
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

impl<'a, T> From<PoisonError<MutexGuard<'a, T>>> for Error {
    fn from(_: PoisonError<MutexGuard<'a, T>>) -> Self {
        Error::MutexPoisoned
    }
}

// Convenience conversion from ReturnCode to Error
impl From<ReturnCode> for Error {
    fn from(code: ReturnCode) -> Self {
        Error::AngelScript(code)
    }
}

// Convenience conversion from raw FFI return codes
impl From<asERetCodes> for Error {
    fn from(code: asERetCodes) -> Self {
        Error::AngelScript(ReturnCode::from(code))
    }
}
