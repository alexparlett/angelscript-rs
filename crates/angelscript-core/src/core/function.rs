use crate::core::engine::Engine;
use crate::types::enums::*;
use crate::core::error::{ScriptError, ScriptResult};
use crate::core::module::Module;
use crate::core::typeinfo::TypeInfo;
use crate::types::user_data::UserData;
use angelscript_sys::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::ptr::NonNull;
use crate::types::script_memory::ScriptMemoryLocation;
use crate::types::script_data::ScriptData;

/// A wrapper around AngelScript function objects.
///
/// The Function struct provides a safe Rust interface to AngelScript functions,
/// whether they are script functions, registered application functions, or delegates.
/// It allows inspection of function metadata, parameters, variables, and execution context.
///
/// # Function Types
///
/// AngelScript supports several types of functions:
/// - **Script Functions**: Functions written in AngelScript
/// - **Application Functions**: C++ functions registered with the engine
/// - **Delegates**: Function objects that can be passed around and called
/// - **Interface Methods**: Virtual functions defined in interfaces
/// - **Property Accessors**: Getter/setter functions for properties
///
/// # Thread Safety
///
/// Function objects are thread-safe and can be shared between threads. However,
/// the underlying function execution must still respect AngelScript's threading model.
///
/// # Examples
///
/// ```rust
/// use angelscript_rs::{Engine, GetModuleFlags};
///
/// let engine = Engine::create()?;
/// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
///
/// module.add_script_section("script", r#"
///     int add(int a, int b) {
///         return a + b;
///     }
///
///     class MyClass {
///         void method() { }
///     }
/// "#)?;
/// module.build()?;
///
/// // Get a global function
/// if let Some(func) = module.get_function_by_name("add") {
///     println!("Function: {}", func.get_declaration(true, true, true)?);
///     println!("Parameters: {}", func.get_param_count());
///     println!("Return type: {:?}", func.get_return_type_id());
/// }
///
/// // Inspect function metadata
/// if let Some(func) = module.get_function_by_name("add") {
///     println!("Function ID: {}", func.get_id());
///     println!("Function type: {:?}", func.get_func_type());
///     println!("Module: {:?}", func.get_module_name());
///
///     // Inspect parameters
///     for i in 0..func.get_param_count() {
///         if let Ok(param) = func.get_param(i) {
///             println!("Param {}: type_id={}, name={:?}",
///                      i, param.type_id, param.name);
///         }
///     }
/// }
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub struct Function {
    inner: *mut asIScriptFunction,
}

impl Function {
    /// Creates a Function wrapper from a raw AngelScript function pointer.
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid and the function is properly initialized.
    ///
    /// # Arguments
    /// * `function` - Raw pointer to AngelScript function
    ///
    /// # Returns
    /// A new Function wrapper
    pub(crate) fn from_raw(function: *mut asIScriptFunction) -> Self {
        Function { inner: function }
    }

    /// Gets the raw AngelScript function pointer.
    ///
    /// # Safety
    /// This function provides direct access to the AngelScript function pointer.
    /// The caller must ensure proper usage according to AngelScript's API.
    pub(crate) fn as_raw(&self) -> *mut asIScriptFunction {
        self.inner
    }

    // ========== VTABLE ORDER (matches asIScriptFunction__bindgen_vtable) ==========

    /// Gets the engine that owns this function.
    ///
    /// # Returns
    /// The engine instance or an error if the engine is not available
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    /// let engine = function.get_engine()?;
    /// println!("Engine version: {}", Engine::get_library_version());
    /// ```
    pub fn get_engine(&self) -> ScriptResult<Engine> {
        unsafe {
            let result: *mut asIScriptEngine =
                (self.as_vtable().asIScriptFunction_GetEngine)(self.inner);
            let ptr = NonNull::new(result).ok_or(ScriptError::NullPointer)?;
            Ok(Engine::from_raw(ptr))
        }
    }

