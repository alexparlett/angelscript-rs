//! ScriptRegistry - stores script-defined types and functions.

use rustc_hash::FxHashMap;
use crate::types::{TypeHash, TypeDef, FunctionDef};

/// Registry for script-defined types and functions.
pub struct ScriptRegistry {
    types: FxHashMap<TypeHash, TypeDef>,
    type_by_name: FxHashMap<String, TypeHash>,
    functions: FxHashMap<TypeHash, FunctionDef>,
    func_by_name: FxHashMap<String, Vec<TypeHash>>,
}

impl ScriptRegistry {
    pub fn new() -> Self {
        Self {
            types: FxHashMap::default(),
            type_by_name: FxHashMap::default(),
            functions: FxHashMap::default(),
            func_by_name: FxHashMap::default(),
        }
    }
}

impl Default for ScriptRegistry {
    fn default() -> Self {
        Self::new()
    }
}
