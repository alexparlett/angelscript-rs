use crate::core::engine::Engine;
use crate::core::error::{ScriptError, ScriptResult};
use crate::core::lockable_shared_bool::LockableSharedBool;
use crate::core::typeinfo::TypeInfo;
use crate::types::script_memory::{ScriptMemoryLocation, Void};
use crate::types::script_data::ScriptData;
use crate::types::user_data::UserData;
use angelscript_sys::*;
use std::ffi::CStr;
use std::ptr::NonNull;

/// A wrapper for AngelScript script object instances.
///
/// `ScriptObject` represents an instance of a script class created in AngelScript.
/// It provides access to the object's properties, type information, reference counting,
/// and other object-specific operations.
///
/// # Object Lifecycle
///
/// Script objects use reference counting for memory management. When the reference
/// count reaches zero, the object is automatically destroyed. The wrapper automatically
/// manages references when created and dropped.
///
/// # Property Access
///
/// Script objects can have properties that can be accessed by index or name.
/// Properties can be primitive types, complex objects, or references.
///
/// # Weak References
///
/// Script objects support weak references through the weak reference flag system.
/// This allows checking if an object is still alive without keeping it alive.
///
/// # Examples
///
/// ## Creating and Using Script Objects
///
/// ```rust
/// use angelscript_rs::{Engine, GetModuleFlags};
///
/// let engine = Engine::create()?;
/// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
///
/// module.add_script_section("classes", r#"
///     class Player {
///         string name;
///         int score;
///         float health;
///
///         Player(const string &in n) {
///             name = n;
///             score = 0;
///             health = 100.0;
///         }
///
///         void addScore(int points) {
///             score += points;
///         }
///     }
/// "#)?;
/// module.build()?;
///
/// // Get the Player type
/// let player_type = module.get_type_info_by_name("Player")
///     .expect("Player type should exist");
///
/// // Create a Player instance
/// let player_obj = engine.create_script_object::<ScriptObject>(&player_type)?;
///
/// // Access properties
/// println!("Player has {} properties", player_obj.get_property_count());
/// for info in player_obj.get_all_properties() {
///     println!("Property: {:?}", info.name);
/// }
/// ```
///
/// ## Property Manipulation
///
/// ```rust
/// // Assuming we have a Player object with name, score, and health properties
/// let player_obj = /* ... get player object ... */;
///
/// // Get property by name
/// if let Some(name) = player_obj.get_property_by_name::<String>("name") {
///     println!("Player name: {}", name);
/// }
///
/// // Set property by name
/// player_obj.set_property_by_name("score", 100i32);
///
/// // Get property by index
/// if let Some(health) = player_obj.get_property::<f32>(2) {
///     println!("Player health: {}", health);
/// }
/// ```
///
/// ## Weak References
///
/// ```rust
/// let player_obj = /* ... get player object ... */;
///
/// // Create a weak reference
/// if let Some(weak_ref) = player_obj.create_weak_ref() {
///     // Check if object is still alive
///     if weak_ref.is_alive() {
///         println!("Player object is still alive");
///
///         // Try to get a strong reference
///         if let Some(strong_ref) = weak_ref.upgrade() {
///             // Use the strong reference
///             println!("Got strong reference to player");
///         }
///     } else {
///         println!("Player object has been destroyed");
///     }
/// }
/// ```
///
/// ## Object Copying
///
/// ```rust
/// let player1 = /* ... get first player object ... */;
/// let player2 = /* ... get second player object ... */;
///
/// // Copy properties from player1 to player2
/// player2.copy_from(&player1)?;
/// ```
#[derive(Debug, Clone)]
pub struct ScriptObject {
    inner: *mut asIScriptObject,
}

impl ScriptObject {
    /// Creates a ScriptObject wrapper from a raw AngelScript pointer.
    ///
    /// # Safety
    /// The pointer must be valid and point to a properly initialized `asIScriptObject`.
    /// This function automatically adds a reference to the object.
    ///
    /// # Arguments
    /// * `ptr` - Raw pointer to AngelScript script object
    ///
    /// # Returns
    /// A new ScriptObject wrapper
    pub(crate) fn from_raw(ptr: *mut asIScriptObject) -> Self {
        let wrapper = Self { inner: ptr };
        wrapper
            .add_ref()
            .expect("Failed to add reference to script object");
        wrapper
    }

    /// Checks if the script object pointer is null.
    ///
    /// # Returns
    /// true if the internal pointer is null, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    /// if obj.is_null() {
    ///     println!("Object pointer is null");
    /// }
    /// ```
    pub fn is_null(&self) -> bool {
        self.inner.is_null()
    }

