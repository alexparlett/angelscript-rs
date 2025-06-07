use crate::core::context::Context;
use crate::core::error::{ScriptError, ScriptResult};
use crate::core::function::Function;
use crate::core::lockable_shared_bool::LockableSharedBool;
use crate::core::module::Module;
use crate::core::script_generic::ScriptGeneric;
use crate::core::typeinfo::TypeInfo;
use crate::internal::callback_manager::CallbackManager;
use crate::internal::jit_compiler::JITCompiler;
use crate::internal::stringfactory::get_string_factory_instance;
use crate::internal::thread_manager::{ExclusiveLockGuard, SharedLockGuard, ThreadManager};
use crate::plugins::plugin;
use crate::plugins::plugin::Plugin;
use crate::types::callbacks::{
    CircularRefCallbackFn, CleanContextUserDataCallbackFn, CleanEngineUserDataCallbackFn,
    CleanFunctionUserDataCallbackFn, CleanModuleUserDataCallbackFn, CleanScriptObjectCallbackFn,
    CleanTypeInfoCallbackFn, GenericFn, MessageCallbackFn, RequestContextCallbackFn,
    ReturnContextCallbackFn, TranslateAppExceptionCallbackFn,
};
use crate::types::enums::*;
use crate::types::script_data::ScriptData;
use crate::types::script_memory::ScriptMemoryLocation;
use crate::types::user_data::UserData;
use angelscript_sys::{
    asALLOCFUNC_t, asAllocMem, asAtomicDec, asAtomicInc, asCreateLockableSharedBool,
    asCreateScriptEngine, asDWORD, asECallConvTypes_asCALL_GENERIC, asEMsgType,
    asETypeIdFlags, asFREEFUNC_t, asFreeMem, asGENFUNC_t, asGenericFunction, asGetActiveContext,
    asGetLibraryOptions, asGetLibraryVersion, asGetThreadManager, asIScriptEngine,
    asIScriptEngine__bindgen_vtable, asIScriptGeneric, asIStringFactory, asITypeInfo,
    asMessageInfoFunction, asPWORD, asResetGlobalMemoryFunctions, asScriptContextFunction,
    asSetGlobalMemoryFunctions, asUINT, ANGELSCRIPT_VERSION,
};
use std::alloc::{alloc, Layout};
use std::ffi::{c_char, CStr, CString};
use std::marker::PhantomData;
use std::os::raw::c_void;
use std::ptr;
use std::ptr::NonNull;

#[derive(Debug, PartialEq, Eq)]
pub struct Engine {
    inner: NonNull<asIScriptEngine>,
    is_root: bool,
    phantom_data: PhantomData<asIScriptEngine>,
}

impl Engine {
    #[cfg(feature = "rust-alloc")]
    pub unsafe extern "C" fn unified_alloc(size: usize) -> *mut std::ffi::c_void { unsafe {
        let layout = Layout::from_size_align(size, 8).unwrap();
        alloc(layout) as *mut std::ffi::c_void
    }}

    #[cfg(feature = "rust-alloc")]
    pub unsafe extern "C" fn unified_free(ptr: *mut std::ffi::c_void) { unsafe {
        if !ptr.is_null() {
            libc::free(ptr);
        }
    }}

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

    pub fn install(&self, plugin: Plugin) -> ScriptResult<()> {
        let was_namespaced = if let Some(namespace) = plugin.namespace() {
            self.set_default_namespace(namespace)?;
            true
        } else {
            false
        };
        for registration in plugin.registrations {
            match registration {
                plugin::Registration::GlobalFunction {
                    declaration,
                    function,
                    auxiliary,
                } => {
                    self.register_global_function(&declaration, function, auxiliary)?;
                }
                plugin::Registration::GlobalProperty {
                    declaration,
                    property,
                } => {
                    self.register_global_property(&declaration, property)?;
                }
                plugin::Registration::ObjectType {
                    name,
                    size,
                    flags,
                    type_builder,
                } => {
                    self.register_object_type(&name, size, flags)?;

                    // Apply methods
                    for method in type_builder.methods {
                        self.register_object_method(
                            &name,
                            &method.declaration,
                            method.function,
                            method.auxiliary.as_ref(),
                            method.composite_offset,
                            method.is_composite_indirect,
                        )?;
                    }

                    // Apply properties
                    for property in type_builder.properties {
                        self.register_object_property(
                            &name,
                            &property.declaration,
                            property.byte_offset,
                            property.composite_offset.unwrap_or(0),
                            property.is_composite_indirect.unwrap_or(false),
                        )?;
                    }

                    // Apply custom behaviors
                    for behavior in type_builder.behaviors {
                        self.register_object_behaviour(
                            &name,
                            behavior.behavior,
                            &behavior.declaration,
                            behavior.function,
                            behavior.auxiliary.as_ref(),
                            behavior.composite_offset,
                            behavior.is_composite_indirect,
                        )?;
                    }
                }
            }
        }
        if was_namespaced {
            self.set_default_namespace("")?;
        }
        Ok(())
    }

