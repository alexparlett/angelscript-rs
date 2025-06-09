use crate::core::context::Context;
use crate::core::engine::Engine;
use crate::core::function::Function;
use crate::core::module::Module;
use crate::core::script_generic::ScriptGeneric;
use crate::core::script_object::ScriptObject;
use crate::core::typeinfo::TypeInfo;
use crate::types::enums::MessageType;
use crate::types::script_memory::ScriptMemoryLocation;

// ========== CONTEXT MANAGEMENT CALLBACKS ==========

/// Callback for requesting a script context from a custom pool.
///
/// This callback is invoked when AngelScript needs a context for script execution.
/// It allows applications to implement custom context pooling strategies for
/// performance optimization.
///
/// # Arguments
/// * `engine` - The engine requesting the context
///
/// # Returns
/// A context instance, or None if no context is available
///
/// # Performance Considerations
///
/// Context creation can be expensive, so implementing a context pool can significantly
/// improve performance for applications that execute scripts frequently. The callback
/// should be fast and avoid blocking operations.
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::{Engine, Context};
/// use std::sync::{Arc, Mutex};
/// use std::collections::VecDeque;
///
/// // Thread-safe context pool
/// lazy_static! {
///     static ref CONTEXT_POOL: Arc<Mutex<VecDeque<Context>>> =
///         Arc::new(Mutex::new(VecDeque::new()));
/// }
///
/// fn request_context_from_pool(engine: &Engine) -> Option<Context> {
///     let mut pool = CONTEXT_POOL.lock().unwrap();
///
///     // Try to reuse an existing context
///     if let Some(mut context) = pool.pop_front() {
///         // Reset context state before reuse
///         context.unprepare().ok();
///         Some(context)
///     } else {
///         // Create new context if pool is empty
///         match engine.create_context() {
///             Ok(context) => Some(context),
///             Err(_) => None,
///         }
///     }
/// }
/// ```
pub type RequestContextCallbackFn = fn(&Engine) -> Option<Context>;

/// Callback for returning a script context to a custom pool.
///
/// This callback is invoked when AngelScript is done with a context and wants to
/// return it to the application's context pool. It's the counterpart to
/// [`RequestContextCallbackFn`].
///
/// # Arguments
/// * `engine` - The engine returning the context
/// * `context` - The context being returned
///
/// # Implementation Notes
///
/// The callback should:
/// - Clean up any context state if necessary
/// - Return the context to the pool for reuse
/// - Handle any errors gracefully
/// - Be thread-safe if the engine is used from multiple threads
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::{Engine, Context};
///
/// fn return_context_to_pool(engine: &Engine, context: &Context) {
///     let mut pool = CONTEXT_POOL.lock().unwrap();
///
///     // Optionally clean up context state
///     // context.clear_user_data();
///
///     // Return to pool if there's space
///     if pool.len() < MAX_POOL_SIZE {
///         pool.push_back(context.clone());
///     }
///     // If pool is full, just let the context be destroyed
/// }
/// ```
pub type ReturnContextCallbackFn = fn(&Engine, &Context);

// ========== MEMORY MANAGEMENT CALLBACKS ==========

/// Callback for when circular references are detected in the garbage collector.
///
/// This callback is invoked when AngelScript's garbage collector detects objects
/// that reference each other in a cycle, preventing normal reference counting
/// from cleaning them up.
///
/// # Arguments
/// * `type_info` - Type information for the objects involved
/// * `obj1` - Memory location of the first object in the cycle
/// * `obj2` - Memory location of the second object in the cycle
///
/// # Use Cases
///
/// - Logging circular reference detection for debugging
/// - Implementing custom cleanup strategies
/// - Tracking memory usage patterns
/// - Debugging memory leaks
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::{TypeInfo, ScriptMemoryLocation};
///
/// fn handle_circular_reference(
///     type_info: &TypeInfo,
///     obj1: ScriptMemoryLocation,
///     obj2: ScriptMemoryLocation
/// ) {
///     eprintln!(
///         "Circular reference detected in type '{}' between objects at {:?} and {:?}",
///         type_info.get_name().unwrap_or("unknown"),
///         obj1,
///         obj2
///     );
///
///     // Optionally log to file or send to monitoring system
///     log_circular_reference(type_info, obj1, obj2);
/// }
///
/// let mut engine = Engine::create()?;
/// engine.set_circular_ref_detected_callback(handle_circular_reference)?;
/// ```
pub type CircularRefCallbackFn = fn(&TypeInfo, ScriptMemoryLocation, ScriptMemoryLocation);

