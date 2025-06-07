use crate::types::enums::ReturnCode;
use std::ffi::NulError;
use std::str::Utf8Error;
use std::sync::{MutexGuard, PoisonError};
use thiserror::Error;

pub type ScriptResult<T> = anyhow::Result<T, ScriptError>;

#[derive(Error, Debug)]
pub enum ScriptError {
    #[error("AngelScript error: {0:?}")]
    AngelScriptError(ReturnCode),

    #[error("Null pointer encountered")]
    NullPointer,

    #[error("String conversion error: {0}")]
    StringConversion(#[from] NulError),

    #[error("UTF-8 conversion error: {0}")]
    Utf8Conversion(#[from] Utf8Error),

    #[error("External error: {0}")]
    External(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("ScriptGeneric error: {0}")]
    Generic(String),

    #[error("Unknown error code: {0}")]
    Unknown(i32),

    #[error("Failed to create AngelScript engine")]
    FailedToCreateEngine,

    #[error("Mutex poisoned")]
    MutexPoisoned,
}

impl ScriptError {
    pub fn from_code(code: i32) -> ScriptResult<()> {
        if code >= 0 {
            return Ok(());
        }

        let return_code = ReturnCode::from(code);

        match return_code {
            ReturnCode::Success => Ok(()),
            error_code => Err(ScriptError::AngelScriptError(error_code)),
        }
    }
}

impl<T> From<PoisonError<MutexGuard<'_, T>>> for ScriptError {
    fn from(_: PoisonError<MutexGuard<'_, T>>) -> Self {
        ScriptError::MutexPoisoned
    }
}
