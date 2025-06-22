use crate::core::context::Context;
use crate::core::engine::Engine;
use crate::core::error::{ScriptError, ScriptResult};
use crate::core::function::Function;
use crate::core::typeinfo::TypeInfo;
use crate::types::enums::TypeId;
use crate::types::script_data::ScriptData;
use crate::types::user_data::UserData;
use angelscript_sys::*;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::ptr::NonNull;

/// A script module containing compiled AngelScript code.
///
/// A Module represents a compilation unit in AngelScript. It contains script functions,
/// global variables, classes, interfaces, and other declarations. Modules can import
/// functions from other modules and can be compiled independently.
///
/// # Module Lifecycle
///
/// 1. **Creation**: Modules are created through `Engine::get_module()`
/// 2. **Population**: Add script sections using `add_script_section()`
/// 3. **Compilation**: Compile the module using `build()`
/// 4. **Execution**: Get functions and execute them through contexts
/// 5. **Cleanup**: Modules are automatically cleaned up when the engine is destroyed
///
/// # Compilation Process
///
/// ```rust
/// use angelscript_rs::{Engine, GetModuleFlags};
///
/// let engine = Engine::create()?;
/// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
///
/// // Add script sections
/// module.add_script_section("main", r#"
///     int global_var = 42;
///
///     int add(int a, int b) {
///         return a + b;
///     }
///
///     class MyClass {
///         int value;
///         MyClass(int v) { value = v; }
///         int getValue() const { return value; }
///     }
/// "#)?;
///
/// // Compile the module
/// module.build()?;
///
/// // Now you can use the compiled code
/// let add_func = module.get_function_by_name("add")
///     .expect("Function 'add' should exist");
/// ```
///
/// # Namespaces and Access Control
///
/// Modules support namespaces and access masks for organizing code:
///
/// ```rust
/// // Set default namespace for subsequent declarations
/// module.set_default_namespace("MyNamespace")?;
///
/// module.add_script_section("namespaced", r#"
///     void namespacedFunction() {
///         // This function is in MyNamespace
///     }
/// "#)?;
///
/// // Set access mask to control visibility
/// let old_mask = module.set_access_mask(0x01);
/// ```
///
/// # Global Variables
///
/// Modules can contain global variables that persist across function calls:
///
/// ```rust
/// module.add_script_section("globals", r#"
///     int counter = 0;
///     string message = "Hello";
/// "#)?;
/// module.build()?;
///
/// // Access global variables
/// let counter_index = module.get_global_var_index_by_name("counter")
///     .expect("Global variable 'counter' should exist");
///
/// let counter_addr = module.get_address_of_global_var::<i32>(counter_index as u32)
///     .expect("Should get address of counter");
/// ```
///
/// # Module Imports
///
/// Modules can import functions from other modules:
///
/// ```rust
/// // In the importing module's script
/// module.add_script_section("imports", r#"
///     import int utilityFunction(int) from "UtilityModule";
///
///     void myFunction() {
///         int result = utilityFunction(42);
///     }
/// "#)?;
///
/// // Bind the imported function to an actual implementation
/// let import_index = module.get_imported_function_index_by_decl("int utilityFunction(int)")
///     .expect("Import should exist");
///
/// let actual_function = utility_module.get_function_by_name("utilityFunction")
///     .expect("Function should exist in utility module");
///
/// module.bind_imported_function(import_index as u32, &actual_function)?;
/// ```
#[derive(Debug, Clone)]
pub struct Module {
    inner: *mut asIScriptModule,
}

impl Module {
    /// Creates a Module wrapper from a raw AngelScript module pointer.
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid and the module is properly initialized.
    ///
    /// # Arguments
    /// * `module` - Raw pointer to AngelScript module
    ///
    /// # Returns
    /// A new Module wrapper
    pub(crate) fn from_raw(module: *mut asIScriptModule) -> Self {
        Self { inner: module }
    }

    // ========== VTABLE ORDER (matches asIScriptModule__bindgen_vtable) ==========

    /// Gets the engine that owns this module.
    ///
    /// # Returns
    /// The engine instance or an error if the engine is not available
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// let module_engine = module.get_engine()?;
    /// assert_eq!(Engine::get_library_version(), module_engine.get_library_version());
    /// ```
    pub fn get_engine(&self) -> ScriptResult<Engine> {
        unsafe {
            let result: *mut asIScriptEngine =
                (self.as_vtable().asIScriptModule_GetEngine)(self.inner);
            let ptr = NonNull::new(result).ok_or(ScriptError::NullPointer)?;
            Ok(Engine::from_raw(ptr))
        }
    }

    /// Sets the name of this module.
    ///
    /// # Arguments
    /// * `name` - The new name for the module
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("TempName", GetModuleFlags::CreateIfNotExists)?;
    /// module.set_name("MyModule")?;
    /// assert_eq!(module.get_name(), Some("MyModule"));
    /// ```
    pub fn set_name(&self, name: &str) -> ScriptResult<()> {
        let c_name = CString::new(name)?;
        unsafe {
            (self.as_vtable().asIScriptModule_SetName)(self.inner, c_name.as_ptr());
        }
        Ok(())
    }