/// Callback for cleaning up engine user data.
///
/// This callback is invoked when the engine is being destroyed and needs to
/// clean up any user data that was associated with it.
///
/// # Arguments
/// * `engine` - The engine being cleaned up
///
/// # Implementation Notes
///
/// The callback should:
/// - Free any resources associated with the engine
/// - Not attempt to use the engine for script operations
/// - Handle cleanup errors gracefully
/// - Be fast to avoid delaying engine destruction
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::Engine;
///
/// fn cleanup_engine_data(engine: &Engine) {
///     // Clean up any global resources associated with this engine
///     if let Some(data) = engine.get_user_data::<MyEngineData>() {
///         data.cleanup();
///     }
///
///     // Log engine destruction
///     println!("Engine {:p} is being destroyed", engine as *const _);
/// }
///
/// let mut engine = Engine::create()?;
/// engine.set_engine_user_data_cleanup_callback(cleanup_engine_data, MY_DATA_TYPE_ID)?;
/// ```
pub type CleanEngineUserDataCallbackFn = fn(&Engine);

/// Callback for cleaning up module user data.
///
/// This callback is invoked when a module is being destroyed and needs to
/// clean up any user data that was associated with it.
///
/// # Arguments
/// * `module` - The module being cleaned up
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::Module;
///
/// fn cleanup_module_data(module: &Module) {
///     if let Some(data) = module.get_user_data::<MyModuleData>() {
///         data.save_to_disk();
///         data.cleanup();
///     }
///
///     println!("Module '{}' is being destroyed",
///              module.get_name().unwrap_or("unnamed"));
/// }
///
/// let mut engine = Engine::create()?;
/// engine.set_module_user_data_cleanup_callback(cleanup_module_data, MY_DATA_TYPE_ID)?;
/// ```
pub type CleanModuleUserDataCallbackFn = fn(&Module);

/// Callback for cleaning up context user data.
///
/// This callback is invoked when a context is being destroyed and needs to
/// clean up any user data that was associated with it.
///
/// # Arguments
/// * `context` - The context being cleaned up
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::Context;
///
/// fn cleanup_context_data(context: &Context) {
///     if let Some(data) = context.get_user_data::<MyContextData>() {
///         data.finalize_execution();
///         data.cleanup();
///     }
/// }
///
/// let mut engine = Engine::create()?;
/// engine.set_context_user_data_cleanup_callback(cleanup_context_data, MY_DATA_TYPE_ID)?;
/// ```
pub type CleanContextUserDataCallbackFn = fn(&Context);

/// Callback for cleaning up function user data.
///
/// This callback is invoked when a function is being destroyed and needs to
/// clean up any user data that was associated with it.
///
/// # Arguments
/// * `function` - The function being cleaned up
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::Function;
///
/// fn cleanup_function_data(function: &Function) {
///     if let Some(data) = function.get_user_data::<MyFunctionData>() {
///         data.cleanup_profiling_data();
///         data.cleanup();
///     }
///
///     println!("Function '{}' is being destroyed",
///              function.get_name().unwrap_or("unnamed"));
/// }
///
/// let mut engine = Engine::create()?;
/// engine.set_function_user_data_cleanup_callback(cleanup_function_data, MY_DATA_TYPE_ID)?;
/// ```
pub type CleanFunctionUserDataCallbackFn = fn(&Function);