    // ========== VTABLE ORDER (matches asIScriptObject__bindgen_vtable) ==========

    /// Increments the reference count of the object.
    ///
    /// This is automatically called when the wrapper is created and typically
    /// doesn't need to be called manually.
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    /// obj.add_ref()?; // Manually increment reference count
    /// // Remember to call release() to balance this
    /// ```
    pub fn add_ref(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptObject_AddRef)(self.inner)) }
    }

    /// Decrements the reference count of the object.
    ///
    /// When the reference count reaches zero, the object is destroyed.
    /// This is automatically called when the wrapper is dropped.
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    /// // obj.release() is called automatically when dropped
    /// ```
    pub fn release(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptObject_Release)(self.inner)) }
    }

    /// Gets the weak reference flag for this object.
    ///
    /// The weak reference flag is used to implement weak references that can
    /// detect when an object has been destroyed without keeping it alive.
    ///
    /// # Returns
    /// The weak reference flag, or None if not available
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    /// if let Some(weak_flag) = obj.get_weak_ref_flag() {
    ///     if weak_flag.get() {
    ///         println!("Object has been marked for destruction");
    ///     }
    /// }
    /// ```
    pub fn get_weak_ref_flag(&self) -> Option<LockableSharedBool> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptObject_GetWeakRefFlag)(self.inner);
            if ptr.is_null() {
                None
            } else {
                Some(LockableSharedBool::from_raw(ptr))
            }
        }
    }

    /// Gets the type ID of this object.
    ///
    /// The type ID uniquely identifies the object's type within the engine.
    ///
    /// # Returns
    /// The object's type ID
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    /// let type_id = obj.get_type_id();
    /// println!("Object type ID: {}", type_id);
    /// ```
    pub fn get_type_id(&self) -> i32 {
        unsafe { (self.as_vtable().asIScriptObject_GetTypeId)(self.inner) }
    }

    /// Gets detailed type information for this object.
    ///
    /// This provides access to the object's type metadata, including its name,
    /// methods, and other type-specific information.
    ///
    /// # Returns
    /// The object's type information
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    /// let type_info = obj.get_object_type();
    /// if let Some(name) = type_info.get_name() {
    ///     println!("Object type: {}", name);
    /// }
    /// ```
    pub fn get_object_type(&self) -> TypeInfo {
        unsafe { TypeInfo::from_raw((self.as_vtable().asIScriptObject_GetObjectType)(self.inner)) }
    }

    /// Gets the number of properties in this object.
    ///
    /// # Returns
    /// The number of properties
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    /// let prop_count = obj.get_property_count();
    /// println!("Object has {} properties", prop_count);
    ///
    /// for i in 0..prop_count {
    ///     if let Some(name) = obj.get_property_name(i) {
    ///         println!("Property {}: {}", i, name);
    ///     }
    /// }
    /// ```
    pub fn get_property_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptObject_GetPropertyCount)(self.inner) }
    }

    /// Gets the type ID of a specific property.
    ///
    /// # Arguments
    /// * `prop` - The property index (0-based)
    ///
    /// # Returns
    /// The property's type ID
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    /// let type_id = obj.get_property_type_id(0);
    /// println!("First property has type ID: {}", type_id);
    /// ```
    pub fn get_property_type_id(&self, prop: asUINT) -> i32 {
        unsafe { (self.as_vtable().asIScriptObject_GetPropertyTypeId)(self.inner, prop) }
    }

    /// Gets the name of a specific property.
    ///
    /// # Arguments
    /// * `prop` - The property index (0-based)
    ///
    /// # Returns
    /// The property name, or None if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    /// if let Some(name) = obj.get_property_name(0) {
    ///     println!("First property is named: {}", name);
    /// }
    /// ```
    pub fn get_property_name(&self, prop: asUINT) -> Option<&str> {
        unsafe {
            let name = (self.as_vtable().asIScriptObject_GetPropertyName)(self.inner, prop);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    /// Gets the address of a specific property.
    ///
    /// This provides direct access to the property's memory location,
    /// allowing for reading and writing the property value.
    ///
    /// # Arguments
    /// * `prop` - The property index (0-based)
    ///
    /// # Returns
    /// The property's memory address, or None if the index is invalid
    ///
    /// # Safety
    ///
    /// The returned pointer must be used carefully to avoid memory corruption.
    /// Ensure the type matches the property's actual type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    /// if let Some(addr) = obj.get_address_of_property::<i32>(0) {
    ///     // Assuming the first property is an int
    ///     // let value = unsafe { *addr };
    ///     // println!("Property value: {}", value);
    /// }
    /// ```
    pub fn get_address_of_property<T: ScriptData>(&self, prop: asUINT) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptObject_GetAddressOfProperty)(self.inner, prop);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    /// Gets the engine that created this object.
    ///
    /// # Returns
    /// The engine instance or an error if the engine is not available
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    /// let engine = obj.get_engine()?;
    /// println!("Engine version: {}", Engine::get_library_version());
    /// ```
    pub fn get_engine(&self) -> ScriptResult<Engine> {
        unsafe {
            let result: *mut asIScriptEngine =
                (self.as_vtable().asIScriptObject_GetEngine)(self.inner);
            let ptr = NonNull::new(result).ok_or(ScriptError::NullPointer)?;
            Ok(Engine::from_raw(ptr))
        }
    }

    /// Copies all properties from another object to this object.
    ///
    /// Both objects must be of the same type for the copy to succeed.
    /// This performs a deep copy of all property values.
    ///
    /// # Arguments
    /// * `other` - The object to copy from
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj1 = /* ... get first script object ... */;
    /// let obj2 = /* ... get second script object of same type ... */;
    ///
    /// // Copy all properties from obj1 to obj2
    /// obj2.copy_from(&obj1)?;
    /// ```
    pub fn copy_from(&self, other: &ScriptObject) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptObject_CopyFrom)(
                self.inner,
                other.inner,
            ))
        }
    }

    /// Sets user data on this object.
    ///
    /// User data allows applications to associate custom data with script objects.
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
    /// let obj = /* ... get script object ... */;
    /// let mut my_data = MyUserData::new();
    ///
    /// if let Some(old_data) = obj.set_user_data(&mut my_data) {
    ///     println!("Replaced existing user data");
    /// }
    /// ```
    pub fn set_user_data<T: UserData + ScriptData>(&self, data: &mut T) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptObject_SetUserData)(
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

    /// Gets user data from this object.
    ///
    /// # Returns
    /// The user data, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    ///
    /// if let Some(data) = obj.get_user_data::<MyUserData>() {
    ///     println!("Found user data: {:?}", data);
    /// }
    /// ```
    pub fn get_user_data<T: UserData + ScriptData>(&self) -> Option<T> {
        unsafe {
            let ptr =
                (self.as_vtable().asIScriptObject_GetUserData)(self.inner, T::KEY as asPWORD);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    /// Gets the vtable for the underlying AngelScript script object.
    ///
    /// # Safety
    /// This function provides direct access to the AngelScript vtable.
    fn as_vtable(&self) -> &asIScriptObject__bindgen_vtable {
        unsafe { &*(*self.inner).vtable_ }
    }
}

impl Drop for ScriptObject {
    fn drop(&mut self) {
        self.release().expect("Failed to release script object");
    }
}

unsafe impl Send for ScriptObject {}
unsafe impl Sync for ScriptObject {}

// ========== CONVENIENCE METHODS ==========

impl ScriptObject {
    /// Gets a property value by index with type safety.
    ///
    /// This is a convenience method that wraps `get_address_of_property()`.
    ///
    /// # Arguments
    /// * `prop` - The property index (0-based)
    ///
    /// # Returns
    /// The property value, or None if the index is invalid or type mismatch
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    ///
    /// // Get an integer property
    /// if let Some(score) = obj.get_property::<i32>(1) {
    ///     println!("Score: {}", score);
    /// }
    ///
    /// // Get a string property
    /// if let Some(name) = obj.get_property::<String>(0) {
    ///     println!("Name: {}", name);
    /// }
    /// ```
    pub fn get_property<T: ScriptData>(&self, prop: asUINT) -> Option<T> {
        self.get_address_of_property::<T>(prop)
    }

    /// Sets a property value by index with type safety.
    ///
    /// # Arguments
    /// * `prop` - The property index (0-based)
    /// * `value` - The value to set
    ///
    /// # Returns
    /// true if the property was set successfully, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    ///
    /// // Set an integer property
    /// if obj.set_property(1, 100i32) {
    ///     println!("Score set to 100");
    /// }
    ///
    /// // Set a float property
    /// if obj.set_property(2, 95.5f32) {
    ///     println!("Health set to 95.5");
    /// }
    /// ```
    pub fn set_property<T: Sized>(&self, prop: asUINT, value: T) -> bool {
        unsafe {
            let ptr =
                (self.as_vtable().asIScriptObject_GetAddressOfProperty)(self.inner, prop) as *mut T;
            if ptr.is_null() {
                false
            } else {
                ptr.write(value);
                true
            }
        }
    }

    /// Gets a property value by name.
    ///
    /// This is a convenience method that finds the property by name and then gets its value.
    ///
    /// # Arguments
    /// * `name` - The property name
    ///
    /// # Returns
    /// The property value, or None if not found or type mismatch
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    ///
    /// // Get property by name
    /// if let Some(name) = obj.get_property_by_name::<String>("name") {
    ///     println!("Player name: {}", name);
    /// }
    ///
    /// if let Some(score) = obj.get_property_by_name::<i32>("score") {
    ///     println!("Player score: {}", score);
    /// }
    /// ```
    pub fn get_property_by_name<T: ScriptData>(&self, name: &str) -> Option<T> {
        let prop_index = self.find_property_by_name(name)?;
        self.get_property::<T>(prop_index)
    }

    /// Sets a property value by name.
    ///
    /// This is a convenience method that finds the property by name and then sets its value.
    ///
    /// # Arguments
    /// * `name` - The property name
    /// * `value` - The value to set
    ///
    /// # Returns
    /// true if the property was set successfully, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    ///
    /// // Set properties by name
    /// if obj.set_property_by_name("score", 150i32) {
    ///     println!("Score updated");
    /// }
    ///
    /// if obj.set_property_by_name("health", 80.0f32) {
    ///     println!("Health updated");
    /// }
    /// ```
    pub fn set_property_by_name<T: ScriptData + Copy>(&self, name: &str, value: T) -> bool {
        if let Some(prop_index) = self.find_property_by_name(name) {
            self.set_property(prop_index, value)
        } else {
            false
        }
    }

    /// Finds a property index by name.
    ///
    /// # Arguments
    /// * `name` - The property name to search for
    ///
    /// # Returns
    /// The property index, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    ///
    /// if let Some(index) = obj.find_property_by_name("score") {
    ///     println!("Score property is at index {}", index);
    ///     let type_id = obj.get_property_type_id(index);
    ///     println!("Score property type ID: {}", type_id);
    /// }
    /// ```
    pub fn find_property_by_name(&self, name: &str) -> Option<asUINT> {
        let count = self.get_property_count();
        for i in 0..count {
            if let Some(prop_name) = self.get_property_name(i) {
                if prop_name == name {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Gets all properties as a vector of PropertyInfo.
    ///
    /// This is a convenience method that collects information about all properties
    /// into a vector for easier processing.
    ///
    /// # Returns
    /// A vector containing information about all properties
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    ///
    /// let properties = obj.get_all_properties();
    /// println!("Object has {} properties:", properties.len());
    ///
    /// for prop in properties {
    ///     println!("Property {}: name={:?}, type_id={}",
    ///              prop.index, prop.name, prop.type_id);
    /// }
    /// ```
    pub fn get_all_properties(&self) -> Vec<PropertyInfo> {
        let count = self.get_property_count();
        (0..count)
            .map(|i| PropertyInfo {
                index: i,
                name: self.get_property_name(i).map(|s| s.to_string()),
                type_id: self.get_property_type_id(i),
                address: self
                    .get_address_of_property::<ScriptMemoryLocation>(i)
                    .unwrap_or(ScriptMemoryLocation::null()),
            })
            .collect()
    }

    /// Checks if this object is of a specific type.
    ///
    /// # Arguments
    /// * `type_name` - The type name to check against
    ///
    /// # Returns
    /// true if the object is of the specified type, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    ///
    /// if obj.is_type("Player") {
    ///     println!("This is a Player object");
    /// } else if obj.is_type("Enemy") {
    ///     println!("This is an Enemy object");
    /// } else {
    ///     println!("Unknown object type");
    /// }
    /// ```
    pub fn is_type(&self, type_name: &str) -> bool {
        let type_info = self.get_object_type();
        if let Some(name) = type_info.get_name() {
            name == type_name
        } else {
            false
        }
    }

    /// Creates a weak reference to this object.
    ///
    /// Weak references allow checking if an object is still alive without
    /// keeping it alive. This is useful for avoiding circular references.
    ///
    /// # Returns
    /// A weak reference to this object, or None if weak references are not supported
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = /* ... get script object ... */;
    ///
    /// if let Some(weak_ref) = obj.create_weak_ref() {
    ///     // Store the weak reference somewhere
    ///     // Later, check if the object is still alive
    ///     if weak_ref.is_alive() {
    ///         if let Some(strong_ref) = weak_ref.upgrade() {
    ///             // Use the strong reference
    ///             println!("Object is still alive");
    ///         }
    ///     } else {
    ///         println!("Object has been destroyed");
    ///     }
    /// }
    /// ```
    pub fn create_weak_ref(&self) -> Option<WeakScriptObjectRef> {
        let weak_flag = self.get_weak_ref_flag()?;
        Some(WeakScriptObjectRef {
            object_ptr: self.clone(),
            weak_flag,
        })
    }
}

// ========== ADDITIONAL TYPES ==========

/// Information about a script object property.
///
/// This structure contains metadata about a property, including its index,
/// name, type, and memory address.
#[derive(Debug, Clone)]
pub struct PropertyInfo {
    /// The property's index within the object
    pub index: asUINT,
    /// The property's name (if available)
    pub name: Option<String>,
    /// The property's type ID
    pub type_id: i32,
    /// The property's memory address
    pub address: ScriptMemoryLocation,
}

/// A weak reference to a script object.
///
/// Weak references allow checking if an object is still alive without keeping
/// it alive. This is useful for implementing observer patterns, caches, or
/// other scenarios where you want to reference an object without affecting
/// its lifetime.
///
/// # Examples
///
/// ```rust
/// // Create a weak reference
/// let obj = /* ... get script object ... */;
/// let weak_ref = obj.create_weak_ref().expect("Failed to create weak reference");
///
/// // Later, check if the object is still alive
/// if weak_ref.is_alive() {
///     println!("Object is still alive");
///
///     // Try to get a strong reference
///     if let Some(strong_ref) = weak_ref.upgrade() {
///         // Use the strong reference safely
///         println!("Got strong reference, object type: {}",
///                  strong_ref.get_object_type().get_name().unwrap_or("unknown"));
///     }
/// } else {
///     println!("Object has been destroyed");
/// }
/// ```
#[derive(Debug)]
pub struct WeakScriptObjectRef {
    object_ptr: ScriptObject,
    weak_flag: LockableSharedBool,
}

impl WeakScriptObjectRef {
    /// Checks if the referenced object is still alive.
    ///
    /// # Returns
    /// true if the object is still alive, false if it has been destroyed
    ///
    /// # Examples
    ///
    /// ```rust
    /// let weak_ref = /* ... get weak reference ... */;
    ///
    /// if weak_ref.is_alive() {
    ///     println!("Object is still alive");
    /// } else {
    ///     println!("Object has been destroyed");
    /// }
    /// ```
    pub fn is_alive(&self) -> bool {
        !self.weak_flag.get()
    }

    /// Attempts to get a strong reference to the object.
    ///
    /// This method tries to convert the weak reference back to a strong reference.
    /// It may fail if the object is in the process of being destroyed or has
    /// already been destroyed.
    ///
    /// # Returns
    /// A strong reference to the object, or None if the object is no longer available
    ///
    /// # Examples
    ///
    /// ```rust
    /// let weak_ref = /* ... get weak reference ... */;
    ///
    /// match weak_ref.upgrade() {
    ///     Some(strong_ref) => {
    ///         // Use the strong reference safely
    ///         println!("Object properties: {}", strong_ref.get_property_count());
    ///     }
    ///     None => {
    ///         println!("Object is no longer available");
    ///     }
    /// }
    /// ```
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called from multiple threads
    /// concurrently. However, the returned strong reference should be used
    /// according to the object's thread safety guarantees.
    pub fn upgrade(&self) -> Option<ScriptObject> {
        if self.is_alive() {
            // Try to add a reference - this might fail if the object
            // is in the process of being destroyed
            unsafe {
                let vtable = self.object_ptr.as_vtable();
                if (vtable.asIScriptObject_AddRef)(self.object_ptr.inner) >= 0 {
                    Some(ScriptObject::from_raw(self.object_ptr.inner))
                } else {
                    None
                }
            }
        } else {
            None
        }
    }
}

unsafe impl Send for WeakScriptObjectRef {}
unsafe impl Sync for WeakScriptObjectRef {}

impl ScriptData for ScriptObject {
    fn to_script_ptr(&mut self) -> *mut Void {
        self.inner as *mut Void   
    }

    fn from_script_ptr(ptr: *mut Void) -> Self
    where
        Self: Sized
    {
        Self::from_raw(ptr as *mut asIScriptObject)   
    }
}