    /// Gets the name of this module.
    ///
    /// # Returns
    /// The module name, or None if not set
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// assert_eq!(module.get_name(), Some("MyModule"));
    /// ```
    pub fn get_name(&self) -> Option<&str> {
        unsafe {
            let name = (self.as_vtable().asIScriptModule_GetName)(self.inner);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    /// Discards this module, removing it from the engine.
    ///
    /// After calling this method, the module should not be used anymore.
    /// The engine will clean up all resources associated with this module.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("TempModule", GetModuleFlags::CreateIfNotExists)?;
    /// // ... use the module ...
    /// module.discard(); // Clean up the module
    /// ```
    pub fn discard(&self) {
        unsafe {
            (self.as_vtable().asIScriptModule_Discard)(self.inner);
        }
    }

    /// Adds a script section to the module.
    ///
    /// Script sections are pieces of AngelScript code that will be compiled together
    /// when `build()` is called. Multiple sections can be added to build up a complete
    /// module.
    ///
    /// # Arguments
    /// * `name` - Name for this script section (used in error messages)
    /// * `code` - The AngelScript source code
    /// * `line_offset` - Line number offset for error reporting
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    ///
    /// // Add a function
    /// module.add_script_section("functions", r#"
    ///     int add(int a, int b) {
    ///         return a + b;
    ///     }
    /// "#, 0)?;
    ///
    /// // Add a class
    /// module.add_script_section("classes", r#"
    ///     class MyClass {
    ///         int value;
    ///         MyClass(int v) { value = v; }
    ///     }
    /// "#, 0)?;
    ///
    /// // Compile all sections together
    /// module.build()?;
    /// ```
    pub fn add_script_section(&self, name: &str, code: &str, line_offset: i32) -> ScriptResult<()> {
        let c_name = CString::new(name)?;
        let c_code = CString::new(code)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_AddScriptSection)(
                self.inner,
                c_name.as_ptr(),
                c_code.as_ptr(),
                code.len(),
                line_offset,
            ))
        }
    }

    /// Compiles all script sections in the module.
    ///
    /// This method compiles all previously added script sections into executable
    /// bytecode. After successful compilation, functions and other declarations
    /// become available for use.
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Compilation Errors
    ///
    /// If compilation fails, the error will contain information about syntax errors,
    /// type mismatches, or other compilation issues. Use a message callback on the
    /// engine to get detailed error information.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    ///
    /// module.add_script_section("code", r#"
    ///     int factorial(int n) {
    ///         if (n <= 1) return 1;
    ///         return n * factorial(n - 1);
    ///     }
    /// "#, 0)?;
    ///
    /// match module.build() {
    ///     Ok(()) => println!("Module compiled successfully"),
    ///     Err(e) => eprintln!("Compilation failed: {}", e),
    /// }
    /// ```
    pub fn build(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptModule_Build)(self.inner)) }
    }

    /// Compiles a single function from source code.
    ///
    /// This method compiles a standalone function without adding it to the module's
    /// script sections. The function is immediately available after compilation.
    ///
    /// # Arguments
    /// * `section_name` - Name for error reporting
    /// * `code` - The function source code
    /// * `line_offset` - Line number offset for error reporting
    /// * `compile_flags` - Compilation flags
    ///
    /// # Returns
    /// The compiled function or an error
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    ///
    /// let function = module.compile_function(
    ///     "dynamic_func",
    ///     "int multiply(int a, int b) { return a * b; }",
    ///     0,
    ///     0
    /// )?;
    ///
    /// println!("Compiled function: {}", function.get_name().unwrap_or("unnamed"));
    /// ```
    pub fn compile_function(
        &self,
        section_name: &str,
        code: &str,
        line_offset: i32,
        compile_flags: asDWORD,
    ) -> ScriptResult<Function> {
        let c_section_name = CString::new(section_name)?;
        let c_code = CString::new(code)?;
        let mut out_func: *mut asIScriptFunction = ptr::null_mut();

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_CompileFunction)(
                self.inner,
                c_section_name.as_ptr(),
                c_code.as_ptr(),
                line_offset,
                compile_flags,
                &mut out_func,
            ))?;

            if out_func.is_null() {
                Err(ScriptError::NullPointer)
            } else {
                Ok(Function::from_raw(out_func))
            }
        }
    }

    /// Compiles a global variable from source code.
    ///
    /// This method compiles a standalone global variable declaration and adds it
    /// to the module.
    ///
    /// # Arguments
    /// * `section_name` - Name for error reporting
    /// * `code` - The variable declaration
    /// * `line_offset` - Line number offset for error reporting
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    ///
    /// module.compile_global_var(
    ///     "dynamic_var",
    ///     "int dynamic_counter = 100;",
    ///     0
    /// )?;
    ///
    /// let var_index = module.get_global_var_index_by_name("dynamic_counter")
    ///     .expect("Variable should exist");
    /// ```
    pub fn compile_global_var(
        &self,
        section_name: &str,
        code: &str,
        line_offset: i32,
    ) -> ScriptResult<()> {
        let c_section_name = CString::new(section_name)?;
        let c_code = CString::new(code)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_CompileGlobalVar)(
                self.inner,
                c_section_name.as_ptr(),
                c_code.as_ptr(),
                line_offset,
            ))
        }
    }

    /// Sets the access mask for this module.
    ///
    /// The access mask controls which engine registrations are visible to this module.
    /// Only registrations with matching access mask bits will be accessible.
    ///
    /// # Arguments
    /// * `access_mask` - The new access mask
    ///
    /// # Returns
    /// The previous access mask
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    ///
    /// // Allow access to registrations with bits 0 and 1 set
    /// let old_mask = module.set_access_mask(0x03);
    ///
    /// // Now only functions/types registered with compatible access masks are visible
    /// ```
    pub fn set_access_mask(&self, access_mask: asDWORD) -> asDWORD {
        unsafe { (self.as_vtable().asIScriptModule_SetAccessMask)(self.inner, access_mask) }
    }

    /// Sets the default namespace for subsequent declarations.
    ///
    /// # Arguments
    /// * `namespace` - The namespace name (empty string for global namespace)
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    ///
    /// module.set_default_namespace("Math")?;
    /// module.add_script_section("math_funcs", r#"
    ///     float pi() { return 3.14159; }  // This will be in Math namespace
    /// "#, 0)?;
    ///
    /// module.set_default_namespace("")?; // Back to global namespace
    /// ```
    pub fn set_default_namespace(&self, namespace: &str) -> ScriptResult<()> {
        let c_namespace = CString::new(namespace)?;
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_SetDefaultNamespace)(
                self.inner,
                c_namespace.as_ptr(),
            ))
        }
    }

    /// Gets the current default namespace.
    ///
    /// # Returns
    /// The current default namespace, or None if in global namespace
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    ///
    /// assert_eq!(module.get_default_namespace(), None); // Global namespace
    ///
    /// module.set_default_namespace("MyNamespace")?;
    /// assert_eq!(module.get_default_namespace(), Some("MyNamespace"));
    /// ```
    pub fn get_default_namespace(&self) -> Option<&str> {
        unsafe {
            let namespace = (self.as_vtable().asIScriptModule_GetDefaultNamespace)(self.inner);
            if namespace.is_null() {
                None
            } else {
                CStr::from_ptr(namespace).to_str().ok()
            }
        }
    }

    /// Gets the number of functions in this module.
    ///
    /// # Returns
    /// The number of functions
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("funcs", r#"
    ///     void func1() {}
    ///     void func2() {}
    /// "#, 0)?;
    /// module.build()?;
    ///
    /// assert_eq!(module.get_function_count(), 2);
    /// ```
    pub fn get_function_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptModule_GetFunctionCount)(self.inner) }
    }

    /// Gets a function by its index.
    ///
    /// # Arguments
    /// * `index` - The function index (0-based)
    ///
    /// # Returns
    /// The function, or None if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("funcs", "void test() {}", 0)?;
    /// module.build()?;
    ///
    /// if let Some(func) = module.get_function_by_index(0) {
    ///     println!("First function: {}", func.get_name().unwrap_or("unnamed"));
    /// }
    /// ```
    pub fn get_function_by_index(&self, index: asUINT) -> Option<Function> {
        unsafe {
            let func = (self.as_vtable().asIScriptModule_GetFunctionByIndex)(self.inner, index);
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    /// Gets a function by its declaration.
    ///
    /// # Arguments
    /// * `decl` - The function declaration (e.g., "int add(int, int)")
    ///
    /// # Returns
    /// The function, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("funcs", "int add(int a, int b) { return a + b; }", 0)?;
    /// module.build()?;
    ///
    /// if let Some(func) = module.get_function_by_decl("int add(int, int)") {
    ///     println!("Found add function");
    /// }
    /// ```
    pub fn get_function_by_decl(&self, decl: &str) -> Option<Function> {
        let c_decl = match CString::new(decl) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let func =
                (self.as_vtable().asIScriptModule_GetFunctionByDecl)(self.inner, c_decl.as_ptr());
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    /// Gets a function by its name.
    ///
    /// If multiple functions have the same name (overloads), this returns the first one found.
    /// Use `get_function_by_decl()` for precise matching.
    ///
    /// # Arguments
    /// * `name` - The function name
    ///
    /// # Returns
    /// The function, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("funcs", "void hello() { print('Hello!'); }", 0)?;
    /// module.build()?;
    ///
    /// if let Some(func) = module.get_function_by_name("hello") {
    ///     println!("Found hello function");
    /// }
    /// ```
    pub fn get_function_by_name(&self, name: &str) -> Option<Function> {
        let c_name = match CString::new(name) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let func =
                (self.as_vtable().asIScriptModule_GetFunctionByName)(self.inner, c_name.as_ptr());
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    /// Removes a function from the module.
    ///
    /// # Arguments
    /// * `func` - The function to remove
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("funcs", "void temp() {}", 0)?;
    /// module.build()?;
    ///
    /// if let Some(func) = module.get_function_by_name("temp") {
    ///     module.remove_function(&func)?;
    /// }
    /// ```
    pub fn remove_function(&self, func: &Function) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_RemoveFunction)(
                self.inner,
                func.as_raw(),
            ))
        }
    }

    /// Resets all global variables to their initial values.
    ///
    /// # Arguments
    /// * `ctx` - Optional context for executing initialization code
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("globals", "int counter = 0;", 0)?;
    /// module.build()?;
    ///
    /// // ... counter gets modified during execution ...
    ///
    /// // Reset to initial value
    /// module.reset_global_vars(None)?;
    /// ```
    pub fn reset_global_vars(&self, ctx: Option<&Context>) -> ScriptResult<()> {
        unsafe {
            let ctx_ptr = match ctx {
                Some(context) => context.as_ptr(),
                None => ptr::null_mut(),
            };
            ScriptError::from_code((self.as_vtable().asIScriptModule_ResetGlobalVars)(
                self.inner, ctx_ptr,
            ))
        }
    }

    /// Gets the number of global variables in this module.
    ///
    /// # Returns
    /// The number of global variables
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("globals", r#"
    ///     int var1 = 10;
    ///     string var2 = "hello";
    /// "#, 0)?;
    /// module.build()?;
    ///
    /// assert_eq!(module.get_global_var_count(), 2);
    /// ```
    pub fn get_global_var_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptModule_GetGlobalVarCount)(self.inner) }
    }

    /// Gets the index of a global variable by name.
    ///
    /// # Arguments
    /// * `name` - The variable name
    ///
    /// # Returns
    /// The variable index, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("globals", "int myVar = 42;", 0)?;
    /// module.build()?;
    ///
    /// if let Some(index) = module.get_global_var_index_by_name("myVar") {
    ///     println!("Variable myVar is at index {}", index);
    /// }
    /// ```
    pub fn get_global_var_index_by_name(&self, name: &str) -> Option<i32> {
        let c_name = match CString::new(name) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let index = (self.as_vtable().asIScriptModule_GetGlobalVarIndexByName)(
                self.inner,
                c_name.as_ptr(),
            );
            if index < 0 { None } else { Some(index) }
        }
    }

    /// Gets the index of a global variable by declaration.
    ///
    /// # Arguments
    /// * `decl` - The variable declaration (e.g., "int myVar")
    ///
    /// # Returns
    /// The variable index, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("globals", "const string message = 'hello';", 0)?;
    /// module.build()?;
    ///
    /// if let Some(index) = module.get_global_var_index_by_decl("const string message") {
    ///     println!("Variable message is at index {}", index);
    /// }
    /// ```
    pub fn get_global_var_index_by_decl(&self, decl: &str) -> Option<i32> {
        let c_decl = match CString::new(decl) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let index = (self.as_vtable().asIScriptModule_GetGlobalVarIndexByDecl)(
                self.inner,
                c_decl.as_ptr(),
            );
            if index < 0 { None } else { Some(index) }
        }
    }

    /// Gets the declaration string for a global variable.
    ///
    /// # Arguments
    /// * `index` - The variable index
    /// * `include_namespace` - Whether to include the namespace in the declaration
    ///
    /// # Returns
    /// The variable declaration, or None if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("globals", "int myVar = 42;", 0)?;
    /// module.build()?;
    ///
    /// for i in 0..module.get_global_var_count() {
    ///     if let Some(decl) = module.get_global_var_declaration(i, true) {
    ///         println!("Global variable {}: {}", i, decl);
    ///     }
    /// }
    /// ```
    pub fn get_global_var_declaration(
        &self,
        index: asUINT,
        include_namespace: bool,
    ) -> Option<&str> {
        unsafe {
            let decl = (self.as_vtable().asIScriptModule_GetGlobalVarDeclaration)(
                self.inner,
                index,
                include_namespace,
            );
            if decl.is_null() {
                None
            } else {
                CStr::from_ptr(decl).to_str().ok()
            }
        }
    }

    /// Gets detailed information about a global variable.
    ///
    /// # Arguments
    /// * `index` - The variable index
    ///
    /// # Returns
    /// Variable information or an error if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("globals", "const int myVar = 42;", 0)?;
    /// module.build()?;
    ///
    /// for i in 0..module.get_global_var_count() {
    ///     match module.get_global_var(i) {
    ///         Ok(info) => {
    ///             println!("Variable: name={:?}, type_id={}, const={}",
    ///                      info.name, info.type_id, info.is_const);
    ///         }
    ///         Err(e) => eprintln!("Error getting variable info: {}", e),
    ///     }
    /// }
    /// ```
    pub fn get_global_var(&self, index: asUINT) -> ScriptResult<ModuleGlobalVarInfo> {
        let mut name: *const c_char = ptr::null();
        let mut namespace: *const c_char = ptr::null();
        let mut type_id: i32 = 0;
        let mut is_const: bool = false;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_GetGlobalVar)(
                self.inner,
                index,
                &mut name,
                &mut namespace,
                &mut type_id,
                &mut is_const,
            ))?;

            Ok(ModuleGlobalVarInfo {
                name: if name.is_null() {
                    None
                } else {
                    CStr::from_ptr(name).to_str().ok().map(|s| s.to_string())
                },
                namespace: if namespace.is_null() {
                    None
                } else {
                    CStr::from_ptr(namespace)
                        .to_str()
                        .ok()
                        .map(|s| s.to_string())
                },
                type_id,
                is_const,
            })
        }
    }

    /// Gets the address of a global variable.
    ///
    /// This allows direct access to the variable's memory for reading and writing.
    ///
    /// # Arguments
    /// * `index` - The variable index
    ///
    /// # Returns
    /// A pointer to the variable's memory, or None if the index is invalid
    ///
    /// # Safety
    ///
    /// The returned pointer must be used carefully to avoid memory corruption.
    /// Ensure the type matches the variable's actual type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("globals", "int counter = 10;", 0)?;
    /// module.build()?;
    ///
    /// let index = module.get_global_var_index_by_name("counter")
    ///     .expect("Variable should exist") as u32;
    ///
    /// if let Some(counter_ptr) = module.get_address_of_global_var::<i32>(index) {
    ///     // Read the current value
    ///     // let current_value = unsafe { *counter_ptr };
    ///     //
    ///     // Modify the value
    ///     // unsafe { *counter_ptr = 20; }
    /// }
    /// ```
    pub fn get_address_of_global_var<T: ScriptData>(&self, index: asUINT) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptModule_GetAddressOfGlobalVar)(self.inner, index);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    /// Removes a global variable from the module.
    ///
    /// # Arguments
    /// * `index` - The variable index
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("globals", "int tempVar = 0;", 0)?;
    /// module.build()?;
    ///
    /// if let Some(index) = module.get_global_var_index_by_name("tempVar") {
    ///     module.remove_global_var(index as u32)?;
    /// }
    /// ```
    pub fn remove_global_var(&self, index: asUINT) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_RemoveGlobalVar)(
                self.inner, index,
            ))
        }
    }

    /// Gets the number of object types declared in this module.
    ///
    /// # Returns
    /// The number of object types
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("classes", r#"
    ///     class MyClass {}
    ///     class AnotherClass {}
    /// "#, 0)?;
    /// module.build()?;
    ///
    /// assert_eq!(module.get_object_type_count(), 2);
    /// ```
    pub fn get_object_type_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptModule_GetObjectTypeCount)(self.inner) }
    }

    /// Gets an object type by index.
    ///
    /// # Arguments
    /// * `index` - The type index (0-based)
    ///
    /// # Returns
    /// The type information, or None if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("classes", "class MyClass {}", 0)?;
    /// module.build()?;
    ///
    /// if let Some(type_info) = module.get_object_type_by_index(0) {
    ///     println!("First class: {}", type_info.get_name().unwrap_or("unnamed"));
    /// }
    /// ```
    pub fn get_object_type_by_index(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let type_info =
                (self.as_vtable().asIScriptModule_GetObjectTypeByIndex)(self.inner, index);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    /// Gets a type ID by declaration.
    ///
    /// # Arguments
    /// * `decl` - The type declaration (e.g., "MyClass", "int[]")
    ///
    /// # Returns
    /// The type ID, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("classes", "class MyClass {}", 0)?;
    /// module.build()?;
    ///
    /// if let Some(type_id) = module.get_type_id_by_decl("MyClass") {
    ///     println!("MyClass has type ID: {}", type_id);
    /// }
    /// ```
    pub fn get_type_id_by_decl(&self, decl: &str) -> Option<TypeId> {
        let c_decl = match CString::new(decl) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let type_id =
                (self.as_vtable().asIScriptModule_GetTypeIdByDecl)(self.inner, c_decl.as_ptr());
            if type_id < 0 {
                None
            } else {
                Some(TypeId::from(type_id as u32))
            }
        }
    }

    /// Gets type information by name.
    ///
    /// # Arguments
    /// * `name` - The type name
    ///
    /// # Returns
    /// The type information, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("classes", "class MyClass {}", 0)?;
    /// module.build()?;
    ///
    /// if let Some(type_info) = module.get_type_info_by_name("MyClass") {
    ///     println!("Found type: {}", type_info.get_name().unwrap_or("unnamed"));
    /// }
    /// ```
    pub fn get_type_info_by_name(&self, name: &str) -> Option<TypeInfo> {
        let c_name = match CString::new(name) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let type_info =
                (self.as_vtable().asIScriptModule_GetTypeInfoByName)(self.inner, c_name.as_ptr());
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
    /// The type information, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("classes", "class MyClass {}", 0)?;
    /// module.build()?;
    ///
    /// if let Some(type_info) = module.get_type_info_by_decl("MyClass") {
    ///     println!("Found type: {}", type_info.get_name().unwrap_or("unnamed"));
    /// }
    /// ```
    pub fn get_type_info_by_decl(&self, decl: &str) -> Option<TypeInfo> {
        let c_decl = match CString::new(decl) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let type_info =
                (self.as_vtable().asIScriptModule_GetTypeInfoByDecl)(self.inner, c_decl.as_ptr());
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    /// Gets the number of enums declared in this module.
    ///
    /// # Returns
    /// The number of enums
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("enums", r#"
    ///     enum Color { Red, Green, Blue }
    ///     enum Status { Active, Inactive }
    /// "#, 0)?;
    /// module.build()?;
    ///
    /// assert_eq!(module.get_enum_count(), 2);
    /// ```
    pub fn get_enum_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptModule_GetEnumCount)(self.inner) }
    }

    /// Gets an enum by index.
    ///
    /// # Arguments
    /// * `index` - The enum index (0-based)
    ///
    /// # Returns
    /// The enum type information, or None if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("enums", "enum Color { Red, Green, Blue }", 0)?;
    /// module.build()?;
    ///
    /// if let Some(enum_info) = module.get_enum_by_index(0) {
    ///     println!("First enum: {}", enum_info.get_name().unwrap_or("unnamed"));
    /// }
    /// ```
    pub fn get_enum_by_index(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let type_info = (self.as_vtable().asIScriptModule_GetEnumByIndex)(self.inner, index);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    /// Gets the number of typedefs declared in this module.
    ///
    /// # Returns
    /// The number of typedefs
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("typedefs", r#"
    ///     typedef int MyInt;
    ///     typedef string MyString;
    /// "#, 0)?;
    /// module.build()?;
    ///
    /// assert_eq!(module.get_typedef_count(), 2);
    /// ```
    pub fn get_typedef_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptModule_GetTypedefCount)(self.inner) }
    }

    /// Gets a typedef by index.
    ///
    /// # Arguments
    /// * `index` - The typedef index (0-based)
    ///
    /// # Returns
    /// The typedef type information, or None if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("typedefs", "typedef int MyInt;", 0)?;
    /// module.build()?;
    ///
    /// if let Some(typedef_info) = module.get_typedef_by_index(0) {
    ///     println!("First typedef: {}", typedef_info.get_name().unwrap_or("unnamed"));
    /// }
    /// ```
    pub fn get_typedef_by_index(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let type_info = (self.as_vtable().asIScriptModule_GetTypedefByIndex)(self.inner, index);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    /// Gets the number of imported functions in this module.
    ///
    /// # Returns
    /// The number of imported functions
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("imports", r#"
    ///     import void func1() from "OtherModule";
    ///     import int func2(int) from "OtherModule";
    /// "#, 0)?;
    /// module.build()?;
    ///
    /// assert_eq!(module.get_imported_function_count(), 2);
    /// ```
    pub fn get_imported_function_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptModule_GetImportedFunctionCount)(self.inner) }
    }

    /// Gets the index of an imported function by declaration.
    ///
    /// # Arguments
    /// * `decl` - The function declaration
    ///
    /// # Returns
    /// The import index, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("imports", r#"
    ///     import void myFunc() from "OtherModule";
    /// "#, 0)?;
    /// module.build()?;
    ///
    /// if let Some(index) = module.get_imported_function_index_by_decl("void myFunc()") {
    ///     println!("Import myFunc is at index {}", index);
    /// }
    /// ```
    pub fn get_imported_function_index_by_decl(&self, decl: &str) -> Option<i32> {
        let c_decl = match CString::new(decl) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let index = (self
                .as_vtable()
                .asIScriptModule_GetImportedFunctionIndexByDecl)(
                self.inner, c_decl.as_ptr()
            );
            if index < 0 { None } else { Some(index) }
        }
    }

    /// Gets the declaration of an imported function.
    ///
    /// # Arguments
    /// * `import_index` - The import index
    ///
    /// # Returns
    /// The function declaration, or None if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("imports", "import void myFunc() from 'OtherModule';", 0)?;
    /// module.build()?;
    ///
    /// for i in 0..module.get_imported_function_count() {
    ///     if let Some(decl) = module.get_imported_function_declaration(i) {
    ///         println!("Import {}: {}", i, decl);
    ///     }
    /// }
    /// ```
    pub fn get_imported_function_declaration(&self, import_index: asUINT) -> Option<&str> {
        unsafe {
            let decl = (self
                .as_vtable()
                .asIScriptModule_GetImportedFunctionDeclaration)(
                self.inner, import_index
            );
            if decl.is_null() {
                None
            } else {
                CStr::from_ptr(decl).to_str().ok()
            }
        }
    }

    /// Gets the source module name for an imported function.
    ///
    /// # Arguments
    /// * `import_index` - The import index
    ///
    /// # Returns
    /// The source module name, or None if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("imports", "import void myFunc() from 'UtilityModule';", 0)?;
    /// module.build()?;
    ///
    /// for i in 0..module.get_imported_function_count() {
    ///     if let Some(source) = module.get_imported_function_source_module(i) {
    ///         println!("Import {} comes from module: {}", i, source);
    ///     }
    /// }
    /// ```
    pub fn get_imported_function_source_module(&self, import_index: asUINT) -> Option<&str> {
        unsafe {
            let module = (self
                .as_vtable()
                .asIScriptModule_GetImportedFunctionSourceModule)(
                self.inner, import_index
            );
            if module.is_null() {
                None
            } else {
                CStr::from_ptr(module).to_str().ok()
            }
        }
    }

    /// Binds an imported function to an actual function implementation.
    ///
    /// # Arguments
    /// * `import_index` - The import index
    /// * `func` - The function to bind to
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// // In the importing module
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("imports", "import int add(int, int) from 'MathModule';", 0)?;
    /// module.build()?;
    ///
    /// // In the source module
    /// let math_module = engine.get_module("MathModule", GetModuleFlags::CreateIfNotExists)?;
    /// math_module.add_script_section("math", "int add(int a, int b) { return a + b; }", 0)?;
    /// math_module.build()?;
    ///
    /// // Bind the import
    /// let import_index = module.get_imported_function_index_by_decl("int add(int, int)")
    ///     .expect("Import should exist") as u32;
    /// let add_func = math_module.get_function_by_name("add")
    ///     .expect("Function should exist");
    ///
    /// module.bind_imported_function(import_index, &add_func)?;
    /// ```
    pub fn bind_imported_function(
        &self,
        import_index: asUINT,
        func: &Function,
    ) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_BindImportedFunction)(
                self.inner,
                import_index,
                func.as_raw(),
            ))
        }
    }

    /// Unbinds an imported function.
    ///
    /// # Arguments
    /// * `import_index` - The import index
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// let import_index = 0; // Previously bound import
    /// module.unbind_imported_function(import_index)?;
    /// ```
    pub fn unbind_imported_function(&self, import_index: asUINT) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_UnbindImportedFunction)(
                self.inner,
                import_index,
            ))
        }
    }

    /// Automatically binds all imported functions.
    ///
    /// This attempts to find and bind all imported functions by looking for
    /// matching functions in other modules loaded in the same engine.
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// // After loading all modules that provide imported functions
    /// module.bind_all_imported_functions()?;
    /// ```
    pub fn bind_all_imported_functions(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_BindAllImportedFunctions)(
                self.inner,
            ))
        }
    }

    /// Unbinds all imported functions.
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// module.unbind_all_imported_functions()?;
    /// ```
    pub fn unbind_all_imported_functions(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self
                .as_vtable()
                .asIScriptModule_UnbindAllImportedFunctions)(
                self.inner
            ))
        }
    }

    /// Saves the compiled bytecode to a binary stream.
    ///
    /// # Arguments
    /// * `out` - The output stream
    /// * `strip_debug_info` - Whether to remove debug information
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section("code", "void test() {}", 0)?;
    /// module.build()?;
    ///
    /// // Save bytecode (implementation depends on BinaryStream)
    /// // let mut stream = MyBinaryStream::new();
    /// // module.save_byte_code(&mut stream, false)?;
    /// ```
    pub fn save_byte_code(
        &self,
        out: &mut BinaryStream,
        strip_debug_info: bool,
    ) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_SaveByteCode)(
                self.inner,
                out.as_ptr(),
                strip_debug_info,
            ))
        }
    }

    /// Loads compiled bytecode from a binary stream.
    ///
    /// # Arguments
    /// * `input` - The input stream
    ///
    /// # Returns
    /// true if debug info was stripped, false otherwise, or an error
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    ///
    /// // Load bytecode (implementation depends on BinaryStream)
    /// // let mut stream = MyBinaryStream::from_file("module.asb")?;
    /// // let was_stripped = module.load_byte_code(&mut stream)?;
    /// //
    /// // if was_stripped {
    /// //     println!("Debug information was not available");
    /// // }
    /// ```
    pub fn load_byte_code(&self, input: &mut BinaryStream) -> ScriptResult<bool> {
        let mut was_debug_info_stripped: bool = false;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_LoadByteCode)(
                self.inner,
                input.as_ptr(),
                &mut was_debug_info_stripped,
            ))?;
        }

        Ok(was_debug_info_stripped)
    }

    /// Sets user data on this module.
    ///
    /// User data allows applications to associate custom data with modules.
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
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// let mut my_data = MyUserData::new();
    ///
    /// if let Some(old_data) = module.set_user_data(&mut my_data) {
    ///     println!("Replaced existing user data");
    /// }
    /// ```
    pub fn set_user_data<T: UserData + ScriptData>(&self, data: &mut T) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptModule_SetUserData)(
                self.inner,
                data.to_script_ptr(),
                T::KEY,
            );
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    /// Gets user data from this module.
    ///
    /// # Returns
    /// The user data, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    ///
    /// if let Some(data) = module.get_user_data::<MyUserData>() {
    ///     println!("Found user data: {:?}", data);
    /// }
    /// ```
    pub fn get_user_data<T: UserData + ScriptData>(&self) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptModule_GetUserData)(self.inner, T::KEY);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    /// Gets the vtable for the underlying AngelScript module.
    ///
    /// # Safety
    /// This function provides direct access to the AngelScript vtable.
    fn as_vtable(&self) -> &asIScriptModule__bindgen_vtable {
        unsafe { &*(*self.inner).vtable_ }
    }
}

