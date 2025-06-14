use crate::core::context::Context;
use crate::core::diagnostics::{Diagnostic, DiagnosticKind, Diagnostics};
use crate::core::error::{ScriptError, ScriptResult};
use crate::core::function::Function;
use crate::core::lockable_shared_bool::LockableSharedBool;
use crate::core::module::Module;
use crate::core::script_generic::ScriptGeneric;
use crate::core::typeinfo::TypeInfo;
use crate::internal::callback_manager::CallbackManager;
use crate::internal::jit_compiler::JITCompiler;
use crate::internal::thread_manager::{ExclusiveLockGuard, SharedLockGuard, ThreadManager};
use crate::types::callbacks::{
    CircularRefCallbackFn, CleanContextUserDataCallbackFn, CleanEngineUserDataCallbackFn,
    CleanFunctionUserDataCallbackFn, CleanModuleUserDataCallbackFn, CleanScriptObjectCallbackFn,
    CleanTypeInfoCallbackFn, GenericFn, MessageCallbackFn, MessageInfo, RequestContextCallbackFn,
    ReturnContextCallbackFn, TranslateAppExceptionCallbackFn,
};
use crate::types::enums::*;
use crate::types::script_data::ScriptData;
use crate::types::script_memory::{ScriptMemoryLocation, Void};
use crate::types::user_data::UserData;
use angelscript_sys::*;
use std::alloc::{alloc, Layout};
use std::ffi::{c_char, CStr, CString};
use std::marker::PhantomData;
use std::os::raw::c_void;
use std::ptr;
use std::ptr::NonNull;

/// Trait for types that can be installed into an AngelScript engine.
///
/// This trait allows for modular registration of functionality with the engine,
/// such as types, functions, or entire modules.
pub trait EngineInstallable {
    /// Install this component into the given engine.
    ///
    /// # Arguments
    /// * `engine` - The engine to install into
    ///
    /// # Returns
    /// A result indicating success or failure of the installation
    fn install(self, engine: &Engine) -> ScriptResult<()>;
}

/// The main AngelScript engine wrapper.
///
/// This struct provides a safe Rust interface to the AngelScript scripting engine.
/// It handles script compilation, execution, type registration, and memory management.
///
/// # Thread Safety
/// The Engine is thread-safe when AngelScript is prepared for multithreading
/// using [`Engine::prepare_multithread`].
///
/// # Examples
///
/// ```rust
/// use angelscript::prelude::Engine;
///
/// // Create a new engine
/// let engine = Engine::create()?;
///
/// // Register a global function
/// engine.register_global_function("void print(const string &in)", print_function, None)?;
///
/// // Get a module and add script code
/// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
/// module.add_script_section("script", "void main() { print(\"Hello World!\"); }")?;
/// module.build()?;
///
/// // Execute the script
/// let context = engine.create_context()?;
/// let function = module.get_function_by_name("main")?;
/// context.prepare(function)?;
/// context.execute()?;
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Engine {
    inner: NonNull<asIScriptEngine>,
    is_root: bool,
    phantom_data: PhantomData<asIScriptEngine>,
}

impl Engine {
    /// Custom memory allocator that uses Rust's allocator.
    ///
    /// This function is used when the `rust-alloc` feature is enabled
    /// to ensure all memory allocation goes through Rust's allocator.
    #[cfg(feature = "rust-alloc")]
    pub unsafe extern "C" fn unified_alloc(size: usize) -> *mut std::ffi::c_void {
        unsafe {
            let layout = Layout::from_size_align(size, 8).unwrap();
            alloc(layout) as *mut std::ffi::c_void
        }
    }

    /// Custom memory deallocator that uses libc's free.
    ///
    /// This function is used when the `rust-alloc` feature is enabled.
    #[cfg(feature = "rust-alloc")]
    pub unsafe extern "C" fn unified_free(ptr: *mut std::ffi::c_void) {
        unsafe {
            if !ptr.is_null() {
                libc::free(ptr);
            }
        }
    }

    /// Creates a new AngelScript engine.
    ///
    /// This initializes the AngelScript engine and optionally sets up
    /// custom memory allocation functions when the `rust-alloc` feature is enabled.
    ///
    /// # Returns
    /// A new Engine instance or an error if creation failed
    ///
    /// # Examples
    ///
    /// ```rust
    /// let engine = Engine::create()?;
    /// ```
    pub fn create() -> ScriptResult<Engine> {
        unsafe {
            #[cfg(feature = "rust-alloc")]
            Self::set_global_memory_functions(Some(Self::unified_alloc), Some(Self::unified_free))?;

            let engine_ptr = asCreateScriptEngine(ANGELSCRIPT_VERSION as asDWORD);

            let engine_wrapper = Engine {
                inner: NonNull::new(engine_ptr).ok_or(ScriptError::FailedToCreateEngine)?,
                is_root: true,
                phantom_data: PhantomData,
            };
            Ok(engine_wrapper)
        }
    }

    /// Installs a component that implements [`EngineInstallable`] into this engine.
    ///
    /// This is a convenience method for registering types, functions, or modules.
    ///
    /// # Arguments
    /// * `installable` - The component to install
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn install<T: EngineInstallable>(&self, installable: T) -> ScriptResult<()> {
        installable.install(self)
    }