    /// Gets the AngelScript library version string
    pub fn get_library_version() -> &'static str {
        unsafe {
            let version_ptr = asGetLibraryVersion();
            CStr::from_ptr(version_ptr).to_str().unwrap_or("Unknown")
        }
    }

    /// Gets the AngelScript library compilation options
    pub fn get_library_options() -> &'static str {
        unsafe {
            let options_ptr = asGetLibraryOptions();
            CStr::from_ptr(options_ptr).to_str().unwrap_or("Unknown")
        }
    }

    // ========== CONTEXT MANAGEMENT ==========

    /// Gets the currently active script context
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

    /// Prepares AngelScript for multithreaded use
    ///
    /// The implementation used depends on the compile-time feature:
    /// - Default: Uses AngelScript's built-in C++ thread manager
    /// - `rust-threads`: Uses a pure Rust implementation
    pub fn prepare_multithread() -> ScriptResult<ThreadManager> {
        ThreadManager::prepare()
    }

    /// Unprepares AngelScript from multithreaded use
    pub fn unprepare_multithread() {
        ThreadManager::unprepare()
    }

    /// Gets the current thread manager
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

    /// Acquires an exclusive lock for thread synchronization
    pub fn acquire_exclusive_lock() {
        ThreadManager::acquire_exclusive_lock()
    }

    /// Releases an exclusive lock
    pub fn release_exclusive_lock() {
        ThreadManager::release_exclusive_lock()
    }

    /// Acquires a shared lock for thread synchronization
    pub fn acquire_shared_lock() {
        ThreadManager::acquire_shared_lock()
    }

    /// Releases a shared lock
    pub fn release_shared_lock() {
        ThreadManager::release_shared_lock()
    }

    /// Creates an exclusive lock guard for RAII locking
    pub fn exclusive_lock() -> ExclusiveLockGuard {
        ExclusiveLockGuard::new()
    }

    /// Creates a shared lock guard for RAII locking
    pub fn shared_lock() -> SharedLockGuard {
        SharedLockGuard::new()
    }

    // ========== ATOMIC OPERATIONS ==========

    /// Atomically increments an integer value
    pub fn atomic_inc(value: &mut i32) -> i32 {
        unsafe { asAtomicInc(value as *mut i32) }
    }

    /// Atomically decrements an integer value
    pub fn atomic_dec(value: &mut i32) -> i32 {
        unsafe { asAtomicDec(value as *mut i32) }
    }

    // ========== THREAD CLEANUP ==========

    /// Performs thread-specific cleanup
    pub fn thread_cleanup() -> ScriptResult<()> {
        ThreadManager::cleanup_local_data()
    }

    // ========== MEMORY MANAGEMENT ==========

    /// Sets custom global memory allocation functions
    pub fn set_global_memory_functions(
        alloc_func: asALLOCFUNC_t,
        free_func: asFREEFUNC_t,
    ) -> ScriptResult<()> {
        unsafe { ScriptError::from_code(asSetGlobalMemoryFunctions(alloc_func, free_func)) }
    }

    /// Resets global memory functions to default
    pub fn reset_global_memory_functions() -> ScriptResult<()> {
        unsafe { ScriptError::from_code(asResetGlobalMemoryFunctions()) }
    }

    /// Allocates memory using AngelScript's allocator
    pub fn alloc_mem(size: usize) -> ScriptMemoryLocation {
        unsafe { ScriptMemoryLocation::from_mut(asAllocMem(size)) }
    }

    /// Frees memory allocated by AngelScript's allocator
    pub fn free_mem(mut mem: ScriptMemoryLocation) {
        unsafe {
            asFreeMem(mem.as_mut_ptr());
        }
    }

    // ========== UTILITY OBJECTS ==========

    /// Creates a new lockable shared boolean
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

    /// Executes a closure with an exclusive lock held
    pub fn with_exclusive_lock<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = Self::exclusive_lock();
        f()
    }

    /// Executes a closure with a shared lock held
    pub fn with_shared_lock<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = Self::shared_lock();
        f()
    }

    /// Checks if AngelScript is prepared for multithreading
    pub fn is_multithreading_prepared() -> bool {
        Self::get_thread_manager().is_some()
    }

    /// Gets information about the current threading setup
    pub fn get_threading_info() -> String {
        match Self::get_thread_manager() {
            Some(manager) => manager.info(),
            None => "No thread manager (single-threaded mode)".to_string(),
        }
    }

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
    pub fn add_ref(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_AddRef)(
                self.inner.as_ptr(),
            ))
        }
    }

    pub fn release(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_Release)(
                self.inner.as_ptr(),
            ))
        }
    }

    pub fn shutdown_and_release(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_ShutDownAndRelease)(
                self.inner.as_ptr(),
            ))
        }
    }

    // Engine properties
    pub fn set_engine_property(&self, property: EngineProperty, value: usize) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_SetEngineProperty)(
                self.inner.as_ptr(),
                property.into(),
                value,
            ))
        }
    }

    pub fn get_engine_property(&self, property: EngineProperty) -> asPWORD {
        unsafe {
            (self.as_vtable().asIScriptEngine_GetEngineProperty)(
                self.inner.as_ptr(),
                property.into(),
            )
        }
    }

    pub fn set_message_callback(&mut self, callback: MessageCallbackFn) -> ScriptResult<()> {
        CallbackManager::set_message_callback(Some(callback))?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_SetMessageCallback)(
                self.inner.as_ptr(),
                &mut asMessageInfoFunction(Some(CallbackManager::cvoid_msg_callback)),
                ptr::null_mut(),
                CallingConvention::Cdecl.into(),
            ))
        }
    }

    pub fn clear_message_callback(&mut self) -> ScriptResult<()> {
        CallbackManager::set_message_callback(None)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_ClearMessageCallback)(
                self.inner.as_ptr(),
            ))
        }
    }

    pub fn write_message(
        &self,
        section: &str,
        row: i32,
        col: i32,
        msg_type: asEMsgType,
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
                msg_type,
                c_message.as_ptr(),
            ))
        }
    }

    // JIT compiler
    pub fn set_jit_compiler(&self, compiler: &mut JITCompiler) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_SetJITCompiler)(
                self.inner.as_ptr(),
                compiler.as_ptr(),
            ))
        }
    }

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
    unsafe extern "C" fn generic_callback_thunk(arg1: *mut asIScriptGeneric) {
        let func = ScriptGeneric::from_raw(arg1).get_function();
        let user_data = func.get_user_data::<GenericFnUserData>();

        if let Some(f) = user_data {
            let s = ScriptGeneric::from_raw(arg1);
            f.call(&s);
        }
    }

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

    pub fn get_global_function_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetGlobalFunctionCount)(self.inner.as_ptr()) }
    }

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

    pub fn get_global_property_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetGlobalPropertyCount)(self.inner.as_ptr()) }
    }

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

    pub fn register_object_method(
        &self,
        obj: &str,
        declaration: &str,
        func_ptr: GenericFn,
        auxiliary: Option<&Box<dyn ScriptData>>,
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
                    .map(|mut aux| aux.to_script_ptr())
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

    pub fn register_object_behaviour(
        &self,
        obj: &str,
        behaviour: Behaviour,
        declaration: &str,
        func_ptr: GenericFn,
        auxiliary: Option<&Box<dyn ScriptData>>,
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
                auxiliary.map_or_else(ptr::null_mut, |mut aux| aux.to_script_ptr()),
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
    pub fn register_interface(&self, name: &str) -> ScriptResult<()> {
        let c_name = CString::new(name)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RegisterInterface)(
                self.inner.as_ptr(),
                c_name.as_ptr(),
            ))
        }
    }

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

    pub fn get_object_type_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetObjectTypeCount)(self.inner.as_ptr()) }
    }

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
    pub(crate) fn register_string_factory(
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
    pub fn register_default_array_type(&self, type_name: &str) -> ScriptResult<()> {
        let c_type = CString::new(type_name)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RegisterDefaultArrayType)(
                self.inner.as_ptr(),
                c_type.as_ptr(),
            ))
        }
    }

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
    pub fn register_enum(&self, type_name: &str) -> ScriptResult<()> {
        let c_type = CString::new(type_name)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RegisterEnum)(
                self.inner.as_ptr(),
                c_type.as_ptr(),
            ))
        }
    }

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

    pub fn get_enum_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetEnumCount)(self.inner.as_ptr()) }
    }

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
    pub fn register_funcdef(&self, decl: &str) -> ScriptResult<()> {
        let c_decl = CString::new(decl)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_RegisterFuncdef)(
                self.inner.as_ptr(),
                c_decl.as_ptr(),
            ))
        }
    }

    pub fn get_funcdef_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetFuncdefCount)(self.inner.as_ptr()) }
    }

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

    pub fn get_typedef_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetTypedefCount)(self.inner.as_ptr()) }
    }

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
    pub fn begin_config_group(&self, group_name: &str) -> ScriptResult<()> {
        let c_group = CString::new(group_name)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_BeginConfigGroup)(
                self.inner.as_ptr(),
                c_group.as_ptr(),
            ))
        }
    }

    pub fn end_config_group(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_EndConfigGroup)(
                self.inner.as_ptr(),
            ))
        }
    }

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
    pub fn set_default_access_mask(&self, default_mask: asDWORD) -> asDWORD {
        unsafe {
            (self.as_vtable().asIScriptEngine_SetDefaultAccessMask)(
                self.inner.as_ptr(),
                default_mask,
            )
        }
    }

    // Namespaces
    pub fn set_default_namespace(&self, name_space: &str) -> ScriptResult<()> {
        let c_namespace = CString::new(name_space)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_SetDefaultNamespace)(
                self.inner.as_ptr(),
                c_namespace.as_ptr(),
            ))
        }
    }

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

    pub fn discard_module(&self, name: &str) -> ScriptResult<()> {
        let c_name = CString::new(name)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_DiscardModule)(
                self.inner.as_ptr(),
                c_name.as_ptr(),
            ))
        }
    }

    pub fn get_module_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetModuleCount)(self.inner.as_ptr()) }
    }

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
    pub fn get_last_function_id(&self) -> i32 {
        unsafe { (self.as_vtable().asIScriptEngine_GetLastFunctionId)(self.inner.as_ptr()) }
    }

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

    pub fn release_script_object<T: ScriptData>(&self, obj: &mut T, type_info: &TypeInfo) {
        unsafe {
            (self.as_vtable().asIScriptEngine_ReleaseScriptObject)(
                self.inner.as_ptr(),
                obj.to_script_ptr(),
                type_info.as_ptr(),
            );
        }
    }

    pub fn add_ref_script_object<T: ScriptData>(&self, obj: &mut T, type_info: &TypeInfo) {
        unsafe {
            (self.as_vtable().asIScriptEngine_AddRefScriptObject)(
                self.inner.as_ptr(),
                obj.to_script_ptr(),
                type_info.as_ptr(),
            );
        }
    }

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

    pub fn return_context(&self, ctx: Context) {
        unsafe {
            (self.as_vtable().asIScriptEngine_ReturnContext)(self.inner.as_ptr(), ctx.as_ptr());
        }
    }

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
    pub fn garbage_collect(&self, flags: asDWORD, num_iterations: asUINT) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptEngine_GarbageCollect)(
                self.inner.as_ptr(),
                flags,
                num_iterations,
            ))
        }
    }

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

    pub fn gc_enum_callback<T: ScriptData>(&self, reference: &mut T) {
        unsafe {
            (self.as_vtable().asIScriptEngine_GCEnumCallback)(
                self.inner.as_ptr(),
                reference.to_script_ptr(),
            );
        }
    }

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
    pub fn set_user_data<T: UserData + ScriptData>(&self, data: &mut T) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptEngine_SetUserData)(
                self.inner.as_ptr(),
                data.to_script_ptr(),
                T::TYPE_ID as asPWORD,
            );
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    pub fn get_user_data<T: UserData + ScriptData>(&self) -> ScriptResult<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptEngine_GetUserData)(
                self.inner.as_ptr(),
                T::TYPE_ID as asPWORD,
            );
            if ptr.is_null() {
                Err(ScriptError::NullPointer)
            } else {
                Ok(ScriptData::from_script_ptr(ptr))
            }
        }
    }

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

    pub fn with_default_plugins(&self) -> ScriptResult<()> {
        #[cfg(feature = "string")]
        {
            self.install(crate::plugins::string::plugin()?)?;
            self.register_string_factory("string", get_string_factory_instance())?;
        }

        Ok(())
    }

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

struct GenericFnUserData(pub GenericFn);

impl GenericFnUserData {
    pub fn call(self, ctx: &ScriptGeneric) {
        (self.0)(ctx)
    }
}

unsafe impl Send for GenericFnUserData {}
unsafe impl Sync for GenericFnUserData {}

impl UserData for GenericFnUserData {
    const TYPE_ID: usize = 0x129032719; // Must be unique!
}

#[derive(Debug)]
pub struct GlobalPropertyInfo {
    pub name: Option<String>,
    pub name_space: Option<String>,
    pub type_id: i32,
    pub is_const: bool,
    pub config_group: Option<String>,
    pub pointer: ScriptMemoryLocation,
    pub access_mask: asDWORD,
}

#[derive(Debug)]
pub struct GCStatistics {
    pub current_size: asUINT,
    pub total_destroyed: asUINT,
    pub total_detected: asUINT,
    pub new_objects: asUINT,
    pub total_new_destroyed: asUINT,
}

#[derive(Debug)]
pub struct GCObjectInfo {
    pub seq_nbr: asUINT,
    pub obj: ScriptMemoryLocation,
    pub type_info: Option<TypeInfo>,
}
