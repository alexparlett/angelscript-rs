use angelscript_sys::{asIStringFactory, asIStringFactory__bindgen_vtable, asUINT};
use std::collections::HashMap;
use std::ffi::{c_char, c_void};
use std::sync::{Arc, Mutex, OnceLock};

pub struct InnerCache {
    strings: HashMap<Arc<String>, usize>, // Maps Arc<String> to ref count
}

impl InnerCache {
    pub fn new() -> Self {
        println!("[InnerCache] Initialized empty InnerCache.");
        Self {
            strings: HashMap::new(),
        }
    }

    /// Intern a string and return a stable pointer.
    pub fn intern(&mut self, string: &str) -> *const String {
        println!(
            "[InnerCache::intern] Attempting to intern string: '{}'",
            string
        );

        let arc_string = Arc::new(string.to_string());
        if let Some(count) = self.strings.get_mut(&arc_string) {
            *count += 1;
            println!(
                "[InnerCache::intern] String already exists in cache. Updated ref count: {}",
                count
            );
            // Ensure the pointer is derived from the current key in the cache,
            // as `arc_string` is a new Arc unrelated with previous instances in memory.
            for key in self.strings.keys() {
                if key.as_str() == string {
                    return Arc::as_ptr(key) as *const String;
                }
            }
        }

        // Add the new string to the cache.
        self.strings.insert(arc_string.clone(), 1);
        println!(
            "[InnerCache::intern] Added new string to cache. Cache size: {}",
            self.strings.len()
        );
        Arc::as_ptr(&arc_string)
    }

    /// Release a string pointer and decrement its reference count.
    /// Frees the memory if the reference count drops to zero.
    pub fn release(&mut self, raw_ptr: *const String) -> bool {
        println!(
            "[InnerCache::release] Attempting to release string pointer: {:?}",
            raw_ptr
        );

        // Extract the key-value pair where the raw pointer matches
        let found_entry = self
            .strings
            .iter()
            .find(|(key, _)| Arc::as_ptr(key) == raw_ptr)
            .map(|(key, count)| (key.clone(), *count)); // Clone key to avoid holding a borrow

        if let Some((key, count)) = found_entry {
            if count > 1 {
                // Decrement reference count
                if let Some(current_count) = self.strings.get_mut(&key) {
                    *current_count -= 1;
                    println!(
                        "[InnerCache::release] Decremented ref count for string. New count: {}",
                        current_count
                    );
                }
            } else {
                // Remove the string if reference count is 0
                self.strings.remove(&key);
                println!("[InnerCache::release] Ref count reached 0. Removed string from cache.");
            }
            true
        } else {
            println!("[InnerCache::release] Failed to release string. Pointer not found in cache.");
            true
        }
    }
}

#[derive(Clone)]
pub struct StringFactory {
    pub(crate) cache: Arc<Mutex<InnerCache>>,
}

impl StringFactory {
    pub(crate) fn new() -> Self {
        println!("[StringFactory::new] Creating a new StringFactory instance.");
        Self {
            cache: Arc::new(Mutex::new(InnerCache::new())),
        }
    }

    pub(crate) fn reset(&self) {
        let mut cache = self.cache.lock().unwrap();
        println!(
            "[StringFactory::reset] Resetting string cache. Current cache size: {}",
            cache.strings.len()
        );
        cache.strings.clear();
    }

