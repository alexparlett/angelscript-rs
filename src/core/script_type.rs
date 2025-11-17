use crate::core::type_registry::{PropertyFlags, TypeRegistry};
use crate::core::types::{BehaviourType, TypeId, TypeKind};
use std::sync::{Arc, RwLock};

pub struct ScriptType {
    type_id: TypeId,
    registry: Arc<RwLock<TypeRegistry>>,
}

impl ScriptType {
    pub(crate) fn new(type_id: TypeId, registry: Arc<RwLock<TypeRegistry>>) -> Self {
        Self { type_id, registry }
    }

    pub fn add_ref(&self) -> i32 {
        1
    }

    pub fn release(&self) -> i32 {
        0
    }

    pub fn get_name(&self) -> String {
        let registry = self.registry.read().unwrap();
        registry
            .get_type(self.type_id)
            .map(|t| t.name.clone())
            .unwrap_or_default()
    }

    pub fn get_namespace(&self) -> String {
        let registry = self.registry.read().unwrap();
        registry
            .get_type(self.type_id)
            .map(|t| t.namespace.join("::"))
            .unwrap_or_default()
    }

    pub fn get_base_type(&self) -> Option<ScriptType> {
        let registry = self.registry.read().unwrap();
        registry
            .get_type(self.type_id)
            .and_then(|t| t.base_type)
            .map(|base_id| ScriptType::new(base_id, Arc::clone(&self.registry)))
    }

    pub fn derives_from(&self, other: &ScriptType) -> bool {
        let mut current_type_id = Some(self.type_id);
        let registry = self.registry.read().unwrap();

        while let Some(type_id) = current_type_id {
            if type_id == other.type_id {
                return true;
            }

            current_type_id = registry.get_type(type_id).and_then(|t| t.base_type);
        }

        false
    }

    pub fn get_flags(&self) -> u32 {
        let registry = self.registry.read().unwrap();
        registry
            .get_type(self.type_id)
            .map(|t| t.flags.bits())
            .unwrap_or(0)
    }

    pub fn get_type_id(&self) -> i32 {
        self.type_id as i32
    }

    pub fn get_sub_type_id(&self, _sub_type_index: u32) -> i32 {
        0
    }

    pub fn get_sub_type(&self, _sub_type_index: u32) -> Option<ScriptType> {
        None
    }

    pub fn get_sub_type_count(&self) -> u32 {
        0
    }

    pub fn get_interface_count(&self) -> u32 {
        let registry = self.registry.read().unwrap();
        registry
            .get_type(self.type_id)
            .map(|t| t.interfaces.len() as u32)
            .unwrap_or(0)
    }

    pub fn get_interface(&self, index: u32) -> Option<ScriptType> {
        let registry = self.registry.read().unwrap();
        registry
            .get_type(self.type_id)
            .and_then(|t| t.interfaces.get(index as usize).copied())
            .map(|interface_id| ScriptType::new(interface_id, Arc::clone(&self.registry)))
    }

    pub fn implements_interface(&self, other: &ScriptType) -> bool {
        let registry = self.registry.read().unwrap();
        registry
            .get_type(self.type_id)
            .map(|t| t.interfaces.contains(&other.type_id))
            .unwrap_or(false)
    }

    pub fn get_property_count(&self) -> u32 {
        let registry = self.registry.read().unwrap();
        registry
            .get_type(self.type_id)
            .map(|t| t.properties.len() as u32)
            .unwrap_or(0)
    }

    pub fn get_property(
        &self,
        index: u32,
        name: &mut Option<String>,
        type_id: &mut i32,
        is_private: &mut bool,
        is_protected: &mut bool,
        is_reference: &mut bool,
        is_const: &mut bool,
        config_group: &mut Option<String>,
        default_value: &mut Option<String>,
        access_mask: &mut u32,
    ) -> Result<i32, String> {
        let registry = self.registry.read().unwrap();
        let type_info = registry.get_type(self.type_id).ok_or("Type not found")?;

        let prop = type_info
            .properties
            .get(index as usize)
            .ok_or("Property index out of bounds")?;

        *name = Some(prop.name.clone());
        *type_id = prop.type_id as i32;
        *is_private = prop.access == crate::core::types::AccessSpecifier::Private;
        *is_protected = prop.access == crate::core::types::AccessSpecifier::Protected;
        *is_reference = false;
        *is_const = prop.flags.contains(PropertyFlags::CONST);
        *config_group = None;
        *default_value = None;
        *access_mask = 0xFFFFFFFF;

        Ok(0)
    }

    pub fn get_property_declaration(&self, index: u32, include_namespace: bool) -> Option<String> {
        let registry = self.registry.read().unwrap();
        let type_info = registry.get_type(self.type_id)?;
        let prop = type_info.properties.get(index as usize)?;

        let prop_type = registry.get_type(prop.type_id)?;
        let mut decl = String::new();

        if prop.flags.contains(PropertyFlags::CONST) {
            decl.push_str("const ");
        }

        if include_namespace && !prop_type.namespace.is_empty() {
            decl.push_str(&prop_type.namespace.join("::"));
            decl.push_str("::");
        }

        decl.push_str(&prop_type.name);
        decl.push(' ');
        decl.push_str(&prop.name);

        Some(decl)
    }

