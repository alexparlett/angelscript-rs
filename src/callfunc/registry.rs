//! System Function Registry
//!
//! This module provides the registry that stores all native/system functions.
//! Each function is stored with its SystemFunctionInterface which describes
//! how to call it.

use super::interface::SystemFunctionInterface;
use crate::core::types::FunctionId;
use std::collections::HashMap;

/// Registry of all system/native functions
/// 
/// This is the central storage for native function implementations.
/// When a script calls a native function, the VM looks up the function
/// in this registry to get the SystemFunctionInterface which contains
/// the actual callable.
/// 
/// This is equivalent to how AngelScript stores `sysFuncIntf` per function
/// in the C++ implementation.
#[derive(Default)]
pub struct SystemFunctionRegistry {
    /// FunctionId -> SystemFunctionInterface mapping
    functions: HashMap<FunctionId, SystemFunctionInterface>,
}

impl SystemFunctionRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    /// Register a system function
    /// 
    /// Associates the function ID with its interface (which contains the callable).
    /// If a function with this ID already exists, it will be replaced.
    pub fn register(&mut self, func_id: FunctionId, interface: SystemFunctionInterface) {
        self.functions.insert(func_id, interface);
    }

    /// Try to register a system function
    /// 
    /// Returns an error if a function with this ID already exists.
    pub fn try_register(
        &mut self,
        func_id: FunctionId,
        interface: SystemFunctionInterface,
    ) -> Result<(), String> {
        if self.functions.contains_key(&func_id) {
            return Err(format!(
                "System function {} already registered", 
                func_id
            ));
        }
        self.functions.insert(func_id, interface);
        Ok(())
    }

    /// Get a system function interface by ID
    pub fn get(&self, func_id: FunctionId) -> Option<&SystemFunctionInterface> {
        self.functions.get(&func_id)
    }

    /// Check if a function is registered
    pub fn contains(&self, func_id: FunctionId) -> bool {
        self.functions.contains_key(&func_id)
    }

    /// Remove a function from the registry
    pub fn remove(&mut self, func_id: FunctionId) -> Option<SystemFunctionInterface> {
        self.functions.remove(&func_id)
    }

    /// Get the number of registered functions
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }

    /// Get all function IDs
    pub fn function_ids(&self) -> impl Iterator<Item = FunctionId> + '_ {
        self.functions.keys().copied()
    }

    /// Clear all registered functions
    pub fn clear(&mut self) {
        self.functions.clear();
    }

    /// Get iterator over all functions
    pub fn iter(&self) -> impl Iterator<Item = (FunctionId, &SystemFunctionInterface)> {
        self.functions.iter().map(|(&id, iface)| (id, iface))
    }
}

impl std::fmt::Debug for SystemFunctionRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemFunctionRegistry")
            .field("count", &self.functions.len())
            .field("function_ids", &self.functions.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::interface::{CallConv, NativeCallable, ReturnType};
    use std::sync::Arc;

    fn create_dummy_interface(name: &str) -> SystemFunctionInterface {
        SystemFunctionInterface::new(
            NativeCallable::Generic(Arc::new(|_ctx| Ok(()))),
            CallConv::Generic,
            None,
            vec![],
            ReturnType::Void,
            name,
        )
    }

    #[test]
    fn test_register_and_get() {
        let mut registry = SystemFunctionRegistry::new();
        
        registry.register(1000, create_dummy_interface("test_func"));
        
        assert!(registry.contains(1000));
        assert!(!registry.contains(1001));
        
        let iface = registry.get(1000).unwrap();
        assert_eq!(iface.name, "test_func");
    }

    #[test]
    fn test_try_register_duplicate() {
        let mut registry = SystemFunctionRegistry::new();
        
        registry.register(1000, create_dummy_interface("first"));
        
        let result = registry.try_register(1000, create_dummy_interface("second"));
        assert!(result.is_err());
        
        // Original should still be there
        assert_eq!(registry.get(1000).unwrap().name, "first");
    }

    #[test]
    fn test_remove() {
        let mut registry = SystemFunctionRegistry::new();
        registry.register(1000, create_dummy_interface("test"));
        
        assert!(registry.contains(1000));
        
        let removed = registry.remove(1000);
        assert!(removed.is_some());
        assert!(!registry.contains(1000));
    }

    #[test]
    fn test_len_and_empty() {
        let mut registry = SystemFunctionRegistry::new();
        
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        
        registry.register(1000, create_dummy_interface("a"));
        registry.register(1001, create_dummy_interface("b"));
        
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_clear() {
        let mut registry = SystemFunctionRegistry::new();
        registry.register(1000, create_dummy_interface("a"));
        registry.register(1001, create_dummy_interface("b"));
        
        registry.clear();
        
        assert!(registry.is_empty());
    }
}