    /// Gets the AngelScript library version string.
    ///
    /// # Returns
    /// A string containing the AngelScript library version
    pub fn get_library_version() -> &'static str {
        unsafe {
            let version_ptr = asGetLibraryVersion();
            CStr::from_ptr(version_ptr).to_str().unwrap_or("Unknown")
        }
    }

    /// Gets the AngelScript library compilation options.
    ///
    /// # Returns
    /// A string containing the compilation options used when building AngelScript
    pub fn get_library_options() -> &'static str {
        unsafe {
            let options_ptr = asGetLibraryOptions();
            CStr::from_ptr(options_ptr).to_str().unwrap_or("Unknown")
        }
    }

    // ========== CONTEXT MANAGEMENT ==========

    /// Gets the currently active script context.
    ///
    /// # Returns
    /// The active context, or None if no context is currently active
    pub fn get_active_context() -> Option<Context> {
        unsafe {
            let context_ptr = asGetActiveContext();
            if context_ptr.is_null() {
                None
            } else {
                Some(Context::from_raw(context_ptr))
            }
        }
    }

    // ========== THREADING SUPPORT ==========

    /// Prepares AngelScript for multithreaded use.
    ///
    /// The implementation used depends on the compile-time feature:
    /// - Default: Uses AngelScript's built-in C++ thread manager
    /// - `rust-threads`: Uses a pure Rust implementation
    ///
    /// # Returns
    /// A ThreadManager instance for managing thread synchronization
    pub fn prepare_multithread() -> ScriptResult<ThreadManager> {
        ThreadManager::prepare()
    }

    /// Unprepares AngelScript from multithreaded use.
    ///
    /// This should be called when shutting down multithreaded operations.
    pub fn unprepare_multithread() {
        ThreadManager::unprepare()
    }

    /// Gets the current thread manager.
    ///
    /// # Returns
    /// The current ThreadManager, or None if not prepared for multithreading
    pub fn get_thread_manager() -> Option<ThreadManager> {
        unsafe {
            let mgr_ptr = asGetThreadManager();
            if mgr_ptr.is_null() {
                None
            } else {
                Some(ThreadManager::from_raw(mgr_ptr))
            }
        }
    }

    // ========== THREAD SYNCHRONIZATION ==========

    /// Acquires an exclusive lock for thread synchronization.
    ///
    /// This should be paired with [`release_exclusive_lock`] or use
    /// [`exclusive_lock`] for RAII-style locking.
    pub fn acquire_exclusive_lock() {
        ThreadManager::acquire_exclusive_lock()
    }

    /// Releases an exclusive lock.
    pub fn release_exclusive_lock() {
        ThreadManager::release_exclusive_lock()
    }

    /// Acquires a shared lock for thread synchronization.
    ///
    /// This should be paired with [`release_shared_lock`] or use
    /// [`shared_lock`] for RAII-style locking.
    pub fn acquire_shared_lock() {
        ThreadManager::acquire_shared_lock()
    }

    /// Releases a shared lock.
    pub fn release_shared_lock() {
        ThreadManager::release_shared_lock()
    }

    /// Creates an exclusive lock guard for RAII locking.
    ///
    /// The lock is automatically released when the guard is dropped.
    ///
    /// # Returns
    /// An exclusive lock guard
    pub fn exclusive_lock() -> ExclusiveLockGuard {
        ExclusiveLockGuard::new()
    }

    /// Creates a shared lock guard for RAII locking.
    ///
    /// The lock is automatically released when the guard is dropped.
    ///
    /// # Returns
    /// A shared lock guard
    pub fn shared_lock() -> SharedLockGuard {
        SharedLockGuard::new()
    }

    // ========== ATOMIC OPERATIONS ==========

    /// Atomically increments an integer value.
    ///
    /// # Arguments
    /// * `value` - The value to increment
    ///
    /// # Returns
    /// The new value after incrementing
    pub fn atomic_inc(value: &mut i32) -> i32 {
        unsafe { asAtomicInc(value as *mut i32) }
    }

    /// Atomically decrements an integer value.
    ///
    /// # Arguments
    /// * `value` - The value to decrement
    ///
    /// # Returns
    /// The new value after decrementing
    pub fn atomic_dec(value: &mut i32) -> i32 {
        unsafe { asAtomicDec(value as *mut i32) }
    }

    // ========== THREAD CLEANUP ==========

    /// Performs thread-specific cleanup.
    ///
    /// This should be called when a thread that has used AngelScript is about to terminate.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn thread_cleanup() -> ScriptResult<()> {
        ThreadManager::cleanup_local_data()
    }

    // ========== MEMORY MANAGEMENT ==========

    /// Sets custom global memory allocation functions.
    ///
    /// # Arguments
    /// * `alloc_func` - Custom allocation function
    /// * `free_func` - Custom deallocation function
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_global_memory_functions(
        alloc_func: asALLOCFUNC_t,
        free_func: asFREEFUNC_t,
    ) -> ScriptResult<()> {
        unsafe { ScriptError::from_code(asSetGlobalMemoryFunctions(alloc_func, free_func)) }
    }

    /// Resets global memory functions to default.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn reset_global_memory_functions() -> ScriptResult<()> {
        unsafe { ScriptError::from_code(asResetGlobalMemoryFunctions()) }
    }

    /// Allocates memory using AngelScript's allocator.
    ///
    /// # Arguments
    /// * `size` - The number of bytes to allocate
    ///
    /// # Returns
    /// A memory location wrapper
    pub fn alloc_mem(size: usize) -> ScriptMemoryLocation {
        unsafe { ScriptMemoryLocation::from_mut(asAllocMem(size)) }
    }

    /// Frees memory allocated by AngelScript's allocator.
    ///
    /// # Arguments
    /// * `mem` - The memory location to free
    pub fn free_mem(mut mem: ScriptMemoryLocation) {
        unsafe {
            asFreeMem(mem.as_mut_ptr());
        }
    }

    // ========== UTILITY OBJECTS ==========

    /// Creates a new lockable shared boolean.
    ///
    /// This is useful for implementing weak references and other
    /// thread-safe boolean flags.
    ///
    /// # Returns
    /// A new lockable shared boolean, or None if creation failed
    pub fn create_lockable_shared_bool() -> Option<LockableSharedBool> {
        unsafe {
            let ptr = asCreateLockableSharedBool();
            if ptr.is_null() {
                None
            } else {
                Some(LockableSharedBool::from_raw(ptr))
            }
        }
    }

    // ========== CONVENIENCE METHODS ==========

    /// Executes a closure with an exclusive lock held.
    ///
    /// This is a convenience method that automatically acquires and releases
    /// an exclusive lock around the closure execution.
    ///
    /// # Arguments
    /// * `f` - The closure to execute
    ///
    /// # Returns
    /// The return value of the closure
    pub fn with_exclusive_lock<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = Self::exclusive_lock();
        f()
    }

    /// Executes a closure with a shared lock held.
    ///
    /// This is a convenience method that automatically acquires and releases
    /// a shared lock around the closure execution.
    ///
    /// # Arguments
    /// * `f` - The closure to execute
    ///
    /// # Returns
    /// The return value of the closure
    pub fn with_shared_lock<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = Self::shared_lock();
        f()
    }

    /// Checks if AngelScript is prepared for multithreading.
    ///
    /// # Returns
    /// true if multithreading is prepared, false otherwise
    pub fn is_multithreading_prepared() -> bool {
        Self::get_thread_manager().is_some()
    }

    /// Gets information about the current threading setup.
    ///
    /// # Returns
    /// A string describing the current threading configuration
    pub fn get_threading_info() -> String {
        match Self::get_thread_manager() {
            Some(manager) => manager.info(),
            None => "No thread manager (single-threaded mode)".to_string(),
        }
    }

    /// Creates an Engine wrapper from a raw AngelScript engine pointer.
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid and the engine is properly initialized.
    ///
    /// # Arguments
    /// * `engine` - Raw pointer to AngelScript engine
    ///
    /// # Returns
    /// A new Engine wrapper
    pub(crate) fn from_raw(engine: NonNull<asIScriptEngine>) -> Engine {
        let engine_wrapper = Engine {
            inner: engine,
            is_root: false,
            phantom_data: PhantomData,
        };
        engine_wrapper
            .add_ref()
            .expect("Failed to add ref to engine");
        engine_wrapper
    }

    // Reference counting (following bindings order)

    /// Increments the reference count of the engine.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn add_ref(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_AddRef)(
                self.inner.as_ptr(),
            ))
        }
    }

    /// Decrements the reference count of the engine.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn release(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_Release)(
                self.inner.as_ptr(),
            ))
        }
    }

    /// Shuts down and releases the engine.
    ///
    /// This should be called when the engine is no longer needed.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn shutdown_and_release(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_ShutDownAndRelease)(
                self.inner.as_ptr(),
            ))
        }
    }

    // Engine properties

    /// Sets an engine property.
    ///
    /// # Arguments
    /// * `property` - The property to set
    /// * `value` - The value to set
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_engine_property(&self, property: EngineProperty, value: usize) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_SetEngineProperty)(
                self.inner.as_ptr(),
                property.into(),
                value,
            ))
        }
    }

    /// Gets an engine property value.
    ///
    /// # Arguments
    /// * `property` - The property to get
    ///
    /// # Returns
    /// The property value
    pub fn get_engine_property(&self, property: EngineProperty) -> asPWORD {
        unsafe {
            (self.as_vtable().asIScriptEngine_GetEngineProperty)(
                self.inner.as_ptr(),
                property.into(),
            )
        }
    }

    /// Sets a message callback for compilation and runtime messages.
    ///
    /// # Arguments
    /// * `callback` - The callback function to set
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_message_callback(
        &mut self,
        callback: MessageCallbackFn,
        data: Option<&mut dyn ScriptData>,
    ) -> ScriptResult<()> {
        CallbackManager::set_message_callback(Some(callback))?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_SetMessageCallback)(
                self.inner.as_ptr(),
                &mut asMessageInfoFunction(Some(CallbackManager::cvoid_msg_callback)),
                data.map(|v| v.to_script_ptr())
                    .unwrap_or_else(|| std::ptr::null_mut()),
                CallingConvention::Cdecl.into(),
            ))
        }
    }

    /// Clears the message callback.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn clear_message_callback(&mut self) -> ScriptResult<()> {
        CallbackManager::set_message_callback(None)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_ClearMessageCallback)(
                self.inner.as_ptr(),
            ))
        }
    }

    /// Writes a message to the message callback.
    ///
    /// # Arguments
    /// * `section` - The section name where the message originated
    /// * `row` - The row number
    /// * `col` - The column number
    /// * `msg_type` - The type of message
    /// * `message` - The message text
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn write_message(
        &self,
        section: &str,
        row: i32,
        col: i32,
        msg_type: MessageType,
        message: &str,
    ) -> ScriptResult<()> {
        let c_section = CString::new(section)?;
        let c_message = CString::new(message)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_WriteMessage)(
                self.inner.as_ptr(),
                c_section.as_ptr(),
                row,
                col,
                msg_type.into(),
                c_message.as_ptr(),
            ))
        }
    }

    // JIT compiler

    /// Sets a JIT compiler for the engine.
    ///
    /// # Arguments
    /// * `compiler` - The JIT compiler to set
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_jit_compiler(&self, compiler: &mut JITCompiler) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_SetJITCompiler)(
                self.inner.as_ptr(),
                compiler.as_ptr(),
            ))
        }
    }

    /// Gets the current JIT compiler.
    ///
    /// # Returns
    /// The current JIT compiler, or None if none is set
    pub fn get_jit_compiler(&self) -> Option<JITCompiler> {
        unsafe {
            let compiler = (self.as_vtable().asIScriptEngine_GetJITCompiler)(self.inner.as_ptr());
            if compiler.is_null() {
                None
            } else {
                Some(JITCompiler::from_raw(compiler))
            }
        }
    }

    // Global functions

    /// Internal thunk function for generic callbacks.
    ///
    /// # Safety
    /// This function is called by AngelScript and must handle the generic interface correctly.
    unsafe extern "C" fn generic_callback_thunk(arg1: *mut asIScriptGeneric) {
        let func = ScriptGeneric::from_raw(arg1).get_function();
        let user_data = func.get_user_data::<GenericFnUserData>();

        if let Some(f) = user_data {
            let s = ScriptGeneric::from_raw(arg1);
            f.call(&s);
        }
    }

    /// Registers a global function with the engine.
    ///
    /// # Arguments
    /// * `declaration` - The function declaration string
    /// * `func_ptr` - The function pointer to register
    /// * `auxiliary` - Optional auxiliary data
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// engine.register_global_function(
    ///     "void print(const string &in)",
    ///     |ctx: &ScriptGeneric| {
    ///         let text: String = ctx.get_arg(0);
    ///         println!("{}", text);
    ///     },
    ///     None
    /// )?;
    /// ```
    pub fn register_global_function(
        &self,
        declaration: &str,
        func_ptr: GenericFn,
        auxiliary: Option<Box<dyn ScriptData>>,
    ) -> ScriptResult<()> {
        let c_decl = CString::new(declaration)?;

        let base_func: asGENFUNC_t = Some(Engine::generic_callback_thunk);

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RegisterGlobalFunction)(
                self.inner.as_ptr(),
                c_decl.as_ptr(),
                &mut asGenericFunction(base_func),
                CallingConvention::Generic.into(),
                auxiliary
                    .map(|mut aux| aux.to_script_ptr())
                    .unwrap_or_else(std::ptr::null_mut),
            ))
        }?;

        if let Some(func_obj) = self.get_global_function_by_decl(declaration) {
            let user_data = Box::new(GenericFnUserData(func_ptr));
            func_obj.set_user_data(Box::leak(user_data));
        }

        Ok(())
    }

    /// Gets the number of registered global functions.
    ///
    /// # Returns
    /// The number of global functions
    pub fn get_global_function_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetGlobalFunctionCount)(self.inner.as_ptr()) }
    }

    /// Gets a global function by index.
    ///
    /// # Arguments
    /// * `index` - The function index
    ///
    /// # Returns
    /// The function, or None if the index is invalid
    pub fn get_global_function_by_index(&self, index: u32) -> Option<Function> {
        unsafe {
            let func = (self.as_vtable().asIScriptEngine_GetGlobalFunctionByIndex)(
                self.inner.as_ptr(),
                index,
            );
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    /// Gets a global function by declaration.
    ///
    /// # Arguments
    /// * `decl` - The function declaration
    ///
    /// # Returns
    /// The function, or None if not found
    pub fn get_global_function_by_decl(&self, decl: &str) -> Option<Function> {
        let c_decl = CString::new(decl).ok()?;

        unsafe {
            let func = (self.as_vtable().asIScriptEngine_GetGlobalFunctionByDecl)(
                self.inner.as_ptr(),
                c_decl.as_ptr(),
            );
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    // Global properties

    /// Registers a global property with the engine.
    ///
    /// # Arguments
    /// * `declaration` - The property declaration string
    /// * `pointer` - Pointer to the property data
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn register_global_property(
        &self,
        declaration: &str,
        mut pointer: Box<dyn ScriptData>,
    ) -> ScriptResult<()> {
        let c_decl = CString::new(declaration)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RegisterGlobalProperty)(
                self.inner.as_ptr(),
                c_decl.as_ptr(),
                pointer.to_script_ptr(),
            ))
        }
    }

    /// Gets the number of registered global properties.
    ///
    /// # Returns
    /// The number of global properties
    pub fn get_global_property_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetGlobalPropertyCount)(self.inner.as_ptr()) }
    }

    /// Gets information about a global property by index.
    ///
    /// # Arguments
    /// * `index` - The property index
    ///
    /// # Returns
    /// Property information, or an error if the index is invalid
    pub fn get_global_property_by_index(&self, index: asUINT) -> ScriptResult<GlobalPropertyInfo> {
        let mut name: *const c_char = ptr::null();
        let mut name_space: *const c_char = ptr::null();
        let mut type_id: i32 = 0;
        let mut is_const: bool = false;
        let mut config_group: *const c_char = ptr::null();
        let mut pointer: *mut c_void = ptr::null_mut();
        let mut access_mask: asDWORD = 0;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_GetGlobalPropertyByIndex)(
                self.inner.as_ptr(),
                index,
                &mut name,
                &mut name_space,
                &mut type_id,
                &mut is_const,
                &mut config_group,
                &mut pointer,
                &mut access_mask,
            ))?;

            Ok(GlobalPropertyInfo {
                name: if name.is_null() {
                    None
                } else {
                    CStr::from_ptr(name).to_str().ok().map(|s| s.to_string())
                },
                name_space: if name_space.is_null() {
                    None
                } else {
                    CStr::from_ptr(name_space)
                        .to_str()
                        .ok()
                        .map(|s| s.to_string())
                },
                type_id,
                is_const,
                config_group: if config_group.is_null() {
                    None
                } else {
                    CStr::from_ptr(config_group)
                        .to_str()
                        .ok()
                        .map(|s| s.to_string())
                },
                pointer: ScriptMemoryLocation::from_mut(pointer),
                access_mask,
            })
        }
    }

    /// Gets the index of a global property by name.
    ///
    /// # Arguments
    /// * `name` - The property name
    ///
    /// # Returns
    /// The property index, or an error if not found
    pub fn get_global_property_index_by_name(&self, name: &str) -> ScriptResult<i32> {
        let c_name = CString::new(name)?;

        unsafe {
            let index = (self
                .as_vtable()
                .asIScriptEngine_GetGlobalPropertyIndexByName)(
                self.inner.as_ptr(),
                c_name.as_ptr(),
            );
            if index < 0 {
                ScriptError::from_code(index)?;
            }
            Ok(index)
        }
    }

    /// Gets the index of a global property by declaration.
    ///
    /// # Arguments
    /// * `decl` - The property declaration
    ///
    /// # Returns
    /// The property index, or an error if not found
    pub fn get_global_property_index_by_decl(&self, decl: &str) -> ScriptResult<i32> {
        let c_decl = CString::new(decl)?;

        unsafe {
            let index = (self
                .as_vtable()
                .asIScriptEngine_GetGlobalPropertyIndexByDecl)(
                self.inner.as_ptr(),
                c_decl.as_ptr(),
            );
            if index < 0 {
                ScriptError::from_code(index)?;
            }
            Ok(index)
        }
    }

    // Object types

    /// Registers an object type with the engine.
    ///
    /// # Arguments
    /// * `name` - The type name
    /// * `byte_size` - The size of the type in bytes
    /// * `flags` - Object type flags
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn register_object_type(
        &self,
        name: &str,
        byte_size: i32,
        flags: ObjectTypeFlags,
    ) -> ScriptResult<()> {
        let c_name = CString::new(name)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RegisterObjectType)(
                self.inner.as_ptr(),
                c_name.as_ptr(),
                byte_size,
                flags.into(),
            ))
        }
    }

    /// Registers a property for an object type.
    ///
    /// # Arguments
    /// * `obj` - The object type name
    /// * `declaration` - The property declaration
    /// * `byte_offset` - Byte offset of the property in the object
    /// * `composite_offset` - Composite offset for complex types
    /// * `is_composite_indirect` - Whether the composite is indirect
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn register_object_property(
        &self,
        obj: &str,
        declaration: &str,
        byte_offset: i32,
        composite_offset: i32,
        is_composite_indirect: bool,
    ) -> ScriptResult<()> {
        let c_obj = CString::new(obj)?;
        let c_decl = CString::new(declaration)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RegisterObjectProperty)(
                self.inner.as_ptr(),
                c_obj.as_ptr(),
                c_decl.as_ptr(),
                byte_offset,
                composite_offset,
                is_composite_indirect,
            ))
        }
    }

    /// Registers a method for an object type.
    ///
    /// # Arguments
    /// * `obj` - The object type name
    /// * `declaration` - The method declaration
    /// * `func_ptr` - The function pointer
    /// * `auxiliary` - Optional auxiliary data
    /// * `composite_offset` - Optional composite offset
    /// * `is_composite_indirect` - Optional composite indirect flag
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn register_object_method(
        &self,
        obj: &str,
        declaration: &str,
        func_ptr: GenericFn,
        auxiliary: Option<&mut Box<dyn ScriptData>>,
        composite_offset: Option<i32>,
        is_composite_indirect: Option<bool>,
    ) -> ScriptResult<()> {
        let c_obj = CString::new(obj)?;
        let c_decl = CString::new(declaration)?;

        let base_func: asGENFUNC_t = Some(Engine::generic_callback_thunk);

        let func_id = unsafe {
            let result = (self.as_vtable().asIScriptEngine_RegisterObjectMethod)(
                self.inner.as_ptr(),
                c_obj.as_ptr(),
                c_decl.as_ptr(),
                &mut asGenericFunction(base_func),
                CallingConvention::Generic.into(),
                auxiliary
                    .map(|aux| aux.to_script_ptr())
                    .unwrap_or_else(std::ptr::null_mut),
                composite_offset.unwrap_or(0),
                is_composite_indirect.unwrap_or(false),
            );

            ScriptError::from_code(result).map(|_| result)
        }?;

        if let Some(func_obj) = self.get_function_by_id(func_id) {
            let user_data = Box::new(GenericFnUserData(func_ptr));
            func_obj.set_user_data(Box::leak(user_data));
        }

        Ok(())
    }

    /// Registers a behaviour for an object type.
    ///
    /// # Arguments
    /// * `obj` - The object type name
    /// * `behaviour` - The behaviour type
    /// * `declaration` - The behaviour declaration
    /// * `func_ptr` - The function pointer
    /// * `auxiliary` - Optional auxiliary data
    /// * `composite_offset` - Optional composite offset
    /// * `is_composite_indirect` - Optional composite indirect flag
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn register_object_behaviour(
        &self,
        obj: &str,
        behaviour: Behaviour,
        declaration: &str,
        func_ptr: GenericFn,
        auxiliary: Option<&mut Box<dyn ScriptData>>,
        composite_offset: Option<i32>,
        is_composite_indirect: Option<bool>,
    ) -> ScriptResult<()> {
        let c_obj = CString::new(obj)?;
        let c_decl = CString::new(declaration)?;

        let base_func: asGENFUNC_t = Some(Engine::generic_callback_thunk);

        let func_id = unsafe {
            let result = (self.as_vtable().asIScriptEngine_RegisterObjectBehaviour)(
                self.inner.as_ptr(),
                c_obj.as_ptr(),
                behaviour.into(),
                c_decl.as_ptr(),
                &mut asGenericFunction(base_func),
                asECallConvTypes_asCALL_GENERIC,
                auxiliary.map_or_else(ptr::null_mut, |aux| aux.to_script_ptr()),
                composite_offset.unwrap_or(0),
                is_composite_indirect.unwrap_or(false),
            );

            ScriptError::from_code(result).map(|_| result)
        }?;

        if let Some(func_obj) = self.get_function_by_id(func_id) {
            let user_data = Box::new(GenericFnUserData(func_ptr));
            func_obj.set_user_data(Box::leak(user_data));
        }

        Ok(())
    }

    // Interfaces

    /// Registers an interface with the engine.
    ///
    /// # Arguments
    /// * `name` - The interface name
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn register_interface(&self, name: &str) -> ScriptResult<()> {
        let c_name = CString::new(name)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RegisterInterface)(
                self.inner.as_ptr(),
                c_name.as_ptr(),
            ))
        }
    }

    /// Registers a method for an interface.
    ///
    /// # Arguments
    /// * `intf` - The interface name
    /// * `declaration` - The method declaration
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn register_interface_method(&self, intf: &str, declaration: &str) -> ScriptResult<()> {
        let c_intf = CString::new(intf)?;
        let c_decl = CString::new(declaration)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RegisterInterfaceMethod)(
                self.inner.as_ptr(),
                c_intf.as_ptr(),
                c_decl.as_ptr(),
            ))
        }
    }

    /// Gets the number of registered object types.
    ///
    /// # Returns
    /// The number of object types
    pub fn get_object_type_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetObjectTypeCount)(self.inner.as_ptr()) }
    }

    /// Gets an object type by index.
    ///
    /// # Arguments
    /// * `index` - The type index
    ///
    /// # Returns
    /// The type info, or None if the index is invalid
    pub fn get_object_type_by_index(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let type_info =
                (self.as_vtable().asIScriptEngine_GetObjectTypeByIndex)(self.inner.as_ptr(), index);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // String factory

    /// Registers a string factory with the engine.
    ///
    /// # Arguments
    /// * `datatype` - The string datatype
    /// * `factory` - The string factory
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn register_string_factory(
        &self,
        datatype: &str,
        factory: &asIStringFactory,
    ) -> ScriptResult<()> {
        let c_datatype = CString::new(datatype)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RegisterStringFactory)(
                self.inner.as_ptr(),
                c_datatype.as_ptr(),
                factory as *const _ as *mut asIStringFactory,
            ))
        }
    }

    /// Gets the string factory return type ID.
    ///
    /// # Returns
    /// A tuple of (type_id, flags) or an error
    pub fn get_string_factory_return_type_id(&self) -> ScriptResult<(i32, asDWORD)> {
        let mut flags: asDWORD = 0;

        unsafe {
            let type_id = (self
                .as_vtable()
                .asIScriptEngine_GetStringFactoryReturnTypeId)(
                self.inner.as_ptr(), &mut flags
            );
            if type_id < 0 {
                ScriptError::from_code(type_id)?;
            }
            Ok((type_id, flags))
        }
    }

    // Default array

    /// Registers the default array type.
    ///
    /// # Arguments
    /// * `type_name` - The array type name
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn register_default_array_type(&self, type_name: &str) -> ScriptResult<()> {
        let c_type = CString::new(type_name)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RegisterDefaultArrayType)(
                self.inner.as_ptr(),
                c_type.as_ptr(),
            ))
        }
    }

    /// Gets the default array type ID.
    ///
    /// # Returns
    /// The type ID or an error
    pub fn get_default_array_type_id(&self) -> ScriptResult<i32> {
        unsafe {
            let type_id =
                (self.as_vtable().asIScriptEngine_GetDefaultArrayTypeId)(self.inner.as_ptr());
            if type_id < 0 {
                ScriptError::from_code(type_id)?;
            }
            Ok(type_id)
        }
    }

    // Enums

    /// Registers an enum type.
    ///
    /// # Arguments
    /// * `type_name` - The enum type name
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn register_enum(&self, type_name: &str) -> ScriptResult<()> {
        let c_type = CString::new(type_name)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RegisterEnum)(
                self.inner.as_ptr(),
                c_type.as_ptr(),
            ))
        }
    }

    /// Registers an enum value.
    ///
    /// # Arguments
    /// * `type_name` - The enum type name
    /// * `name` - The enum value name
    /// * `value` - The enum value
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn register_enum_value(&self, type_name: &str, name: &str, value: i32) -> ScriptResult<()> {
        let c_type = CString::new(type_name)?;
        let c_name = CString::new(name)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RegisterEnumValue)(
                self.inner.as_ptr(),
                c_type.as_ptr(),
                c_name.as_ptr(),
                value,
            ))
        }
    }

    /// Gets the number of registered enums.
    ///
    /// # Returns
    /// The number of enums
    pub fn get_enum_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetEnumCount)(self.inner.as_ptr()) }
    }

    /// Gets an enum by index.
    ///
    /// # Arguments
    /// * `index` - The enum index
    ///
    /// # Returns
    /// The enum type info, or None if the index is invalid
    pub fn get_enum_by_index(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let type_info =
                (self.as_vtable().asIScriptEngine_GetEnumByIndex)(self.inner.as_ptr(), index);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // Funcdefs

    /// Registers a function definition (funcdef).
    ///
    /// # Arguments
    /// * `decl` - The function definition declaration
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn register_funcdef(&self, decl: &str) -> ScriptResult<()> {
        let c_decl = CString::new(decl)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RegisterFuncdef)(
                self.inner.as_ptr(),
                c_decl.as_ptr(),
            ))
        }
    }

    /// Gets the number of registered funcdefs.
    ///
    /// # Returns
    /// The number of funcdefs
    pub fn get_funcdef_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetFuncdefCount)(self.inner.as_ptr()) }
    }

    /// Gets a funcdef by index.
    ///
    /// # Arguments
    /// * `index` - The funcdef index
    ///
    /// # Returns
    /// The funcdef type info, or None if the index is invalid
    pub fn get_funcdef_by_index(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let type_info =
                (self.as_vtable().asIScriptEngine_GetFuncdefByIndex)(self.inner.as_ptr(), index);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // Typedefs

    /// Registers a typedef.
    ///
    /// # Arguments
    /// * `type_name` - The typedef name
    /// * `decl` - The typedef declaration
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn register_typedef(&self, type_name: &str, decl: &str) -> ScriptResult<()> {
        let c_type = CString::new(type_name)?;
        let c_decl = CString::new(decl)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RegisterTypedef)(
                self.inner.as_ptr(),
                c_type.as_ptr(),
                c_decl.as_ptr(),
            ))
        }
    }

    /// Gets the number of registered typedefs.
    ///
    /// # Returns
    /// The number of typedefs
    pub fn get_typedef_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetTypedefCount)(self.inner.as_ptr()) }
    }

    /// Gets a typedef by index.
    ///
    /// # Arguments
    /// * `index` - The typedef index
    ///
    /// # Returns
    /// The typedef type info, or None if the index is invalid
    pub fn get_typedef_by_index(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let type_info =
                (self.as_vtable().asIScriptEngine_GetTypedefByIndex)(self.inner.as_ptr(), index);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // Configuration groups

    /// Begins a configuration group.
    ///
    /// Configuration groups allow you to remove related registrations together.
    ///
    /// # Arguments
    /// * `group_name` - The group name
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn begin_config_group(&self, group_name: &str) -> ScriptResult<()> {
        let c_group = CString::new(group_name)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_BeginConfigGroup)(
                self.inner.as_ptr(),
                c_group.as_ptr(),
            ))
        }
    }

    /// Ends the current configuration group.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn end_config_group(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_EndConfigGroup)(
                self.inner.as_ptr(),
            ))
        }
    }

    /// Removes a configuration group and all its registrations.
    ///
    /// # Arguments
    /// * `group_name` - The group name to remove
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn remove_config_group(&self, group_name: &str) -> ScriptResult<()> {
        let c_group = CString::new(group_name)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RemoveConfigGroup)(
                self.inner.as_ptr(),
                c_group.as_ptr(),
            ))
        }
    }

    // Access control

    /// Sets the default access mask for registrations.
    ///
    /// # Arguments
    /// * `default_mask` - The default access mask
    ///
    /// # Returns
    /// The previous default access mask
    pub fn set_default_access_mask(&self, default_mask: asDWORD) -> asDWORD {
        unsafe {
            (self.as_vtable().asIScriptEngine_SetDefaultAccessMask)(
                self.inner.as_ptr(),
                default_mask,
            )
        }
    }

    // Namespaces

    /// Sets the default namespace for registrations.
    ///
    /// # Arguments
    /// * `name_space` - The namespace name
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_default_namespace(&self, name_space: &str) -> ScriptResult<()> {
        let c_namespace = CString::new(name_space)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_SetDefaultNamespace)(
                self.inner.as_ptr(),
                c_namespace.as_ptr(),
            ))
        }
    }

    /// Gets the current default namespace.
    ///
    /// # Returns
    /// The current default namespace, or None if not set
    pub fn get_default_namespace(&self) -> Option<&str> {
        unsafe {
            let namespace =
                (self.as_vtable().asIScriptEngine_GetDefaultNamespace)(self.inner.as_ptr());
            if namespace.is_null() {
                None
            } else {
                CStr::from_ptr(namespace).to_str().ok()
            }
        }
    }

    // Modules

    /// Gets or creates a module.
    ///
    /// # Arguments
    /// * `name` - The module name
    /// * `flag` - Flags controlling module creation
    ///
    /// # Returns
    /// The module or an error
    pub fn get_module(&self, name: &str, flag: GetModuleFlags) -> ScriptResult<Module> {
        let c_name = CString::new(name)?;

        unsafe {
            let module = (self.as_vtable().asIScriptEngine_GetModule)(
                self.inner.as_ptr(),
                c_name.as_ptr(),
                flag.into(),
            );
            if module.is_null() {
                Err(ScriptError::NullPointer)
            } else {
                Ok(Module::from_raw(module))
            }
        }
    }

    /// Discards a module.
    ///
    /// # Arguments
    /// * `name` - The module name to discard
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn discard_module(&self, name: &str) -> ScriptResult<()> {
        let c_name = CString::new(name)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_DiscardModule)(
                self.inner.as_ptr(),
                c_name.as_ptr(),
            ))
        }
    }

    /// Gets the number of modules.
    ///
    /// # Returns
    /// The number of modules
    pub fn get_module_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetModuleCount)(self.inner.as_ptr()) }
    }

    /// Gets a module by index.
    ///
    /// # Arguments
    /// * `index` - The module index
    ///
    /// # Returns
    /// The module, or None if the index is invalid
    pub fn get_module_by_index(&self, index: asUINT) -> Option<Module> {
        unsafe {
            let module =
                (self.as_vtable().asIScriptEngine_GetModuleByIndex)(self.inner.as_ptr(), index);
            if module.is_null() {
                None
            } else {
                Some(Module::from_raw(module))
            }
        }
    }

    // Functions

    /// Gets the ID of the last registered function.
    ///
    /// # Returns
    /// The function ID
    pub fn get_last_function_id(&self) -> i32 {
        unsafe { (self.as_vtable().asIScriptEngine_GetLastFunctionId)(self.inner.as_ptr()) }
    }

    /// Gets a function by its ID.
    ///
    /// # Arguments
    /// * `func_id` - The function ID
    ///
    /// # Returns
    /// The function, or None if the ID is invalid
    pub fn get_function_by_id(&self, func_id: i32) -> Option<Function> {
        unsafe {
            let func_ptr =
                (self.as_vtable().asIScriptEngine_GetFunctionById)(self.inner.as_ptr(), func_id);
            if func_ptr.is_null() {
                None
            } else {
                Some(Function::from_raw(func_ptr))
            }
        }
    }

    // Type information

    /// Gets a type ID by declaration.
    ///
    /// # Arguments
    /// * `decl` - The type declaration
    ///
    /// # Returns
    /// The type ID or an error
    pub fn get_type_id_by_decl(&self, decl: &str) -> ScriptResult<i32> {
        let c_decl = CString::new(decl)?;

        unsafe {
            let type_id = (self.as_vtable().asIScriptEngine_GetTypeIdByDecl)(
                self.inner.as_ptr(),
                c_decl.as_ptr(),
            );
            if type_id < 0 {
                ScriptError::from_code(type_id)?;
            }
            Ok(type_id)
        }
    }

    /// Gets the declaration string for a type ID.
    ///
    /// # Arguments
    /// * `type_id` - The type ID
    /// * `include_namespace` - Whether to include the namespace
    ///
    /// # Returns
    /// The type declaration, or None if the type ID is invalid
    pub fn get_type_declaration(&self, type_id: i32, include_namespace: bool) -> Option<&str> {
        unsafe {
            let decl = (self.as_vtable().asIScriptEngine_GetTypeDeclaration)(
                self.inner.as_ptr(),
                type_id,
                include_namespace,
            );
            if decl.is_null() {
                None
            } else {
                CStr::from_ptr(decl).to_str().ok()
            }
        }
    }

    /// Gets the size of a primitive type.
    ///
    /// # Arguments
    /// * `type_id` - The type ID
    ///
    /// # Returns
    /// The size in bytes, or an error if the type is not primitive
    pub fn get_size_of_primitive_type(&self, type_id: i32) -> ScriptResult<i32> {
        unsafe {
            let size = (self.as_vtable().asIScriptEngine_GetSizeOfPrimitiveType)(
                self.inner.as_ptr(),
                type_id,
            );
            if size < 0 {
                ScriptError::from_code(size)?;
            }
            Ok(size)
        }
    }

    /// Gets type information by type ID.
    ///
    /// # Arguments
    /// * `type_id` - The type ID
    ///
    /// # Returns
    /// The type info, or None if the type ID is invalid
    pub fn get_type_info_by_id(&self, type_id: TypeId) -> Option<TypeInfo> {
        unsafe {
            let type_id: asETypeIdFlags = type_id.into();
            let type_info = (self.as_vtable().asIScriptEngine_GetTypeInfoById)(
                self.inner.as_ptr(),
                type_id as i32,
            );
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    /// Gets type information by name.
    ///
    /// # Arguments
    /// * `name` - The type name
    ///
    /// # Returns
    /// The type info, or None if not found
    pub fn get_type_info_by_name(&self, name: &str) -> Option<TypeInfo> {
        let c_name = CString::new(name).ok()?;

        unsafe {
            let type_info = (self.as_vtable().asIScriptEngine_GetTypeInfoByName)(
                self.inner.as_ptr(),
                c_name.as_ptr(),
            );
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    /// Gets type information by declaration.
    ///
    /// # Arguments
    /// * `decl` - The type declaration
    ///
    /// # Returns
    /// The type info, or None if not found
    pub fn get_type_info_by_decl(&self, decl: &str) -> Option<TypeInfo> {
        let c_decl = CString::new(decl).ok()?;

        unsafe {
            let type_info = (self.as_vtable().asIScriptEngine_GetTypeInfoByDecl)(
                self.inner.as_ptr(),
                c_decl.as_ptr(),
            );
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // Context creation

    /// Creates a new script context.
    ///
    /// # Returns
    /// A new context or an error
    pub fn create_context(&self) -> ScriptResult<Context> {
        unsafe {
            let ctx = (self.as_vtable().asIScriptEngine_CreateContext)(self.inner.as_ptr());
            if ctx.is_null() {
                Err(ScriptError::NullPointer)
            } else {
                Ok(Context::from_raw(ctx))
            }
        }
    }

    // Script objects

    /// Creates a new script object of the given type.
    ///
    /// # Arguments
    /// * `type_info` - The type information
    ///
    /// # Returns
    /// The created object or an error
    pub fn create_script_object<T: ScriptData>(&self, type_info: &TypeInfo) -> ScriptResult<T> {
        unsafe {
            let obj = (self.as_vtable().asIScriptEngine_CreateScriptObject)(
                self.inner.as_ptr(),
                type_info.as_ptr(),
            );
            if obj.is_null() {
                Err(ScriptError::NullPointer)
            } else {
                Ok(ScriptData::from_script_ptr(obj))
            }
        }
    }

    /// Creates a copy of a script object.
    ///
    /// # Arguments
    /// * `obj` - The object to copy
    /// * `type_info` - The type information
    ///
    /// # Returns
    /// The copied object, or None if copying failed
    pub fn create_script_object_copy<T: ScriptData>(
        &self,
        obj: &mut T,
        type_info: &TypeInfo,
    ) -> Option<T> {
        unsafe {
            let new_obj = (self.as_vtable().asIScriptEngine_CreateScriptObjectCopy)(
                self.inner.as_ptr(),
                obj.to_script_ptr(),
                type_info.as_ptr(),
            );
            if new_obj.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(new_obj))
            }
        }
    }

    /// Creates an uninitialized script object.
    ///
    /// # Arguments
    /// * `type_info` - The type information
    ///
    /// # Returns
    /// The uninitialized object memory, or None if creation failed
    pub fn create_uninitialized_script_object(
        &self,
        type_info: &TypeInfo,
    ) -> Option<ScriptMemoryLocation> {
        unsafe {
            let obj = (self
                .as_vtable()
                .asIScriptEngine_CreateUninitializedScriptObject)(
                self.inner.as_ptr(),
                type_info.as_ptr(),
            );
            if obj.is_null() {
                None
            } else {
                Some(ScriptMemoryLocation::from_mut(obj))
            }
        }
    }

    /// Creates a delegate function.
    ///
    /// # Arguments
    /// * `func` - The function to create a delegate for
    /// * `obj` - The object to bind to the delegate
    ///
    /// # Returns
    /// The delegate function, or None if creation failed
    pub fn create_delegate<T: ScriptData>(&self, func: &Function, obj: &mut T) -> Option<Function> {
        unsafe {
            let delegate = (self.as_vtable().asIScriptEngine_CreateDelegate)(
                self.inner.as_ptr(),
                func.as_raw(),
                obj.to_script_ptr(),
            );
            if delegate.is_null() {
                None
            } else {
                Some(Function::from_raw(delegate))
            }
        }
    }

    /// Assigns a script object to another.
    ///
    /// # Arguments
    /// * `src_obj` - The source object
    /// * `type_info` - The type information
    ///
    /// # Returns
    /// The assigned object or an error
    pub fn assign_script_object<T: ScriptData>(
        &self,
        src_obj: &mut T,
        type_info: &TypeInfo,
    ) -> ScriptResult<T> {
        let ptr = std::ptr::null_mut();
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_AssignScriptObject)(
                self.inner.as_ptr(),
                ptr,
                src_obj.to_script_ptr(),
                type_info.as_ptr(),
            ))?;
        }
        Ok(ScriptData::from_script_ptr(ptr))
    }

    /// Releases a script object.
    ///
    /// # Arguments
    /// * `obj` - The object to release
    /// * `type_info` - The type information
    pub fn release_script_object<T: ScriptData>(&self, obj: &mut T, type_info: &TypeInfo) {
        unsafe {
            (self.as_vtable().asIScriptEngine_ReleaseScriptObject)(
                self.inner.as_ptr(),
                obj.to_script_ptr(),
                type_info.as_ptr(),
            );
        }
    }

    /// Adds a reference to a script object.
    ///
    /// # Arguments
    /// * `obj` - The object to add a reference to
    /// * `type_info` - The type information
    pub fn add_ref_script_object<T: ScriptData>(&self, obj: &mut T, type_info: &TypeInfo) {
        unsafe {
            (self.as_vtable().asIScriptEngine_AddRefScriptObject)(
                self.inner.as_ptr(),
                obj.to_script_ptr(),
                type_info.as_ptr(),
            );
        }
    }

    /// Performs a reference cast on an object.
    ///
    /// # Arguments
    /// * `obj` - The object to cast
    /// * `from_type` - The source type
    /// * `to_type` - The target type
    /// * `use_only_implicit_cast` - Whether to use only implicit casts
    ///
    /// # Returns
    /// The cast object, None if casting failed, or an error
    pub fn ref_cast_object<T: ScriptData, U: ScriptData>(
        &self,
        obj: &mut T,
        from_type: &mut TypeInfo,
        to_type: &mut TypeInfo,
        use_only_implicit_cast: bool,
    ) -> ScriptResult<Option<U>> {
        let mut new_ptr: *mut c_void = ptr::null_mut();

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RefCastObject)(
                self.inner.as_ptr(),
                obj.to_script_ptr(),
                from_type.as_ptr(),
                to_type.as_ptr(),
                &mut new_ptr,
                use_only_implicit_cast,
            ))?;

            if new_ptr.is_null() {
                Ok(None)
            } else {
                Ok(Some(ScriptData::from_script_ptr(new_ptr)))
            }
        }
    }

    /// Gets the weak reference flag of a script object.
    ///
    /// # Arguments
    /// * `obj` - The object
    /// * `type_info` - The type information
    ///
    /// # Returns
    /// The weak reference flag, or None if not available
    pub fn get_weak_ref_flag_of_script_object<T: ScriptData>(
        &self,
        obj: &mut T,
        type_info: &TypeInfo,
    ) -> Option<LockableSharedBool> {
        unsafe {
            let flag = (self
                .as_vtable()
                .asIScriptEngine_GetWeakRefFlagOfScriptObject)(
                self.inner.as_ptr(),
                obj.to_script_ptr(),
                type_info.as_ptr(),
            );
            if flag.is_null() {
                None
            } else {
                Some(LockableSharedBool::from_raw(flag))
            }
        }
    }

    // Context management

    /// Requests a context from the context pool.
    ///
    /// # Returns
    /// A context from the pool, or None if none available
    pub fn request_context(&self) -> Option<Context> {
        unsafe {
            let ctx = (self.as_vtable().asIScriptEngine_RequestContext)(self.inner.as_ptr());
            if ctx.is_null() {
                None
            } else {
                Some(Context::from_raw(ctx))
            }
        }
    }

    /// Returns a context to the context pool.
    ///
    /// # Arguments
    /// * `ctx` - The context to return
    pub fn return_context(&self, ctx: Context) {
        unsafe {
            (self.as_vtable().asIScriptEngine_ReturnContext)(self.inner.as_ptr(), ctx.as_ptr());
        }
    }

    /// Sets context callbacks for the context pool.
    ///
    /// # Arguments
    /// * `request_ctx` - Callback for requesting contexts
    /// * `return_ctx` - Callback for returning contexts
    /// * `param` - Parameter to pass to callbacks
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_context_callbacks<T: ScriptData>(
        &mut self,
        request_ctx: RequestContextCallbackFn,
        return_ctx: ReturnContextCallbackFn,
        param: &mut T,
    ) -> ScriptResult<()> {
        CallbackManager::set_request_context_callback(Some(request_ctx))?;
        CallbackManager::set_return_context_callback(Some(return_ctx))?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_SetContextCallbacks)(
                self.inner.as_ptr(),
                Some(CallbackManager::cvoid_request_context_callback),
                Some(CallbackManager::cvoid_return_context_callback),
                param.to_script_ptr(),
            ))
        }
    }

    // Parsing

    /// Parses a token from a string.
    ///
    /// # Arguments
    /// * `string` - The string to parse
    ///
    /// # Returns
    /// A tuple of (token_class, token_length)
    pub fn parse_token(&self, string: &str) -> (TokenClass, usize) {
        let c_string = string.as_bytes();
        let mut token_length: asUINT = 0;

        unsafe {
            let token_class = (self.as_vtable().asIScriptEngine_ParseToken)(
                self.inner.as_ptr(),
                c_string.as_ptr() as *const c_char,
                c_string.len(),
                &mut token_length,
            );
            (token_class.into(), token_length as usize)
        }
    }

    // Garbage collection

    /// Performs garbage collection.
    ///
    /// # Arguments
    /// * `flags` - Garbage collection flags
    /// * `num_iterations` - Number of iterations to perform
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn garbage_collect(&self, flags: asDWORD, num_iterations: asUINT) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_GarbageCollect)(
                self.inner.as_ptr(),
                flags,
                num_iterations,
            ))
        }
    }

    /// Gets garbage collection statistics.
    ///
    /// # Returns
    /// Garbage collection statistics
    pub fn get_gc_statistics(&self) -> GCStatistics {
        let mut current_size: asUINT = 0;
        let mut total_destroyed: asUINT = 0;
        let mut total_detected: asUINT = 0;
        let mut new_objects: asUINT = 0;
        let mut total_new_destroyed: asUINT = 0;

        unsafe {
            (self.as_vtable().asIScriptEngine_GetGCStatistics)(
                self.inner.as_ptr(),
                &mut current_size,
                &mut total_destroyed,
                &mut total_detected,
                &mut new_objects,
                &mut total_new_destroyed,
            );
        }

        GCStatistics {
            current_size,
            total_destroyed,
            total_detected,
            new_objects,
            total_new_destroyed,
        }
    }

    /// Notifies the garbage collector of a new object.
    ///
    /// # Arguments
    /// * `obj` - The new object
    /// * `type_info` - The type information
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn notify_garbage_collector_of_new_object<T: ScriptData>(
        &self,
        obj: &mut T,
        type_info: &mut TypeInfo,
    ) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self
                .as_vtable()
                .asIScriptEngine_NotifyGarbageCollectorOfNewObject)(
                self.inner.as_ptr(),
                obj.to_script_ptr(),
                type_info.as_ptr(),
            ))
        }
    }

    /// Gets information about an object in the garbage collector.
    ///
    /// # Arguments
    /// * `idx` - The object index
    ///
    /// # Returns
    /// Object information, or None if the index is invalid
    pub fn get_object_in_gc(&self, idx: asUINT) -> Option<GCObjectInfo> {
        let mut seq_nbr: asUINT = 0;
        let mut obj: *mut c_void = ptr::null_mut();
        let mut type_info: *mut asITypeInfo = ptr::null_mut();

        unsafe {
            let result = (self.as_vtable().asIScriptEngine_GetObjectInGC)(
                self.inner.as_ptr(),
                idx,
                &mut seq_nbr,
                &mut obj,
                &mut type_info,
            );

            if result < 0 || obj.is_null() {
                None
            } else {
                Some(GCObjectInfo {
                    seq_nbr,
                    obj: ScriptMemoryLocation::from_mut(obj),
                    type_info: if type_info.is_null() {
                        None
                    } else {
                        Some(TypeInfo::from_raw(type_info))
                    },
                })
            }
        }
    }

    /// Garbage collection enum callback.
    ///
    /// # Arguments
    /// * `reference` - The reference to enumerate
    pub fn gc_enum_callback<T: ScriptData>(&self, reference: &mut T) {
        unsafe {
            (self.as_vtable().asIScriptEngine_GCEnumCallback)(
                self.inner.as_ptr(),
                reference.to_script_ptr(),
            );
        }
    }

    /// Forwards garbage collection enum references.
    ///
    /// # Arguments
    /// * `ref_obj` - The reference object
    /// * `type_info` - The type information
    pub fn forward_gc_enum_references<T: ScriptData>(
        &self,
        ref_obj: &mut T,
        type_info: &mut TypeInfo,
    ) {
        unsafe {
            (self.as_vtable().asIScriptEngine_ForwardGCEnumReferences)(
                self.inner.as_ptr(),
                ref_obj.to_script_ptr(),
                type_info.as_ptr(),
            );
        }
    }

    /// Forwards garbage collection release references.
    ///
    /// # Arguments
    /// * `ref_obj` - The reference object
    /// * `type_info` - The type information
    pub fn forward_gc_release_references<T: ScriptData>(
        &self,
        ref_obj: &mut T,
        type_info: &mut TypeInfo,
    ) {
        unsafe {
            (self.as_vtable().asIScriptEngine_ForwardGCReleaseReferences)(
                self.inner.as_ptr(),
                ref_obj.to_script_ptr(),
                type_info.as_ptr(),
            );
        }
    }

    /// Sets a callback for when circular references are detected.
    ///
    /// # Arguments
    /// * `callback` - The callback function
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_circular_ref_detected_callback(
        &mut self,
        callback: CircularRefCallbackFn,
    ) -> ScriptResult<()> {
        CallbackManager::set_circular_ref_callback(Some(callback))?;

        unsafe {
            (self
                .as_vtable()
                .asIScriptEngine_SetCircularRefDetectedCallback)(
                self.inner.as_ptr(),
                Some(CallbackManager::cvoid_circular_ref_callback),
                ptr::null_mut(),
            );
        }

        Ok(())
    }

    // User data

    /// Sets user data on the engine.
    ///
    /// # Arguments
    /// * `data` - The user data to set
    ///
    /// # Returns
    /// The previous user data, or None if none was set
    pub fn set_user_data<T: UserData + ScriptData>(&self, data: &mut T) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptEngine_SetUserData)(
                self.inner.as_ptr(),
                data.to_script_ptr(),
                T::KEY as asPWORD,
            );
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    /// Gets user data from the engine.
    ///
    /// # Returns
    /// The user data or an error if not found
    pub fn get_user_data<T: UserData + ScriptData>(&self) -> ScriptResult<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptEngine_GetUserData)(
                self.inner.as_ptr(),
                T::KEY as asPWORD,
            );
            if ptr.is_null() {
                Err(ScriptError::NullPointer)
            } else {
                Ok(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    /// Sets a cleanup callback for engine user data.
    ///
    /// # Arguments
    /// * `callback` - The cleanup callback
    /// * `type_id` - The type ID for the user data
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_engine_user_data_cleanup_callback(
        &mut self,
        callback: CleanEngineUserDataCallbackFn,
        type_id: asPWORD,
    ) -> ScriptResult<()> {
        CallbackManager::add_engine_user_data_cleanup_callback(type_id, callback)?;

        unsafe {
            (self
                .as_vtable()
                .asIScriptEngine_SetEngineUserDataCleanupCallback)(
                self.inner.as_ptr(),
                Some(CallbackManager::cvoid_engine_user_data_cleanup_callback),
                type_id,
            );
        }

        Ok(())
    }

    /// Sets a cleanup callback for module user data.
    ///
    /// # Arguments
    /// * `callback` - The cleanup callback
    /// * `type_id` - The type ID for the user data
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_module_user_data_cleanup_callback(
        &mut self,
        callback: CleanModuleUserDataCallbackFn,
        type_id: asPWORD,
    ) -> ScriptResult<()> {
        CallbackManager::add_module_user_data_cleanup_callback(type_id, callback)?;
        unsafe {
            (self
                .as_vtable()
                .asIScriptEngine_SetModuleUserDataCleanupCallback)(
                self.inner.as_ptr(),
                Some(CallbackManager::cvoid_module_user_data_cleanup_callback),
                type_id,
            );
        }

        Ok(())
    }

    /// Sets a cleanup callback for context user data.
    ///
    /// # Arguments
    /// * `callback` - The cleanup callback
    /// * `type_id` - The type ID for the user data
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_context_user_data_cleanup_callback(
        &mut self,
        callback: CleanContextUserDataCallbackFn,
        type_id: asPWORD,
    ) -> ScriptResult<()> {
        CallbackManager::add_context_user_data_cleanup_callback(type_id, callback)?;

        unsafe {
            (self
                .as_vtable()
                .asIScriptEngine_SetContextUserDataCleanupCallback)(
                self.inner.as_ptr(),
                Some(CallbackManager::cvoid_context_user_data_cleanup_callback),
                type_id,
            );
        }

        Ok(())
    }

    /// Sets a cleanup callback for function user data.
    ///
    /// # Arguments
    /// * `callback` - The cleanup callback
    /// * `type_id` - The type ID for the user data
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_function_user_data_cleanup_callback(
        &mut self,
        callback: CleanFunctionUserDataCallbackFn,
        type_id: asPWORD,
    ) -> ScriptResult<()> {
        CallbackManager::add_function_user_data_cleanup_callback(type_id, callback)?;

        unsafe {
            (self
                .as_vtable()
                .asIScriptEngine_SetFunctionUserDataCleanupCallback)(
                self.inner.as_ptr(),
                Some(CallbackManager::cvoid_function_user_data_cleanup_callback),
                type_id,
            );
        }

        Ok(())
    }

    /// Sets a cleanup callback for type info user data.
    ///
    /// # Arguments
    /// * `callback` - The cleanup callback
    /// * `type_id` - The type ID for the user data
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_type_info_user_data_cleanup_callback(
        &mut self,
        callback: CleanTypeInfoCallbackFn,
        type_id: asPWORD,
    ) -> ScriptResult<()> {
        CallbackManager::add_type_info_user_data_cleanup_callback(type_id, callback)?;

        unsafe {
            (self
                .as_vtable()
                .asIScriptEngine_SetTypeInfoUserDataCleanupCallback)(
                self.inner.as_ptr(),
                Some(CallbackManager::cvoid_type_info_user_data_cleanup_callback),
                type_id,
            );
        }

        Ok(())
    }

    /// Sets a cleanup callback for script object user data.
    ///
    /// # Arguments
    /// * `callback` - The cleanup callback
    /// * `type_id` - The type ID for the user data
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_script_object_user_data_cleanup_callback(
        &mut self,
        callback: CleanScriptObjectCallbackFn,
        type_id: asPWORD,
    ) -> ScriptResult<()> {
        CallbackManager::add_script_object_user_data_cleanup_callback(type_id, callback)?;
        unsafe {
            (self
                .as_vtable()
                .asIScriptEngine_SetScriptObjectUserDataCleanupCallback)(
                self.inner.as_ptr(),
                Some(CallbackManager::cvoid_script_object_user_data_cleanup_callback),
                type_id,
            );
        }

        Ok(())
    }

    // Translate app exception callback

    /// Sets a callback for translating application exceptions.
    ///
    /// # Arguments
    /// * `callback` - The translation callback
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_translate_app_exception_callback(
        &mut self,
        callback: TranslateAppExceptionCallbackFn,
    ) -> ScriptResult<()> {
        CallbackManager::set_translate_exception_callback(Some(callback))?;

        let conv: u32 = CallingConvention::Cdecl.into();
        unsafe {
            ScriptError::from_code((self
                .as_vtable()
                .asIScriptEngine_SetTranslateAppExceptionCallback)(
                self.inner.as_ptr(),
                asScriptContextFunction(Some(CallbackManager::cvoid_translate_exception_callback)),
                ptr::null_mut(),
                conv as i32,
            ))
        }
    }

    /// Sets a diagnostic collector that will receive all compilation messages
    pub fn set_diagnostic_callback(&mut self, diagnostics: &mut Diagnostics) -> ScriptResult<()> {
        // Clear any existing diagnostics
        diagnostics.clear();

        let callback = |message_info: &MessageInfo, mem: &mut ScriptMemoryLocation| {
            let diagnostic = Diagnostic {
                kind: DiagnosticKind::from(message_info.msg_type),
                message: message_info.message.clone(),
                section: if message_info.section.is_empty() {
                    None
                } else {
                    Some(message_info.section.clone())
                },
                row: message_info.row,
                col: message_info.col,
            };

            mem.as_ref_mut::<Diagnostics>().add_diagnostic(diagnostic);
        };

        self.set_message_callback(callback, Some(diagnostics))
    }

    /// Clears the diagnostic callback (same as clearing message callback)
    pub fn clear_diagnostic_callback(&mut self) -> ScriptResult<()> {
        self.clear_message_callback()
    }

    /// Gets the vtable for the underlying AngelScript engine.
    ///
    /// # Safety
    /// This function provides direct access to the AngelScript vtable.
    fn as_vtable(&self) -> &asIScriptEngine__bindgen_vtable {
        unsafe { &*(*self.inner.as_ptr()).vtable_ }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        match self.is_root {
            true => {
                self.shutdown_and_release()
                    .expect("Failed to shutdown engine");
            }
            false => {
                self.release().expect("Failed to release engine");
            }
        };
    }
}

impl Clone for Engine {
    fn clone(&self) -> Self {
        Engine::from_raw(self.inner)
    }
}

unsafe impl Send for Engine {}
unsafe impl Sync for Engine {}

/// User data wrapper for generic function pointers.
///
/// This struct wraps a generic function pointer for storage as user data.
struct GenericFnUserData(pub GenericFn);

impl GenericFnUserData {
    /// Calls the wrapped function.
    ///
    /// # Arguments
    /// * `ctx` - The script context
    pub fn call(self, ctx: &ScriptGeneric) {
        (self.0)(ctx)
    }
}

unsafe impl Send for GenericFnUserData {}
unsafe impl Sync for GenericFnUserData {}

impl UserData for GenericFnUserData {
    const KEY: usize = 0x129032719; // Must be unique!
}

impl ScriptData for GenericFnUserData {
    fn to_script_ptr(&mut self) -> *mut Void {
        self as *const Self as *mut Void
    }

    fn from_script_ptr(ptr: *mut Void) -> Self
    where
        Self: Sized,
    {
        unsafe { ptr.cast::<Self>().read() }
    }
}

/// Information about a global property.
#[derive(Debug)]
pub struct GlobalPropertyInfo {
    /// The property name
    pub name: Option<String>,
    /// The namespace the property belongs to
    pub name_space: Option<String>,
    /// The type ID of the property
    pub type_id: i32,
    /// Whether the property is const
    pub is_const: bool,
    /// The configuration group the property belongs to
    pub config_group: Option<String>,
    /// Pointer to the property data
    pub pointer: ScriptMemoryLocation,
    /// Access mask for the property
    pub access_mask: asDWORD,
}

/// Garbage collection statistics.
#[derive(Debug)]
pub struct GCStatistics {
    /// Current number of objects in the garbage collector
    pub current_size: asUINT,
    /// Total number of objects destroyed
    pub total_destroyed: asUINT,
    /// Total number of circular references detected
    pub total_detected: asUINT,
    /// Number of new objects since last collection
    pub new_objects: asUINT,
    /// Total number of new objects destroyed
    pub total_new_destroyed: asUINT,
}

/// Information about an object in the garbage collector.
#[derive(Debug)]
pub struct GCObjectInfo {
    /// Sequence number of the object
    pub seq_nbr: asUINT,
    /// Pointer to the object
    pub obj: ScriptMemoryLocation,
    /// Type information for the object
    pub type_info: Option<TypeInfo>,
}
