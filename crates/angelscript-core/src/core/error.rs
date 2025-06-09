use crate::types::enums::ReturnCode;
use std::ffi::NulError;
use std::str::Utf8Error;
use std::sync::{MutexGuard, PoisonError};
use thiserror::Error;

/// A specialized Result type for AngelScript operations.
///
/// This type alias provides a convenient way to handle AngelScript-specific errors
/// throughout the crate. It uses `anyhow::Result` for flexible error handling while
/// maintaining type safety with `ScriptError`.
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::{Engine, ScriptResult};
///
/// fn create_engine() -> ScriptResult<Engine> {
///     Engine::create()
/// }
///
/// match create_engine() {
///     Ok(engine) => println!("Engine created successfully"),
///     Err(e) => eprintln!("Failed to create engine: {}", e),
/// }
/// ```
pub type ScriptResult<T> = anyhow::Result<T, ScriptError>;

/// Comprehensive error type for AngelScript operations.
///
/// This enum covers all possible error conditions that can occur when working with
/// AngelScript, from low-level AngelScript errors to Rust-specific issues like
/// string conversion failures.
///
/// The error types are designed to provide clear, actionable error messages while
/// maintaining compatibility with Rust's error handling ecosystem through the
/// `thiserror` crate.
///
/// # Error Categories
///
/// - **AngelScript Errors**: Direct errors from the AngelScript engine
/// - **Pointer Errors**: Null pointer dereferences
/// - **Conversion Errors**: String and encoding conversion failures
/// - **Threading Errors**: Mutex poisoning and synchronization issues
/// - **Generic Errors**: Custom error messages for specific scenarios
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::{ScriptError, ReturnCode};
///
/// // Handle specific error types
/// match some_operation() {
///     Err(ScriptError::AngelScriptError(ReturnCode::InvalidConfiguration)) => {
///         eprintln!("Invalid engine configuration");
///     }
///     Err(ScriptError::NullPointer) => {
///         eprintln!("Unexpected null pointer");
///     }
///     Err(ScriptError::StringConversion(e)) => {
///         eprintln!("String conversion failed: {}", e);
///     }
///     Err(e) => {
///         eprintln!("Other error: {}", e);
///     }
///     Ok(result) => {
///         // Handle success
///     }
/// }
/// ```
#[derive(Error, Debug)]
pub enum ScriptError {
    /// An error returned by the AngelScript engine.
    ///
    /// This variant wraps AngelScript's native error codes, providing detailed
    /// information about what went wrong during script compilation, execution,
    /// or engine operations.
    ///
    /// # Common AngelScript Errors
    ///
    /// - `InvalidConfiguration`: Engine configuration is invalid
    /// - `InvalidName`: Invalid identifier or name
    /// - `NameTaken`: Name is already in use
    /// - `InvalidDeclaration`: Syntax error in declaration
    /// - `InvalidObject`: Invalid object reference
    /// - `InvalidTypeId`: Invalid type identifier
    /// - `AlreadyRegistered`: Item is already registered
    /// - `MultipleMatches`: Multiple matches found (ambiguous)
    /// - `NoModule`: No module available
    /// - `NoFunction`: Function not found
    /// - `NotSupported`: Operation not supported
    ///
    /// # Examples
    ///
    /// ```rust
    /// use angelscript_rs::{Engine, ScriptError, ReturnCode};
    ///
    /// let engine = Engine::create()?;
    ///
    /// // This will fail with InvalidDeclaration
    /// match engine.register_global_function("invalid syntax", my_func, None) {
    ///     Err(ScriptError::AngelScriptError(ReturnCode::InvalidDeclaration)) => {
    ///         println!("Function declaration has invalid syntax");
    ///     }
    ///     _ => {}
    /// }
    /// ```
    #[error("AngelScript error: {0:?}")]
    AngelScriptError(ReturnCode),

    /// A null pointer was encountered when a valid pointer was expected.
    ///
    /// This error occurs when AngelScript returns a null pointer for operations
    /// that should return valid objects, or when internal pointer validation fails.
    /// This typically indicates a serious error in the AngelScript engine state
    /// or incorrect usage of the API.
    ///
    /// # Common Causes
    ///
    /// - Attempting to use an object after it has been destroyed
    /// - Engine or context creation failure
    /// - Invalid function or module references
    /// - Memory allocation failures
    ///
    /// # Examples
    ///
    /// ```rust
    /// use angelscript_rs::{Engine, ScriptError};
    ///
    /// let engine = Engine::create()?;
    /// let module = engine.get_module("NonExistent", GetModuleFlags::OnlyIfExists);
    ///
    /// match module {
    ///     Err(ScriptError::NullPointer) => {
    ///         println!("Module does not exist");
    ///     }
    ///     _ => {}
    /// }
    /// ```
    #[error("Null pointer encountered")]
    NullPointer,