/// Callback for cleaning up type info user data.
///
/// This callback is invoked when type information is being destroyed and needs to
/// clean up any user data that was associated with it.
///
/// # Arguments
/// * `type_info` - The type info being cleaned up
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::TypeInfo;
///
/// fn cleanup_type_data(type_info: &TypeInfo) {
///     if let Some(data) = type_info.get_user_data::<MyTypeData>() {
///         data.cleanup_reflection_cache();
///         data.cleanup();
///     }
///
///     println!("Type '{}' is being destroyed",
///              type_info.get_name().unwrap_or("unnamed"));
/// }
///
/// let mut engine = Engine::create()?;
/// engine.set_type_info_user_data_cleanup_callback(cleanup_type_data, MY_DATA_TYPE_ID)?;
/// ```
pub type CleanTypeInfoCallbackFn = fn(&TypeInfo);

/// Callback for cleaning up script object user data.
///
/// This callback is invoked when a script object is being destroyed and needs to
/// clean up any user data that was associated with it.
///
/// # Arguments
/// * `object` - The script object being cleaned up
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::ScriptObject;
///
/// fn cleanup_object_data(object: &ScriptObject) {
///     if let Some(data) = object.get_user_data::<MyObjectData>() {
///         data.cleanup_native_resources();
///         data.cleanup();
///     }
/// }
///
/// let mut engine = Engine::create()?;
/// engine.set_script_object_user_data_cleanup_callback(cleanup_object_data, MY_DATA_TYPE_ID)?;
/// ```
pub type CleanScriptObjectCallbackFn = fn(&ScriptObject);

// ========== EXECUTION AND DEBUGGING CALLBACKS ==========

/// Callback for handling exceptions during script execution.
///
/// This callback is invoked when an exception occurs during script execution,
/// allowing the application to handle the exception, log it, or perform
/// custom error recovery.
///
/// # Arguments
/// * `context` - The context where the exception occurred
/// * `user_data` - User-defined data passed when setting the callback
///
/// # Use Cases
///
/// - Custom exception logging and reporting
/// - Exception recovery and continuation
/// - Debugging and error analysis
/// - Integration with application error handling systems
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::{Context, ScriptMemoryLocation};
///
/// fn handle_script_exception(context: &Context, user_data: ScriptMemoryLocation) {
///     // Get exception information
///     if let Some(exception_msg) = context.get_exception_string() {
///         let (line, col, section) = context.get_exception_line_number();
///
///         eprintln!("Script exception at {}:{} in {:?}: {}",
///                   line, col.unwrap_or(0), section, exception_msg);
///
///         // Log call stack
///         let stack_size = context.get_callstack_size();
///         for i in 0..stack_size {
///             if let Some(func) = context.get_function(i) {
///                 let (func_line, func_col, func_section) = context.get_line_number(i);
///                 println!("  at {} ({}:{} in {:?})",
///                          func.get_name().unwrap_or("unknown"),
///                          func_line, func_col.unwrap_or(0), func_section);
///             }
///         }
///     }
/// }
///
/// let context = engine.create_context()?;
/// context.set_exception_callback(handle_script_exception)?;
/// ```
pub type ExceptionCallbackFn = fn(&Context, ScriptMemoryLocation);

/// Callback for line-by-line execution monitoring.
///
/// This callback is invoked for each line of script code that is executed,
/// allowing for detailed execution monitoring, profiling, and debugging.
///
/// # Arguments
/// * `context` - The context executing the line
/// * `user_data` - User-defined data passed when setting the callback
///
/// # Performance Impact
///
/// This callback is called very frequently during script execution and can
/// significantly impact performance. Use it judiciously and keep the
/// implementation as fast as possible.
///
/// # Use Cases
///
/// - Script debugging and breakpoints
/// - Performance profiling and analysis
/// - Execution tracing and logging
/// - Custom script execution control
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::{Context, ScriptMemoryLocation};
/// use std::sync::atomic::{AtomicU64, Ordering};
///
/// static LINE_COUNT: AtomicU64 = AtomicU64::new(0);
///
/// fn line_execution_callback(context: &Context, user_data: ScriptMemoryLocation) {
///     let count = LINE_COUNT.fetch_add(1, Ordering::Relaxed);
///
///     // Every 1000 lines, print progress
///     if count % 1000 == 0 {
///         if let Some(func) = context.get_function(0) {
///             let (line, _, section) = context.get_line_number(0);
///             println!("Executed {} lines, currently at line {} in {} of function {}",
///                      count, line,
///                      section.unwrap_or("unknown"),
///                      func.get_name().unwrap_or("unknown"));
///         }
///     }
///
///     // Check for breakpoints or other debugging conditions
///     check_breakpoints(context);
/// }
///
/// let mut context = engine.create_context()?;
/// context.set_line_callback(line_execution_callback, &mut ())?;
/// ```
pub type LineCallbackFn = fn(&Context, ScriptMemoryLocation);