    /// Increments the reference count of the function.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn add_ref(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptFunction_AddRef)(self.inner)) }
    }

    /// Decrements the reference count of the function.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn release(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptFunction_Release)(self.inner)) }
    }

    /// Gets the unique identifier of this function.
    ///
    /// Function IDs are unique within an engine instance and can be used
    /// to retrieve functions or compare function identity.
    ///
    /// # Returns
    /// The function's unique ID
    ///
    /// # Examples
    ///
    /// ```rust
    /// let func1 = module.get_function_by_name("test")?;
    /// let func2 = engine.get_function_by_id(func1.get_id())?;
    /// assert_eq!(func1.get_id(), func2.get_id());
    /// ```
    pub fn get_id(&self) -> i32 {
        unsafe { (self.as_vtable().asIScriptFunction_GetId)(self.inner) }
    }

    /// Gets the type of this function.
    ///
    /// # Returns
    /// The function type (script function, system function, delegate, etc.)
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    /// match function.get_func_type() {
    ///     FunctionType::Script => println!("This is a script function"),
    ///     FunctionType::System => println!("This is a registered system function"),
    ///     FunctionType::Interface => println!("This is an interface method"),
    ///     FunctionType::Virtual => println!("This is a virtual method"),
    ///     FunctionType::Delegate => println!("This is a delegate"),
    ///     _ => println!("Other function type"),
    /// }
    /// ```
    pub fn get_func_type(&self) -> FunctionType {
        unsafe { FunctionType::from((self.as_vtable().asIScriptFunction_GetFuncType)(self.inner)) }
    }

    /// Gets the name of the module that contains this function.
    ///
    /// # Returns
    /// The module name, or None if the function is not part of a module
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    /// if let Some(module_name) = function.get_module_name() {
    ///     println!("Function belongs to module: {}", module_name);
    /// }
    /// ```
    pub fn get_module_name(&self) -> Option<&str> {
        unsafe {
            let name = (self.as_vtable().asIScriptFunction_GetModuleName)(self.inner);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    /// Gets the module that contains this function.
    ///
    /// # Returns
    /// The module instance, or None if the function is not part of a module
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    /// if let Some(func_module) = function.get_module() {
    ///     println!("Function module has {} functions",
    ///              func_module.get_function_count());
    /// }
    /// ```
    pub fn get_module(&self) -> Option<Module> {
        unsafe {
            let module = (self.as_vtable().asIScriptFunction_GetModule)(self.inner);
            if module.is_null() {
                None
            } else {
                Some(Module::from_raw(module))
            }
        }
    }

    /// Gets the name of the script section where this function was declared.
    ///
    /// # Returns
    /// The script section name, or None if not applicable
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    /// if let Some(section) = function.get_script_section_name() {
    ///     println!("Function declared in section: {}", section);
    /// }
    /// ```
    pub fn get_script_section_name(&self) -> Option<&str> {
        unsafe {
            let name = (self.as_vtable().asIScriptFunction_GetScriptSectionName)(self.inner);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    /// Gets the configuration group this function belongs to.
    ///
    /// Configuration groups allow batch removal of related registrations.
    ///
    /// # Returns
    /// The configuration group name, or None if not in a group
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = engine.get_global_function_by_decl("void myFunc()")?;
    /// if let Some(group) = function.get_config_group() {
    ///     println!("Function is in config group: {}", group);
    /// }
    /// ```
    pub fn get_config_group(&self) -> Option<&str> {
        unsafe {
            let group = (self.as_vtable().asIScriptFunction_GetConfigGroup)(self.inner);
            if group.is_null() {
                None
            } else {
                CStr::from_ptr(group).to_str().ok()
            }
        }
    }

    /// Gets the access mask for this function.
    ///
    /// Access masks control which modules can access this function.
    ///
    /// # Returns
    /// The access mask as a bitmask
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    /// let access_mask = function.get_access_mask();
    /// if access_mask & 0x01 != 0 {
    ///     println!("Function is accessible to group 1");
    /// }
    /// ```
    pub fn get_access_mask(&self) -> asDWORD {
        unsafe { (self.as_vtable().asIScriptFunction_GetAccessMask)(self.inner) }
    }

    /// Gets auxiliary data associated with this function.
    ///
    /// Auxiliary data is typically used by the application to store
    /// additional information about registered functions.
    ///
    /// # Returns
    /// The auxiliary data
    ///
    /// # Examples
    ///
    /// ```rust
    /// // When registering a function with auxiliary data
    /// let aux_data = MyAuxiliaryData::new();
    /// engine.register_global_function(
    ///     "void myFunc()",
    ///     my_function,
    ///     Some(Box::new(aux_data))
    /// )?;
    ///
    /// // Later, retrieve the auxiliary data
    /// if let Some(func) = engine.get_global_function_by_decl("void myFunc()") {
    ///     let aux: MyAuxiliaryData = func.get_auxiliary();
    ///     // Use auxiliary data...
    /// }
    /// ```
    pub fn get_auxiliary<T: ScriptData>(&self) -> T {
        unsafe {
            let ptr = (self.as_vtable().asIScriptFunction_GetAuxiliary)(self.inner);
            ScriptData::from_script_ptr(ptr)
        }
    }

    /// Gets the object type this function belongs to (for methods).
    ///
    /// # Returns
    /// The object type, or None if this is not a method
    ///
    /// # Examples
    ///
    /// ```rust
    /// // For a method like "void MyClass::method()"
    /// if let Some(method) = module.get_function_by_decl("void MyClass::method()") {
    ///     if let Some(obj_type) = method.get_object_type() {
    ///         println!("Method belongs to type: {}", obj_type.get_name()?);
    ///     }
    /// }
    /// ```
    pub fn get_object_type(&self) -> Option<TypeInfo> {
        unsafe {
            let type_info = (self.as_vtable().asIScriptFunction_GetObjectType)(self.inner);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    /// Gets the name of the object type this function belongs to (for methods).
    ///
    /// # Returns
    /// The object type name, or None if this is not a method
    ///
    /// # Examples
    ///
    /// ```rust
    /// if let Some(method) = module.get_function_by_decl("void MyClass::method()") {
    ///     if let Some(class_name) = method.get_object_name() {
    ///         println!("Method belongs to class: {}", class_name);
    ///     }
    /// }
    /// ```
    pub fn get_object_name(&self) -> Option<&str> {
        unsafe {
            let name = (self.as_vtable().asIScriptFunction_GetObjectName)(self.inner);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    /// Gets the function name.
    ///
    /// # Returns
    /// The function name, or None if not available
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    /// if let Some(name) = function.get_name() {
    ///     println!("Function name: {}", name);
    /// }
    /// ```
    pub fn get_name(&self) -> Option<&str> {
        unsafe {
            let name = (self.as_vtable().asIScriptFunction_GetName)(self.inner);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    /// Gets the namespace this function belongs to.
    ///
    /// # Returns
    /// The namespace name, or None if in the global namespace
    ///
    /// # Examples
    ///
    /// ```rust
    /// // For a function declared as "namespace MyNamespace { void func(); }"
    /// if let Some(function) = module.get_function_by_name("func") {
    ///     if let Some(ns) = function.get_namespace() {
    ///         println!("Function is in namespace: {}", ns);
    ///     }
    /// }
    /// ```
    pub fn get_namespace(&self) -> Option<&str> {
        unsafe {
            let namespace = (self.as_vtable().asIScriptFunction_GetNamespace)(self.inner);
            if namespace.is_null() {
                None
            } else {
                CStr::from_ptr(namespace).to_str().ok()
            }
        }
    }

    /// Gets the full declaration string for this function.
    ///
    /// # Arguments
    /// * `include_object_name` - Whether to include the object name for methods
    /// * `include_namespace` - Whether to include the namespace
    /// * `include_param_names` - Whether to include parameter names
    ///
    /// # Returns
    /// The function declaration string or an error
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    ///
    /// // Get minimal declaration
    /// let minimal = function.get_declaration(false, false, false)?;
    /// println!("Minimal: {}", minimal);
    ///
    /// // Get full declaration with all details
    /// let full = function.get_declaration(true, true, true)?;
    /// println!("Full: {}", full);
    /// ```
    pub fn get_declaration(
        &self,
        include_object_name: bool,
        include_namespace: bool,
        include_param_names: bool,
    ) -> ScriptResult<&str> {
        unsafe {
            let decl = (self.as_vtable().asIScriptFunction_GetDeclaration)(
                self.inner,
                include_object_name,
                include_namespace,
                include_param_names,
            );
            if decl.is_null() {
                Err(ScriptError::NullPointer)
            } else {
                CStr::from_ptr(decl)
                    .to_str()
                    .map_err(ScriptError::from)
            }
        }
    }

    /// Checks if this function is read-only (const).
    ///
    /// Read-only functions cannot modify the object they're called on.
    ///
    /// # Returns
    /// true if the function is read-only, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// // For a function declared as "int getValue() const"
    /// if let Some(method) = module.get_function_by_decl("int MyClass::getValue() const") {
    ///     assert!(method.is_read_only());
    /// }
    /// ```
    pub fn is_read_only(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsReadOnly)(self.inner) }
    }

    /// Checks if this function is private.
    ///
    /// # Returns
    /// true if the function is private, false otherwise
    pub fn is_private(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsPrivate)(self.inner) }
    }

    /// Checks if this function is protected.
    ///
    /// # Returns
    /// true if the function is protected, false otherwise
    pub fn is_protected(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsProtected)(self.inner) }
    }

    /// Checks if this function is final (cannot be overridden).
    ///
    /// # Returns
    /// true if the function is final, false otherwise
    pub fn is_final(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsFinal)(self.inner) }
    }

    /// Checks if this function overrides a base class method.
    ///
    /// # Returns
    /// true if the function is an override, false otherwise
    pub fn is_override(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsOverride)(self.inner) }
    }

    /// Checks if this function is shared between modules.
    ///
    /// # Returns
    /// true if the function is shared, false otherwise
    pub fn is_shared(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsShared)(self.inner) }
    }

    /// Checks if this function is an explicit constructor.
    ///
    /// # Returns
    /// true if the function is explicit, false otherwise
    pub fn is_explicit(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsExplicit)(self.inner) }
    }

    /// Checks if this function is a property accessor.
    ///
    /// # Returns
    /// true if the function is a property accessor, false otherwise
    pub fn is_property(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsProperty)(self.inner) }
    }

    /// Gets the number of parameters this function takes.
    ///
    /// # Returns
    /// The parameter count
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    /// let param_count = function.get_param_count();
    /// println!("Function takes {} parameters", param_count);
    ///
    /// for i in 0..param_count {
    ///     if let Ok(param) = function.get_param(i) {
    ///         println!("Parameter {}: {:?}", i, param);
    ///     }
    /// }
    /// ```
    pub fn get_param_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptFunction_GetParamCount)(self.inner) }
    }

    /// Gets information about a specific parameter.
    ///
    /// # Arguments
    /// * `index` - The parameter index (0-based)
    ///
    /// # Returns
    /// Parameter information or an error if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    ///
    /// for i in 0..function.get_param_count() {
    ///     match function.get_param(i) {
    ///         Ok(param) => {
    ///             println!("Parameter {}: type_id={}, name={:?}, default={:?}",
    ///                      i, param.type_id, param.name, param.default_arg);
    ///         }
    ///         Err(e) => eprintln!("Error getting parameter {}: {}", i, e),
    ///     }
    /// }
    /// ```
    pub fn get_param(&self, index: asUINT) -> ScriptResult<ParamInfo> {
        let mut type_id: i32 = 0;
        let mut flags: asDWORD = 0;
        let mut name: *const c_char = ptr::null();
        let mut default_arg: *const c_char = ptr::null();

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptFunction_GetParam)(
                self.inner,
                index,
                &mut type_id,
                &mut flags,
                &mut name,
                &mut default_arg,
            ))?;

            Ok(ParamInfo {
                type_id,
                flags,
                name: if name.is_null() {
                    None
                } else {
                    CStr::from_ptr(name).to_str().ok()
                },
                default_arg: if default_arg.is_null() {
                    None
                } else {
                    CStr::from_ptr(default_arg).to_str().ok()
                },
            })
        }
    }

    /// Gets the return type information for this function.
    ///
    /// # Returns
    /// A tuple of (type_id, flags) describing the return type
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    /// let (return_type_id, flags) = function.get_return_type_id();
    ///
    /// println!("Return type ID: {}", return_type_id);
    /// if flags & TypeFlags::Reference as u32 != 0 {
    ///     println!("Returns a reference");
    /// }
    /// ```
    pub fn get_return_type_id(&self) -> (i32, asDWORD) {
        let mut flags: asDWORD = 0;
        unsafe {
            let type_id =
                (self.as_vtable().asIScriptFunction_GetReturnTypeId)(self.inner, &mut flags);
            (type_id, flags)
        }
    }

    /// Gets the type ID for this function (for function pointers).
    ///
    /// # Returns
    /// The function's type ID
    pub fn get_type_id(&self) -> i32 {
        unsafe { (self.as_vtable().asIScriptFunction_GetTypeId)(self.inner) }
    }

    /// Checks if this function is compatible with a given type ID.
    ///
    /// # Arguments
    /// * `type_id` - The type ID to check compatibility with
    ///
    /// # Returns
    /// true if compatible, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    /// let func_type_id = function.get_type_id();
    ///
    /// if function.is_compatible_with_type_id(func_type_id) {
    ///     println!("Function is compatible with its own type");
    /// }
    /// ```
    pub fn is_compatible_with_type_id(&self, type_id: i32) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsCompatibleWithTypeId)(self.inner, type_id) }
    }

    /// Gets the object bound to this delegate function.
    ///
    /// # Returns
    /// The delegate object
    ///
    /// # Examples
    ///
    /// ```rust
    /// // For delegate functions created with engine.create_delegate()
    /// if function.get_func_type() == FunctionType::Delegate {
    ///     let obj: MyObject = function.get_delegate_object();
    ///     // Use the bound object...
    /// }
    /// ```
    pub fn get_delegate_object<T: ScriptData>(&self) -> T {
        unsafe {
            let ptr = (self.as_vtable().asIScriptFunction_GetDelegateObject)(self.inner);
            ScriptData::from_script_ptr(ptr)
        }
    }

    /// Gets the type of the object bound to this delegate function.
    ///
    /// # Returns
    /// The delegate object's type, or None if not a delegate
    pub fn get_delegate_object_type(&self) -> Option<TypeInfo> {
        unsafe {
            let type_info = (self.as_vtable().asIScriptFunction_GetDelegateObjectType)(self.inner);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    /// Gets the underlying function for this delegate.
    ///
    /// # Returns
    /// The delegate's underlying function, or None if not a delegate
    pub fn get_delegate_function(&self) -> Option<Function> {
        unsafe {
            let func = (self.as_vtable().asIScriptFunction_GetDelegateFunction)(self.inner);
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    /// Gets the number of local variables in this function.
    ///
    /// # Returns
    /// The number of local variables
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    /// let var_count = function.get_var_count();
    /// println!("Function has {} local variables", var_count);
    ///
    /// for i in 0..var_count {
    ///     if let Ok(var) = function.get_var(i) {
    ///         println!("Variable {}: {:?}", i, var);
    ///     }
    /// }
    /// ```
    pub fn get_var_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptFunction_GetVarCount)(self.inner) }
    }

    /// Gets information about a local variable.
    ///
    /// # Arguments
    /// * `index` - The variable index (0-based)
    ///
    /// # Returns
    /// Variable information or an error if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    ///
    /// for i in 0..function.get_var_count() {
    ///     match function.get_var(i) {
    ///         Ok(var) => {
    ///             println!("Variable {}: name={:?}, type_id={}",
    ///                      i, var.name, var.type_id);
    ///         }
    ///         Err(e) => eprintln!("Error getting variable {}: {}", i, e),
    ///     }
    /// }
    /// ```
    pub fn get_var(&self, index: asUINT) -> ScriptResult<VarInfo> {
        let mut name: *const c_char = ptr::null();
        let mut type_id: i32 = 0;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptFunction_GetVar)(
                self.inner,
                index,
                &mut name,
                &mut type_id,
            ))?;

            Ok(VarInfo {
                name: if name.is_null() {
                    None
                } else {
                    CStr::from_ptr(name).to_str().ok()
                },
                type_id,
            })
        }
    }

    /// Gets the declaration string for a local variable.
    ///
    /// # Arguments
    /// * `index` - The variable index (0-based)
    /// * `include_namespace` - Whether to include the namespace in type names
    ///
    /// # Returns
    /// The variable declaration string or an error
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    ///
    /// for i in 0..function.get_var_count() {
    ///     match function.get_var_decl(i, true) {
    ///         Ok(decl) => println!("Variable {}: {}", i, decl),
    ///         Err(e) => eprintln!("Error getting variable declaration: {}", e),
    ///     }
    /// }
    /// ```
    pub fn get_var_decl(&self, index: asUINT, include_namespace: bool) -> ScriptResult<&str> {
        unsafe {
            let decl = (self.as_vtable().asIScriptFunction_GetVarDecl)(
                self.inner,
                index,
                include_namespace,
            );
            if decl.is_null() {
                Err(ScriptError::NullPointer)
            } else {
                CStr::from_ptr(decl)
                    .to_str()
                    .map_err(ScriptError::Utf8Conversion)
            }
        }
    }

    /// Finds the next line with executable code starting from the given line.
    ///
    /// This is useful for debuggers to find valid breakpoint locations.
    ///
    /// # Arguments
    /// * `line` - The starting line number
    ///
    /// # Returns
    /// The line number with code, or an error if no such line exists
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    ///
    /// // Find the first line with code
    /// match function.find_next_line_with_code(1) {
    ///     Ok(line) => println!("First executable line: {}", line),
    ///     Err(_) => println!("No executable code found"),
    /// }
    /// ```
    pub fn find_next_line_with_code(&self, line: i32) -> ScriptResult<i32> {
        unsafe {
            let result =
                (self.as_vtable().asIScriptFunction_FindNextLineWithCode)(self.inner, line);
            if result < 0 {
                ScriptError::from_code(result)?;
            }
            Ok(result)
        }
    }

    /// Gets information about where this function was declared.
    ///
    /// # Returns
    /// Declaration location information or an error
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    ///
    /// match function.get_declared_at() {
    ///     Ok(info) => {
    ///         println!("Function declared at line {} column {} in {:?}",
    ///                  info.row, info.col, info.script_section);
    ///     }
    ///     Err(e) => eprintln!("Could not get declaration info: {}", e),
    /// }
    /// ```
    pub fn get_declared_at(&self) -> ScriptResult<DeclaredAtInfo> {
        let mut script_section: *const c_char = ptr::null();
        let mut row: i32 = 0;
        let mut col: i32 = 0;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptFunction_GetDeclaredAt)(
                self.inner,
                &mut script_section,
                &mut row,
                &mut col,
            ))?;

            Ok(DeclaredAtInfo {
                script_section: if script_section.is_null() {
                    None
                } else {
                    CStr::from_ptr(script_section).to_str().ok()
                },
                row,
                col,
            })
        }
    }

    /// Gets the bytecode for this function.
    ///
    /// This provides access to the compiled bytecode for script functions,
    /// which can be useful for debugging or analysis tools.
    ///
    /// # Returns
    /// A tuple of (bytecode_pointer, length) or None if not available
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    ///
    /// if let Some((bytecode, length)) = function.get_byte_code() {
    ///     println!("Function has {} bytes of bytecode", length);
    ///     // Analyze bytecode...
    /// } else {
    ///     println!("No bytecode available (probably a system function)");
    /// }
    /// ```
    pub fn get_byte_code(&self) -> Option<(ScriptMemoryLocation, asUINT)> {
        let mut length: asUINT = 0;
        unsafe {
            let ptr = (self.as_vtable().asIScriptFunction_GetByteCode)(self.inner, &mut length);
            if ptr.is_null() {
                None
            } else {
                Some((ScriptMemoryLocation::from_mut(ptr as *mut c_void), length))
            }
        }
    }

    /// Sets a JIT-compiled function for this script function.
    ///
    /// # Arguments
    /// * `jit_func` - The JIT-compiled function pointer
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// // This would typically be done by a JIT compiler
    /// let function = module.get_function_by_name("myFunction")?;
    /// // let jit_func = compile_to_native(function);
    /// // function.set_jit_function(jit_func)?;
    /// ```
    pub fn set_jit_function(&self, jit_func: asJITFunction) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptFunction_SetJITFunction)(
                self.inner, jit_func,
            ))
        }
    }

    /// Gets the JIT-compiled function pointer.
    ///
    /// # Returns
    /// The JIT function pointer, or null if no JIT function is set
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    /// let jit_func = function.get_jit_function();
    ///
    /// if !jit_func.is_null() {
    ///     println!("Function has JIT compilation available");
    /// }
    /// ```
    pub fn get_jit_function(&self) -> JITFunction {
        unsafe {
            (self.as_vtable().asIScriptFunction_GetJITFunction)(self.inner)
        }
    }

    /// Sets user data on this function.
    ///
    /// User data allows applications to associate custom data with functions.
    ///
    /// # Arguments
    /// * `data` - The user data to set
    ///
    /// # Returns
    /// The previous user data, or None if none was set
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    /// let mut my_data = MyUserData::new();
    ///
    /// if let Some(old_data) = function.set_user_data(&mut my_data) {
    ///     println!("Replaced existing user data");
    /// }
    /// ```
    pub fn set_user_data<T: UserData + ScriptData>(&self, data: &mut T) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptFunction_SetUserData)(
                self.inner,
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

    /// Gets user data from this function.
    ///
    /// # Returns
    /// The user data, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    ///
    /// if let Some(data) = function.get_user_data::<MyUserData>() {
    ///     println!("Found user data: {:?}", data);
    /// }
    /// ```
    pub fn get_user_data<T: UserData + ScriptData>(&self) -> Option<T> {
        unsafe {
            let ptr =
                (self.as_vtable().asIScriptFunction_GetUserData)(self.inner, T::KEY as asPWORD);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    /// Gets the vtable for the underlying AngelScript function.
    ///
    /// # Safety
    /// This function provides direct access to the AngelScript vtable.
    fn as_vtable(&self) -> &asIScriptFunction__bindgen_vtable {
        unsafe { &*(*self.inner).vtable_ }
    }
}

unsafe impl Send for Function {}
unsafe impl Sync for Function {}

/// Information about where a function was declared.
#[derive(Debug, Clone)]
pub struct DeclaredAtInfo {
    /// The script section name where the function was declared
    pub script_section: Option<&'static str>,
    /// The row (line) number where the function was declared
    pub row: i32,
    /// The column number where the function was declared
    pub col: i32,
}

/// Information about a local variable in a function.
#[derive(Debug, Clone)]
pub struct VarInfo {
    /// The variable name
    pub name: Option<&'static str>,
    /// The type ID of the variable
    pub type_id: i32,
}

/// Information about a function parameter.
#[derive(Debug, Clone)]
pub struct ParamInfo {
    /// The type ID of the parameter
    pub type_id: i32,
    /// Flags describing the parameter (reference, const, etc.)
    pub flags: u32,
    /// The parameter name
    pub name: Option<&'static str>,
    /// The default argument value (if any)
    pub default_arg: Option<&'static str>,
}

/// Re-export of the JIT function type from AngelScript.
///
/// This type represents a pointer to a JIT-compiled native function
/// that can be used instead of interpreting bytecode.
pub type JITFunction = asJITFunction;