    /// String conversion to C-compatible format failed.
    ///
    /// This error occurs when converting Rust strings to C-style null-terminated
    /// strings fails, typically because the string contains null bytes which are
    /// not allowed in C strings.
    ///
    /// # Common Causes
    ///
    /// - Strings containing null bytes (`\0`)
    /// - Invalid UTF-8 sequences in string data
    ///
    /// # Examples
    ///
    /// ```rust
    /// use angelscript_rs::{Engine, ScriptError};
    ///
    /// let engine = Engine::create()?;
    ///
    /// // This will fail because of the null byte
    /// let invalid_name = "function\0name";
    /// match engine.register_global_function(invalid_name, my_func, None) {
    ///     Err(ScriptError::StringConversion(_)) => {
    ///         println!("Function name contains invalid characters");
    ///     }
    ///     _ => {}
    /// }
    /// ```
    #[error("String conversion error: {0}")]
    StringConversion(#[from] NulError),

    /// UTF-8 string conversion failed.
    ///
    /// This error occurs when converting C strings returned by AngelScript
    /// back to Rust strings fails due to invalid UTF-8 encoding.
    ///
    /// # Common Causes
    ///
    /// - Non-UTF-8 encoded strings from AngelScript
    /// - Corrupted string data
    /// - Platform-specific encoding issues
    ///
    /// # Examples
    ///
    /// ```rust
    /// use angelscript_rs::{Context, ScriptError};
    ///
    /// let context = engine.create_context()?;
    ///
    /// match context.get_exception_string() {
    ///     Some(msg) => println!("Exception: {}", msg),
    ///     None => {
    ///         // This could fail with Utf8Conversion if the string is invalid
    ///         println!("No exception or invalid UTF-8 in exception message");
    ///     }
    /// }
    /// ```
    #[error("UTF-8 conversion error: {0}")]
    Utf8Conversion(#[from] Utf8Error),

    /// An external error from another library or system component.
    ///
    /// This variant allows wrapping errors from other libraries or system
    /// calls that may occur during AngelScript operations. The error is
    /// boxed to allow for any error type that implements the standard
    /// error traits.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use angelscript_rs::ScriptError;
    /// use std::fs;
    ///
    /// fn load_script_file(path: &str) -> Result<String, ScriptError> {
    ///     // File I/O errors will be automatically converted
    ///     let content = fs::read_to_string(path)
    ///         .map_err(|e| ScriptError::External(Box::new(e)))?;
    ///     Ok(content)
    /// }
    /// ```
    #[error("External error: {0}")]
    External(#[from] Box<dyn std::error::Error + Send + Sync>),

    /// A generic error with a custom message.
    ///
    /// This variant is used for application-specific errors or situations
    /// that don't fit into the other error categories. It provides a flexible
    /// way to report custom error conditions with descriptive messages.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use angelscript_rs::ScriptError;
    ///
    /// fn validate_script_name(name: &str) -> Result<(), ScriptError> {
    ///     if name.is_empty() {
    ///         return Err(ScriptError::Generic(
    ///             "Script name cannot be empty".to_string()
    ///         ));
    ///     }
    ///     if name.len() > 64 {
    ///         return Err(ScriptError::Generic(
    ///             "Script name too long (max 64 characters)".to_string()
    ///         ));
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[error("ScriptGeneric error: {0}")]
    Generic(String),

    /// An unknown error code was returned by AngelScript.
    ///
    /// This error occurs when AngelScript returns an error code that is not
    /// recognized by the current version of the bindings. This may happen
    /// when using newer versions of AngelScript with older bindings.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use angelscript_rs::ScriptError;
    ///
    /// match some_operation() {
    ///     Err(ScriptError::Unknown(code)) => {
    ///         eprintln!("Unknown error code: {}. Please update angelscript-rs.", code);
    ///     }
    ///     _ => {}
    /// }
    /// ```
    #[error("Unknown error code: {0}")]
    Unknown(i32),

    /// Failed to create the AngelScript engine.
    ///
    /// This error occurs when the AngelScript engine creation fails, typically
    /// due to memory allocation issues or incompatible AngelScript library
    /// versions.
    ///
    /// # Common Causes
    ///
    /// - Insufficient memory
    /// - Incompatible AngelScript library version
    /// - Platform-specific initialization issues
    /// - Multiple engine creation attempts without proper cleanup
    ///
    /// # Examples
    ///
    /// ```rust
    /// use angelscript_rs::{Engine, ScriptError};
    ///
    /// match Engine::create() {
    ///     Err(ScriptError::FailedToCreateEngine) => {
    ///         eprintln!("Could not initialize AngelScript engine");
    ///         eprintln!("Check memory availability and library compatibility");
    ///     }
    ///     Ok(engine) => {
    ///         // Engine created successfully
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Other error: {}", e);
    ///     }
    /// }
    /// ```
    #[error("Failed to create AngelScript engine")]
    FailedToCreateEngine,

    /// A mutex was poisoned due to a panic in another thread.
    ///
    /// This error occurs when a thread holding a mutex panics, leaving the
    /// mutex in a poisoned state. This is part of Rust's thread safety
    /// guarantees and indicates that shared data may be in an inconsistent state.
    ///
    /// # Recovery
    ///
    /// In most cases, this error indicates a serious problem in the application
    /// and recovery may not be possible. However, you can sometimes recover by:
    ///
    /// 1. Restarting the affected subsystem
    /// 2. Recreating the poisoned data structures
    /// 3. Using `PoisonError::into_inner()` to access the data anyway (unsafe)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use angelscript_rs::ScriptError;
    /// use std::sync::{Arc, Mutex};
    ///
    /// fn handle_shared_data(data: Arc<Mutex<SomeData>>) -> Result<(), ScriptError> {
    ///     let guard = data.lock()?; // May return MutexPoisoned
    ///     // Use the data...
    ///     Ok(())
    /// }
    ///
    /// match handle_shared_data(shared_data) {
    ///     Err(ScriptError::MutexPoisoned) => {
    ///         eprintln!("Shared data corrupted due to panic in another thread");
    ///         // Consider restarting or recreating the data
    ///     }
    ///     _ => {}
    /// }
    /// ```
    #[error("Mutex poisoned")]
    MutexPoisoned,
}

impl ScriptError {
    /// Converts an AngelScript return code to a Result.
    ///
    /// This function is the primary way to convert AngelScript's integer return
    /// codes into Rust's type-safe error handling. It treats non-negative values
    /// as success and negative values as errors.
    ///
    /// # Arguments
    ///
    /// * `code` - The return code from an AngelScript function
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the code indicates success (>= 0)
    /// * `Err(ScriptError)` if the code indicates an error (< 0)
    ///
    /// # AngelScript Return Code Convention
    ///
    /// AngelScript uses the following convention for return codes:
    /// - `>= 0`: Success (may include additional information)
    /// - `< 0`: Error (specific error type determined by the value)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use angelscript_rs::{ScriptError, ReturnCode};
    ///
    /// // Success case
    /// assert!(ScriptError::from_code(0).is_ok());
    /// assert!(ScriptError::from_code(1).is_ok());
    ///
    /// // Error case
    /// let result = ScriptError::from_code(-1);
    /// assert!(result.is_err());
    ///
    /// // Check specific error type
    /// match ScriptError::from_code(-10) {
    ///     Err(ScriptError::AngelScriptError(ReturnCode::InvalidName)) => {
    ///         println!("Invalid name provided");
    ///     }
    ///     _ => {}
    /// }
    /// ```
    ///
    /// # Internal Usage
    ///
    /// This function is primarily used internally by the bindings to convert
    /// AngelScript return codes:
    ///
    /// ```rust
    /// // Internal usage pattern
    /// unsafe {
    ///     let result = angelscript_sys::some_function();
    ///     ScriptError::from_code(result)?; // Convert and propagate errors
    /// }
    /// ```
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

/// Automatic conversion from mutex poison errors.
///
/// This implementation allows mutex poison errors to be automatically converted
/// to `ScriptError::MutexPoisoned`, enabling seamless error propagation when
/// working with shared data structures.
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::{ScriptError, ScriptResult};
/// use std::sync::{Arc, Mutex};
///
/// fn process_shared_data(data: Arc<Mutex<Vec<i32>>>) -> ScriptResult<i32> {
///     let guard = data.lock()?; // Automatic conversion on poison
///     Ok(guard.len() as i32)
/// }
/// ```
impl<T> From<PoisonError<MutexGuard<'_, T>>> for ScriptError {
    fn from(_: PoisonError<MutexGuard<'_, T>>) -> Self {
        ScriptError::MutexPoisoned
    }
}
