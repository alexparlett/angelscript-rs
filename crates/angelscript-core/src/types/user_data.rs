use angelscript_sys::asPWORD;

/// A trait for types that can be stored as user data in AngelScript objects.
///
/// This trait allows Rust types to be associated with AngelScript objects such as
/// engines, contexts, functions, modules, and type information. Each type that
/// implements `UserData` must provide a unique key that identifies it when storing
/// and retrieving the data.
///
/// # Purpose
///
/// User data provides a way to:
/// - Associate application-specific data with AngelScript objects
/// - Store metadata or configuration for script objects
/// - Implement custom cleanup or management logic
/// - Bridge between Rust application state and script execution
///
/// # Key Requirements
///
/// The `KEY` constant must be unique across all types in your application.
/// Using duplicate keys will cause data to be overwritten or retrieved incorrectly.
///
/// # Thread Safety
///
/// Types implementing `UserData` should consider thread safety if they will be
/// accessed from multiple threads. AngelScript objects can be shared between
/// threads when properly synchronized.
///
/// # Examples
///
/// ## Basic Implementation
///
/// ```rust
/// use angelscript_rs::{UserData, ScriptData};
/// use angelscript_sys::asPWORD;
///
/// #[derive(Debug)]
/// struct MyUserData {
///     id: u32,
///     name: String,
///     config: Vec<String>,
/// }
///
/// impl UserData for MyUserData {
///     // Use a unique identifier - consider using a hash of the type name
///     const KEY: asPWORD = 0x12345678;
/// }
///
/// impl ScriptData for MyUserData {
///     // Implementation for converting to/from script pointers
///     // ... (implementation details)
/// }
/// ```
///
/// ## Engine User Data
///
/// ```rust
/// use angelscript_rs::{Engine, UserData, ScriptData};
///
/// #[derive(Debug)]
/// struct EngineConfig {
///     debug_mode: bool,
///     max_execution_time: u32,
///     custom_paths: Vec<String>,
/// }
///
/// impl UserData for EngineConfig {
///     const KEY: asPWORD = 0x87654321;
/// }
///
/// impl ScriptData for EngineConfig {
///     // ... implementation
/// }
///
/// // Usage
/// let engine = Engine::create()?;
/// let mut config = EngineConfig {
///     debug_mode: true,
///     max_execution_time: 5000,
///     custom_paths: vec!["./scripts".to_string()],
/// };
///
/// // Store user data
/// engine.set_user_data(&mut config);
///
/// // Retrieve user data later
/// if let Ok(retrieved_config) = engine.get_user_data::<EngineConfig>() {
///     println!("Debug mode: {}", retrieved_config.debug_mode);
/// }
/// ```
///
/// ## Function User Data
///
/// ```rust
/// #[derive(Debug)]
/// struct FunctionMetadata {
///     performance_stats: std::time::Duration,
///     call_count: u32,
///     last_error: Option<String>,
/// }
///
/// impl UserData for FunctionMetadata {
///     const KEY: asPWORD = 0xABCDEF00;
/// }
///
/// impl ScriptData for FunctionMetadata {
///     // ... implementation
/// }
///
/// // Usage with functions
/// let function = module.get_function_by_name("myFunction")?;
/// let mut metadata = FunctionMetadata {
///     performance_stats: std::time::Duration::new(0, 0),
///     call_count: 0,
///     last_error: None,
/// };
///
/// function.set_user_data(&mut metadata);
/// ```
///
/// ## Multiple User Data Types
///
/// ```rust
/// // Different types can be stored on the same object
/// #[derive(Debug)]
/// struct DebugInfo {
///     breakpoints: Vec<u32>,
///     watch_variables: Vec<String>,
/// }
///
/// impl UserData for DebugInfo {
///     const KEY: asPWORD = 0x11111111;
/// }
///
/// #[derive(Debug)]
/// struct PerformanceInfo {
///     execution_time: std::time::Duration,
///     memory_usage: usize,
/// }
///
/// impl UserData for PerformanceInfo {
///     const KEY: asPWORD = 0x22222222; // Different key!
/// }
///
/// // Both can be stored on the same context
/// let context = engine.create_context()?;
/// let mut debug_info = DebugInfo { /* ... */ };
/// let mut perf_info = PerformanceInfo { /* ... */ };
///
/// context.set_user_data(&mut debug_info);
/// context.set_user_data(&mut perf_info);
///
/// // Retrieve them independently
/// let debug = context.get_user_data::<DebugInfo>();
/// let perf = context.get_user_data::<PerformanceInfo>();
/// ```
///
/// # Key Generation Strategies
///
/// ## Manual Assignment
/// ```rust
/// impl UserData for MyType {
///     const KEY: asPWORD = 0x12345678; // Manually chosen
/// }
/// ```
///
/// ## Hash-Based (Recommended)
/// ```rust
/// use std::collections::hash_map::DefaultHasher;
/// use std::hash::{Hash, Hasher};
///
/// impl UserData for MyType {
///     const KEY: asPWORD = {
///         // This would need to be computed at compile time
///         // Consider using a build script or macro for this
///         0x12345678 // Placeholder - use actual hash
///     };
/// }
/// ```
///
/// ## Macro-Generated
/// ```rust
/// macro_rules! impl_user_data {
///     ($type:ty, $key:expr) => {
///         impl UserData for $type {
///             const KEY: asPWORD = $key;
///         }
///     };
/// }
///
/// impl_user_data!(MyType, 0x12345678);
/// ```
///
/// # Best Practices
///
/// 1. **Unique Keys**: Ensure each type has a unique `KEY` value
/// 2. **Documentation**: Document what each user data type is used for
/// 3. **Cleanup**: Implement proper cleanup in `Drop` if needed
/// 4. **Thread Safety**: Consider `Send` and `Sync` bounds for threaded usage
/// 5. **Versioning**: Consider including version information in complex user data
///
/// # Common Patterns
///
/// ## Configuration Storage
/// ```rust
/// #[derive(Debug)]
/// struct ModuleConfig {
///     optimization_level: u8,
///     enable_debugging: bool,
///     custom_imports: Vec<String>,
/// }
///
/// impl UserData for ModuleConfig {
///     const KEY: asPWORD = 0x10203040;
/// }
/// ```
///
/// ## State Tracking
/// ```rust
/// #[derive(Debug)]
/// struct ExecutionState {
///     current_line: u32,
///     variables_modified: std::collections::HashSet<String>,
///     execution_start: std::time::Instant,
/// }
///
/// impl UserData for ExecutionState {
///     const KEY: asPWORD = 0x50607080;
/// }
/// ```
///
/// ## Resource Management
/// ```rust
/// #[derive(Debug)]
/// struct ResourceTracker {
///     allocated_objects: Vec<*mut std::ffi::c_void>,
///     file_handles: Vec<std::fs::File>,
///     network_connections: Vec<std::net::TcpStream>,
/// }
///
/// impl UserData for ResourceTracker {
///     const KEY: asPWORD = 0x90A0B0C0;
/// }
///
/// impl Drop for ResourceTracker {
///     fn drop(&mut self) {
///         // Clean up resources when user data is dropped
///         for &ptr in &self.allocated_objects {
///             unsafe {
///                 // Free allocated memory
///             }
///         }
///     }
/// }
/// ```
pub trait UserData {
    /// Unique identifier for this user data type.
    ///
    /// This key is used by AngelScript to distinguish between different types
    /// of user data stored on the same object. Each type implementing `UserData`
    /// must have a unique `KEY` value.
    ///
    /// # Requirements
    ///
    /// - Must be unique across all `UserData` implementations in your application
    /// - Should be a compile-time constant
    /// - Recommended to use values that are unlikely to collide accidentally
    ///
    /// # Examples
    ///
    /// ```rust
    /// impl UserData for MyType {
    ///     const KEY: asPWORD = 0x12345678;
    /// }
    ///
    /// impl UserData for AnotherType {
    ///     const KEY: asPWORD = 0x87654321; // Different from MyType
    /// }
    /// ```
    ///
    /// # Key Collision
    ///
    /// If two types use the same key, they will overwrite each other's data:
    ///
    /// ```rust
    /// // BAD: Both types use the same key
    /// impl UserData for TypeA {
    ///     const KEY: asPWORD = 0x12345678;
    /// }
    ///
    /// impl UserData for TypeB {
    ///     const KEY: asPWORD = 0x12345678; // COLLISION!
    /// }
    ///
    /// // This will cause problems:
    /// let mut data_a = TypeA::new();
    /// let mut data_b = TypeB::new();
    ///
    /// engine.set_user_data(&mut data_a);
    /// engine.set_user_data(&mut data_b); // Overwrites data_a!
    ///
    /// // This will return data_b, not data_a
    /// let retrieved = engine.get_user_data::<TypeA>();
    /// ```
    const KEY: asPWORD;
}