    pub fn get_method_count(&self) -> u32 {
        let registry = self.registry.read().unwrap();
        registry
            .get_type(self.type_id)
            .map(|t| t.methods.values().map(|v| v.len()).sum::<usize>() as u32)
            .unwrap_or(0)
    }

    pub fn get_method_by_index(
        &self,
        index: u32,
    ) -> Option<super::script_function::ScriptFunction> {
        let registry = self.registry.read().unwrap();
        let type_info = registry.get_type(self.type_id)?;

        let mut current_index = 0;
        for methods in type_info.methods.values() {
            for method_sig in methods {
                if current_index == index {
                    return Some(super::script_function::ScriptFunction::new(
                        method_sig.function_id,
                        Arc::clone(&self.registry),
                    ));
                }
                current_index += 1;
            }
        }
        None
    }

    pub fn get_method_by_name(
        &self,
        name: &str,
        _all_overloads: bool,
    ) -> Option<super::script_function::ScriptFunction> {
        let registry = self.registry.read().unwrap();
        let type_info = registry.get_type(self.type_id)?;

        type_info
            .get_method(name)
            .and_then(|methods| methods.first())
            .map(|method_sig| {
                super::script_function::ScriptFunction::new(
                    method_sig.function_id,
                    Arc::clone(&self.registry),
                )
            })
    }

    pub fn get_method_by_decl(
        &self,
        _decl: &str,
    ) -> Option<super::script_function::ScriptFunction> {
        None
    }

    pub fn get_behaviour_count(&self) -> u32 {
        let registry = self.registry.read().unwrap();
        registry
            .get_type(self.type_id)
            .map(|t| t.behaviours.len() as u32)
            .unwrap_or(0)
    }

    pub fn get_behaviour_by_index(
        &self,
        index: u32,
        out_behaviour: &mut BehaviourType,
    ) -> Option<super::script_function::ScriptFunction> {
        let registry = self.registry.read().unwrap();
        let type_info = registry.get_type(self.type_id)?;

        let (behaviour_type, func_id) = type_info.behaviours.iter().nth(index as usize)?;

        *out_behaviour = *behaviour_type;

        Some(super::script_function::ScriptFunction::new(
            *func_id,
            Arc::clone(&self.registry),
        ))
    }

    pub fn get_factory_count(&self) -> u32 {
        let registry = self.registry.read().unwrap();
        registry
            .get_type(self.type_id)
            .and_then(|t| {
                if t.behaviours.contains_key(&BehaviourType::Construct) {
                    Some(1)
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }

    pub fn get_factory_by_index(
        &self,
        index: u32,
    ) -> Option<super::script_function::ScriptFunction> {
        if index != 0 {
            return None;
        }

        let registry = self.registry.read().unwrap();
        let type_info = registry.get_type(self.type_id)?;

        let func_id = type_info.behaviours.get(&BehaviourType::Construct)?;

        Some(super::script_function::ScriptFunction::new(
            *func_id,
            Arc::clone(&self.registry),
        ))
    }

    pub fn get_factory_by_decl(
        &self,
        _decl: &str,
    ) -> Option<super::script_function::ScriptFunction> {
        None
    }

    pub fn get_child_funcdef_count(&self) -> u32 {
        0
    }

    pub fn get_child_funcdef(&self, _index: u32) -> Option<ScriptType> {
        None
    }

    pub fn get_parent_type(&self) -> Option<ScriptType> {
        None
    }

    pub fn get_enum_value_count(&self) -> u32 {
        let registry = self.registry.read().unwrap();
        if let Some(type_info) = registry.get_type(self.type_id) {
            if type_info.kind == TypeKind::Enum {
                return type_info.properties.len() as u32;
            }
        }

        0
    }

    pub fn get_enum_value_by_index(&self, index: u32, out_value: &mut i32) -> Option<String> {
        let registry = self.registry.read().unwrap();
        let type_info = registry.get_type(self.type_id)?;

        if type_info.kind != TypeKind::Enum {
            return None;
        }

        let prop = type_info.properties.get(index as usize)?;

        *out_value = index as i32;
        Some(prop.name.clone())
    }

    pub fn get_enum_value_by_name(&self, name: &str) -> Option<i32> {
        let registry = self.registry.read().unwrap();
        let type_info = registry.get_type(self.type_id)?;

        if type_info.kind != TypeKind::Enum {
            return None;
        }

        type_info
            .properties
            .iter()
            .position(|p| p.name == name)
            .map(|pos| pos as i32)
    }

    pub fn get_funcdef_signature(&self) -> Option<super::script_function::ScriptFunction> {
        let registry = self.registry.read().unwrap();
        let type_info = registry.get_type(self.type_id)?;

        if type_info.kind != TypeKind::Funcdef {
            return None;
        }

        None
    }
}