    pub fn singleton() -> &'static StringFactory {
        static INSTANCE: OnceLock<StringFactory> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            println!("[StringFactory] Creating a singleton instance.");
            StringFactory::new()
        })
    }

    pub fn cache(&self) -> Arc<Mutex<InnerCache>> {
        println!("[StringFactory::cache] Returning reference to the cache.");
        self.cache.clone()
    }

    pub unsafe extern "C" fn get_string_constant(
        _this: *mut asIStringFactory,
        data: *const c_char,
        length: u32,
    ) -> *const c_void {
        let factory = StringFactory::singleton();
        let mut cache = factory.cache.lock().unwrap();

        println!(
            "[StringFactory::get_string_constant] Called with pointer: {:?}, length: {}",
            data, length
        );

        // Convert raw data to a Rust string
        let slice = std::slice::from_raw_parts(data as *const u8, length as usize);
        let string = match std::str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("[StringFactory::get_string_constant] Invalid UTF-8 string.");
                return std::ptr::null();
            }
        };
        println!(
            "[StringFactory::get_string_constant] Attempting to intern string: '{}'",
            string
        );

        // Intern the string and return a stable pointer
        let pointer = cache.intern(string) as *const c_void;
        println!(
            "[StringFactory::get_string_constant] Interned string. Pointer: {:?}",
            pointer
        );

        pointer
    }

    /// Release a string constant
    pub unsafe extern "C" fn release_string_constant(
        _this: *mut asIStringFactory,
        str_: *const c_void,
    ) -> i32 {
        let factory = StringFactory::singleton();
        let mut cache = factory.cache.lock().unwrap();

        println!(
            "[StringFactory::release_string_constant] Called with pointer: {:?}",
            str_
        );

        let str_ptr = str_ as *const String;

        if cache.release(str_ptr) {
            println!("[StringFactory::release_string_constant] Successfully released string.");
            0 // Success
        } else {
            eprintln!(
                "[StringFactory::release_string_constant] Failed to release string. Pointer not found."
            );
            -1 // Failure (unknown pointer)
        }
    }

    /// Get raw string data
    pub unsafe extern "C" fn get_raw_string_data(
        _this: *const asIStringFactory,
        str_: *const c_void,
        data: *mut c_char,
        length: *mut asUINT,
    ) -> i32 {
        println!(
            "[StringFactory::get_raw_string_data] Called with str_: {:?}, data: {:?}, length: {:?}",
            str_, data, length
        );

        // Cast the input pointer to a String reference
        if str_.is_null() {
            eprintln!("[StringFactory::get_raw_string_data] Null pointer error.");
            return -1; // Null pointer error
        }

        let str_ptr = str_ as *const String;

        // Use raw pointer dereference to avoid taking ownership
        let string = &*str_ptr;

        // Write the length of the string
        if !length.is_null() {
            length.write(string.len() as asUINT);
            println!(
                "[StringFactory::get_raw_string_data] Wrote string length: {}",
                string.len()
            );
        }

        // Write the string data
        if !data.is_null() {
            std::ptr::copy_nonoverlapping(string.as_ptr() as *const c_char, data, string.len());
            println!(
                "[StringFactory::get_raw_string_data] Wrote string data: '{}'",
                string
            );
        }

        0 // Success
    }
}

unsafe impl Send for StringFactory {}
unsafe impl Sync for StringFactory {}

// Keep the VTable instance alive
static STRING_FACTORY_VTABLE: asIStringFactory__bindgen_vtable = asIStringFactory__bindgen_vtable {
    asIStringFactory_GetStringConstant: StringFactory::get_string_constant,
    asIStringFactory_ReleaseStringConstant: StringFactory::release_string_constant,
    asIStringFactory_GetRawStringData: StringFactory::get_raw_string_data,
};