/// Callback for translating application exceptions to script exceptions.
///
/// This callback is invoked when an application function called from script
/// throws an exception, allowing the application to translate native exceptions
/// into script-understandable exceptions.
///
/// # Arguments
/// * `context` - The context where the exception occurred
/// * `user_data` - User-defined data passed when setting the callback
///
/// # Use Cases
///
/// - Converting C++ exceptions to script exceptions
/// - Providing meaningful error messages to script code
/// - Maintaining exception context across language boundaries
/// - Custom error handling and recovery
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::{Context, ScriptMemoryLocation};
///
/// fn translate_exception(context: &Context, user_data: ScriptMemoryLocation) {
///     // Check what kind of exception occurred in the application
///     // This is typically done by checking thread-local storage or
///     // other mechanisms where the application stored exception info
///
///     if let Some(app_exception) = get_current_app_exception() {
///         match app_exception {
///             AppException::FileNotFound(path) => {
///                 context.set_exception(
///                     &format!("File not found: {}", path),
///                     true
///                 ).ok();
///             }
///             AppException::InvalidArgument(msg) => {
///                 context.set_exception(
///                     &format!("Invalid argument: {}", msg),
///                     true
///                 ).ok();
///             }
///             AppException::OutOfMemory => {
///                 context.set_exception("Out of memory", false).ok();
///             }
///         }
///     }
/// }
///
/// let mut engine = Engine::create()?;
/// engine.set_translate_app_exception_callback(translate_exception)?;
/// ```
pub type TranslateAppExceptionCallbackFn = fn(&Context, ScriptMemoryLocation);

// ========== FUNCTION REGISTRATION CALLBACKS ==========

/// Generic function implementation callback.
///
/// This callback type is used for implementing functions that are registered
/// with AngelScript using the generic calling convention. It provides a
/// flexible way to implement functions that can handle any parameter types
/// and return values.
///
/// # Arguments
/// * `generic` - The generic interface providing access to arguments and return value
///
/// # Generic Interface
///
/// The [`ScriptGeneric`] parameter provides methods to:
/// - Get function arguments using `get_arg(index)`
/// - Set the return value using `set_return_*` methods
/// - Access the calling object for methods
/// - Get auxiliary data associated with the function
///
/// # Examples
///
/// ## Simple Function
///
/// ```rust
/// use angelscript_rs::{Engine, ScriptGeneric};
///
/// fn add_numbers(ctx: &ScriptGeneric) {
///     // Get arguments
///     let a: i32 = ctx.get_arg(0);
///     let b: i32 = ctx.get_arg(1);
///
///     // Calculate result
///     let result = a + b;
///
///     // Set return value
///     ctx.set_return_dword(result as u32);
/// }
///
/// let engine = Engine::create()?;
/// engine.register_global_function("int add(int, int)", add_numbers, None)?;
/// ```
///
/// ## Method Implementation
///
/// ```rust
/// fn vector_length(ctx: &ScriptGeneric) {
///     // Get the object instance (for methods)
///     let vector: &Vector3 = ctx.get_object();
///
///     // Calculate length
///     let length = (vector.x * vector.x + vector.y * vector.y + vector.z * vector.z).sqrt();
///
///     // Return the result
///     ctx.set_return_float(length);
/// }
///
/// engine.register_object_method(
///     "Vector3",
///     "float length()",
///     vector_length,
///     None,
///     None,
///     None
/// )?;
/// ```
///
/// ## Function with String Parameters
///
/// ```rust
/// fn string_concat(ctx: &ScriptGeneric) {
///     // Get string arguments
///     let str1: String = ctx.get_arg(0);
///     let str2: String = ctx.get_arg(1);
///
///     // Concatenate
///     let result = format!("{}{}", str1, str2);
///
///     // Return new string
///     ctx.set_return_object(&result);
/// }
///
/// engine.register_global_function(
///     "string concat(const string &in, const string &in)",
///     string_concat,
///     None
/// )?;
/// ```
///
/// ## Error Handling
///
/// ```rust
/// fn safe_divide(ctx: &ScriptGeneric) {
///     let a: f32 = ctx.get_arg(0);
///     let b: f32 = ctx.get_arg(1);
///
///     if b == 0.0 {
///         // Set an exception instead of returning a value
///         ctx.set_exception("Division by zero");
///         return;
///     }
///
///     let result = a / b;
///     ctx.set_return_float(result);
/// }
///
/// engine.register_global_function("float divide(float, float)", safe_divide, None)?;
/// ```
pub type GenericFn = fn(&ScriptGeneric);