// Module doesn't manage its own lifetime - the engine does
unsafe impl Send for Module {}
unsafe impl Sync for Module {}

// ========== ADDITIONAL TYPES ==========

/// Information about a global variable in a module.
#[derive(Debug, Clone)]
pub struct ModuleGlobalVarInfo {
    /// The variable name
    pub name: Option<String>,
    /// The namespace the variable belongs to
    pub namespace: Option<String>,
    /// The type ID of the variable
    pub type_id: i32,
    /// Whether the variable is const
    pub is_const: bool,
}

/// Wrapper for binary stream operations.
///
/// This is used for saving and loading compiled bytecode.
/// The actual implementation depends on the specific binary stream
/// implementation provided by the application.
#[derive(Debug)]
pub struct BinaryStream {
    inner: *mut asIBinaryStream,
}

impl BinaryStream {
    /// Gets the raw AngelScript binary stream pointer.
    ///
    /// # Safety
    /// This function provides direct access to the AngelScript binary stream pointer.
    pub(crate) fn as_ptr(&self) -> *mut asIBinaryStream {
        self.inner
    }
}

// ========== CONVENIENCE METHODS ==========

impl Module {
    /// Gets all functions in the module.
    ///
    /// This is a convenience method that collects all functions into a vector.
    ///
    /// # Returns
    /// A vector containing all functions in the module
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section_simple("funcs", r#"
    ///     void func1() {}
    ///     void func2() {}
    ///     int func3() { return 42; }
    /// "#)?;
    /// module.build()?;
    ///
    /// let functions = module.get_all_functions();
    /// println!("Module has {} functions", functions.len());
    ///
    /// for func in functions {
    ///     println!("Function: {}", func.get_name().unwrap_or("unnamed"));
    /// }
    /// ```
    pub fn get_all_functions(&self) -> Vec<Function> {
        let count = self.get_function_count();
        (0..count)
            .filter_map(|i| self.get_function_by_index(i))
            .collect()
    }

    /// Gets all global variables in the module.
    ///
    /// This is a convenience method that collects all global variable information into a vector.
    ///
    /// # Returns
    /// A vector containing information about all global variables
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section_simple("vars", r#"
    ///     int counter = 0;
    ///     const string message = "hello";
    ///     float pi = 3.14159;
    /// "#)?;
    /// module.build()?;
    ///
    /// let variables = module.get_all_global_vars();
    /// println!("Module has {} global variables", variables.len());
    ///
    /// for var in variables {
    ///     println!("Variable: {:?} (type_id: {}, const: {})",
    ///              var.name, var.type_id, var.is_const);
    /// }
    /// ```
    pub fn get_all_global_vars(&self) -> Vec<ModuleGlobalVarInfo> {
        let count = self.get_global_var_count();
        (0..count)
            .filter_map(|i| self.get_global_var(i).ok())
            .collect()
    }

    /// Gets all object types in the module.
    ///
    /// This is a convenience method that collects all object type information into a vector.
    ///
    /// # Returns
    /// A vector containing all object types
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section_simple("classes", r#"
    ///     class Player {
    ///         string name;
    ///         int score;
    ///     }
    ///
    ///     class Game {
    ///         Player[] players;
    ///     }
    /// "#)?;
    /// module.build()?;
    ///
    /// let types = module.get_all_object_types();
    /// println!("Module has {} object types", types.len());
    ///
    /// for type_info in types {
    ///     println!("Type: {}", type_info.get_name().unwrap_or("unnamed"));
    /// }
    /// ```
    pub fn get_all_object_types(&self) -> Vec<TypeInfo> {
        let count = self.get_object_type_count();
        (0..count)
            .filter_map(|i| self.get_object_type_by_index(i))
            .collect()
    }

    /// Gets all enums in the module.
    ///
    /// This is a convenience method that collects all enum information into a vector.
    ///
    /// # Returns
    /// A vector containing all enums
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section_simple("enums", r#"
    ///     enum Color { Red, Green, Blue }
    ///     enum Direction { North, South, East, West }
    /// "#)?;
    /// module.build()?;
    ///
    /// let enums = module.get_all_enums();
    /// println!("Module has {} enums", enums.len());
    ///
    /// for enum_info in enums {
    ///     println!("Enum: {}", enum_info.get_name().unwrap_or("unnamed"));
    /// }
    /// ```
    pub fn get_all_enums(&self) -> Vec<TypeInfo> {
        let count = self.get_enum_count();
        (0..count)
            .filter_map(|i| self.get_enum_by_index(i))
            .collect()
    }

    /// Gets all typedefs in the module.
    ///
    /// This is a convenience method that collects all typedef information into a vector.
    ///
    /// # Returns
    /// A vector containing all typedefs
    ///
    /// # Examples
    ///
    /// ```rust
    /// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
    /// module.add_script_section_simple("typedefs", r#"
    ///     typedef int PlayerID;
    ///     typedef string PlayerName;
    /// "#)?;
    /// module.build()?;
    ///
    /// let typedefs = module.get_all_typedefs();
    /// println!("Module has {} typedefs", typedefs.len());
    ///
    /// for typedef_info in typedefs {
    ///     println!("Typedef: {}", typedef_info.get_name().unwrap_or("unnamed"));
    /// }
    /// ```
    pub fn get_all_typedefs(&self) -> Vec<TypeInfo> {
        let count = self.get_typedef_count();
        (0..count)
            .filter_map(|i| self.get_typedef_by_index(i))
            .collect()
    }
}