pub fn get_string_factory_instance() -> &'static asIStringFactory {
    static INSTANCE: OnceLock<asIStringFactory> = OnceLock::new();
    INSTANCE.get_or_init(|| asIStringFactory {
        vtable_: &STRING_FACTORY_VTABLE,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_get_string_constant() {
        unsafe {
            let test_string = CString::new("test").unwrap();

            let factory = StringFactory::singleton();
            factory.reset(); // Reset the singleton state before running this test

            let result = StringFactory::get_string_constant(
                std::ptr::null_mut(), // Simulating '_this' (unused in this case)
                test_string.as_ptr(),
                test_string.as_bytes().len() as u32,
            );

            assert!(!result.is_null()); // Ensure we get a valid pointer back
            let cache = factory.cache.lock().unwrap();

            // Verify that the string exists in the cache
            assert!(cache.strings.iter().any(|(s, _)| s.as_str() == "test"));
        }
    }

    #[test]
    fn test_release_string_constant() {
        unsafe {
            let test_string = CString::new("test").unwrap();

            let factory = StringFactory::singleton();
            factory.reset(); // Reset the singleton state before running this test

            // Add a string to the cache
            let str_ptr = StringFactory::get_string_constant(
                std::ptr::null_mut(),
                test_string.as_ptr(),
                test_string.as_bytes().len() as u32,
            );

            assert!(!str_ptr.is_null()); // Ensure the pointer was added

            // Release the string and verify it gets removed
            let result = StringFactory::release_string_constant(std::ptr::null_mut(), str_ptr);
            assert_eq!(result, 0); // Success

            // Verify the string is removed from the cache
            let cache = factory.cache.lock().unwrap();
            assert!(!cache.strings.iter().any(|(s, _)| s.as_str() == "test"));
        }
    }

    #[test]
    fn test_get_raw_string_data() {
        unsafe {
            let test_string = CString::new("test").unwrap();

            let factory = StringFactory::singleton();
            factory.reset(); // Reset the singleton state before running this test

            // Add a string to the cache
            let str_ptr = StringFactory::get_string_constant(
                std::ptr::null_mut(),
                test_string.as_ptr(),
                test_string.as_bytes().len() as u32,
            );

            assert!(!str_ptr.is_null());

            // Create buffers to hold data from get_raw_string_data
            let mut data_buffer = vec![0; 4]; // Buffer for raw string data
            let mut len = 0u32;

            // Call get_raw_string_data
            let result = StringFactory::get_raw_string_data(
                std::ptr::null(),
                str_ptr,
                data_buffer.as_mut_ptr() as *mut c_char,
                &mut len as *mut u32,
            );

            assert_eq!(result, 0); // Success
            assert_eq!(len, 4); // Length of "test"
            assert_eq!(std::str::from_utf8(&data_buffer).unwrap(), "test"); // Ensure the raw data matches
        }
    }

    #[test]
    fn test_inner_cache_interning() {
        let mut cache = InnerCache::new();

        // Intern a test string
        let str1 = "hello";
        let ptr = cache.intern(str1);

        // Ensure it's interned
        let arc_str1 = cache
            .strings
            .keys()
            .find(|s| s.as_str() == str1)
            .cloned()
            .expect("String not found in map");
        assert_eq!(arc_str1.as_str(), str1);

        // Intern the same string again and ensure the reference count increases
        let _ = cache.intern(str1);
        let count: usize = *cache.strings.get(&arc_str1).unwrap();
        assert_eq!(count, 2);

        // Release the string and verify the count decreases
        cache.release(ptr);

        // Re-acquire `arc_str1` from the map to avoid immutable/mutable borrow conflict
        let arc_str1 = cache
            .strings
            .keys()
            .find(|s| s.as_str() == str1)
            .cloned()
            .expect("String not found in map after release");
        let count: usize = *cache.strings.get(&arc_str1).unwrap();
        assert_eq!(count, 1);

        // Release again and ensure it's removed from the cache
        cache.release(ptr);
        assert!(!cache.strings.contains_key(&arc_str1));
    }

    #[test]
    fn test_unique_and_reused_pointers() {
        unsafe {
            let factory = StringFactory::singleton();
            factory.reset(); // Clear the cache before testing

            // Create CString instances for testing
            let string_hello = CString::new("Hello").unwrap();
            let string_world = CString::new("World").unwrap();
            let string_hello_dup = CString::new("Hello").unwrap(); // Duplicate of "Hello"

            // Call `get_string_constant` for "Hello" and "World"
            let ptr_hello = StringFactory::get_string_constant(
                std::ptr::null_mut(),
                string_hello.as_ptr(),
                string_hello.as_bytes().len() as u32,
            );
            let ptr_world = StringFactory::get_string_constant(
                std::ptr::null_mut(),
                string_world.as_ptr(),
                string_world.as_bytes().len() as u32,
            );
            let ptr_hello_dup = StringFactory::get_string_constant(
                std::ptr::null_mut(),
                string_hello_dup.as_ptr(),
                string_hello_dup.as_bytes().len() as u32,
            );

            // Verify that the pointers for "Hello" and "World" are distinct
            assert!(!ptr_hello.is_null());
            assert!(!ptr_world.is_null());
            assert_ne!(
                ptr_hello, ptr_world,
                "Pointers for 'Hello' and 'World' should be distinct"
            );

            // Verify that the pointer for the duplicate "Hello" is identical to the original "Hello"
            assert_eq!(
                ptr_hello, ptr_hello_dup,
                "Duplicate 'Hello' should return the same pointer as the original"
            );

            // Log the pointer values for debugging if needed
            println!(
                "Pointer for 'Hello': {:?}, Pointer for 'World': {:?}, Pointer for duplicate 'Hello': {:?}",
                ptr_hello, ptr_world, ptr_hello_dup
            );
        }
    }
}