// ========== COMPILATION AND MESSAGING CALLBACKS ==========

/// Information about a compilation or runtime message.
///
/// This struct contains detailed information about messages generated during
/// script compilation, including errors, warnings, and informational messages.
///
/// # Fields
///
/// - `section`: The name of the script section where the message originated
/// - `row`: The line number (1-based) where the message occurred
/// - `col`: The column number (1-based) where the message occurred
/// - `msg_type`: The type of message (error, warning, or information)
/// - `message`: The actual message text
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::{MessageInfo, MessageType};
///
/// fn process_message(info: &MessageInfo) {
///     let prefix = match info.msg_type {
///         MessageType::Error => "ERROR",
///         MessageType::Warning => "WARN",
///         MessageType::Information => "INFO",
///     };
///
///     println!("[{}] {}:{}:{} - {}",
///              prefix, info.section, info.row, info.col, info.message);
/// }
/// ```
#[derive(Debug)]
pub struct MessageInfo {
    /// The script section name where the message originated
    pub section: String,
    /// The line number (1-based) where the message occurred
    pub row: u32,
    /// The column number (1-based) where the message occurred
    pub col: u32,
    /// The type of message (error, warning, or information)
    pub msg_type: MessageType,
    /// The actual message text
    pub message: String,
}

impl MessageInfo {
    /// Creates a new MessageInfo instance.
    ///
    /// # Arguments
    /// * `section` - The script section name
    /// * `row` - The line number
    /// * `col` - The column number
    /// * `msg_type` - The message type
    /// * `message` - The message text
    ///
    /// # Returns
    /// A new MessageInfo instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// let info = MessageInfo::new(
    ///     "main.as".to_string(),
    ///     42,
    ///     15,
    ///     MessageType::Error,
    ///     "Undefined variable 'foo'".to_string()
    /// );
    /// ```
    pub fn new(section: String, row: u32, col: u32, msg_type: MessageType, message: String) -> Self {
        Self {
            section,
            row,
            col,
            msg_type,
            message,
        }
    }

    /// Checks if this message represents an error.
    ///
    /// # Returns
    /// true if this is an error message, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// if info.is_error() {
    ///     eprintln!("Compilation failed: {}", info.message);
    /// }
    /// ```
    pub fn is_error(&self) -> bool {
        matches!(self.msg_type, MessageType::Error)
    }

    /// Checks if this message represents a warning.
    ///
    /// # Returns
    /// true if this is a warning message, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// if info.is_warning() {
    ///     println!("Warning: {}", info.message);
    /// }
    /// ```
    pub fn is_warning(&self) -> bool {
        matches!(self.msg_type, MessageType::Warning)
    }

    /// Checks if this message is informational.
    ///
    /// # Returns
    /// true if this is an informational message, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// if info.is_info() {
    ///     println!("Info: {}", info.message);
    /// }
    /// ```
    pub fn is_info(&self) -> bool {
        matches!(self.msg_type, MessageType::Information)
    }

    /// Formats the message for display.
    ///
    /// # Returns
    /// A formatted string representation of the message
    ///
    /// # Examples
    ///
    /// ```rust
    /// let formatted = info.format();
    /// println!("{}", formatted);
    /// // Output: "ERROR in main.as(42:15): Undefined variable 'foo'"
    /// ```
    pub fn format(&self) -> String {
        let type_str = match self.msg_type {
            MessageType::Error => "ERROR",
            MessageType::Warning => "WARNING",
            MessageType::Information => "INFO",
        };

        format!("{} in {}({}:{}): {}",
                type_str, self.section, self.row, self.col, self.message)
    }

    /// Gets the location as a formatted string.
    ///
    /// # Returns
    /// A string in the format "section(row:col)"
    ///
    /// # Examples
    ///
    /// ```rust
    /// let location = info.get_location();
    /// println!("Error at {}", location);
    /// // Output: "Error at main.as(42:15)"
    /// ```
    pub fn get_location(&self) -> String {
        format!("{}({}:{})", self.section, self.row, self.col)
    }
}

/// Callback for handling compilation and runtime messages.
///
/// This callback is invoked whenever AngelScript generates a message during
/// compilation or execution. It allows applications to handle errors, warnings,
/// and informational messages in a custom way.
///
/// # Arguments
/// * `info` - Detailed information about the message
///
/// # Message Types
///
/// - **Error**: Compilation or runtime errors that prevent execution
/// - **Warning**: Potential issues that don't prevent compilation
/// - **Information**: General informational messages
///
/// # Use Cases
///
/// - Custom error reporting and logging
/// - IDE integration for syntax highlighting and error display
/// - Build system integration
/// - Debugging and development tools
///
/// # Examples
///
/// ## Basic Message Handling
///
/// ```rust
/// use angelscript_rs::{Engine, MessageInfo, MessageType};
///
/// fn handle_messages(info: &MessageInfo) {
///     match info.msg_type {
///         MessageType::Error => {
///             eprintln!("Compilation Error: {}", info.format());
///         }
///         MessageType::Warning => {
///             println!("Warning: {}", info.format());
///         }
///         MessageType::Information => {
///             println!("Info: {}", info.message);
///         }
///     }
/// }
///
/// let mut engine = Engine::create()?;
/// engine.set_message_callback(handle_messages)?;
/// ```
///
/// ## Advanced Message Processing
///
/// ```rust
/// use std::sync::{Arc, Mutex};
/// use std::collections::HashMap;
///
/// // Message collector for batch processing
/// lazy_static! {
///     static ref MESSAGE_COLLECTOR: Arc<Mutex<Vec<MessageInfo>>> =
///         Arc::new(Mutex::new(Vec::new()));
/// }
///
/// fn collect_messages(info: &MessageInfo) {
///     let mut collector = MESSAGE_COLLECTOR.lock().unwrap();
///     collector.push(MessageInfo::new(
///         info.section.clone(),
///         info.row,
///         info.col,
///         info.msg_type,
///         info.message.clone()
///     ));
/// }
///
/// fn process_collected_messages() -> (usize, usize, usize) {
///     let messages = MESSAGE_COLLECTOR.lock().unwrap();
///     let mut errors = 0;
///     let mut warnings = 0;
///     let mut infos = 0;
///
///     for msg in messages.iter() {
///         match msg.msg_type {
///             MessageType::Error => errors += 1,
///             MessageType::Warning => warnings += 1,
///             MessageType::Information => infos += 1,
///         }
///     }
///
///     (errors, warnings, infos)
/// }
///
/// let mut engine = Engine::create()?;
/// engine.set_message_callback(collect_messages)?;
///
/// // After compilation
/// let (errors, warnings, infos) = process_collected_messages();
/// println!("Compilation completed: {} errors, {} warnings, {} info messages",
///          errors, warnings, infos);
/// ```
///
/// ## File-based Logging
///
/// ```rust
/// use std::fs::OpenOptions;
/// use std::io::Write;
///
/// fn log_messages_to_file(info: &MessageInfo) {
///     let mut file = OpenOptions::new()
///         .create(true)
///         .append(true)
///         .open("script_messages.log")
///         .unwrap();
///
///     writeln!(file, "[{}] {}",
///              chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
///              info.format()).ok();
/// }
///
/// let mut engine = Engine::create()?;
/// engine.set_message_callback(log_messages_to_file)?;
/// ```
pub type MessageCallbackFn = fn(&MessageInfo, &mut ScriptMemoryLocation);
