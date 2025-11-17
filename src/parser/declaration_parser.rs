use crate::core::type_registry::{ParameterFlags, ParameterInfo, TypeRegistry};
use crate::core::types::*;
use crate::parser::ast::{DataType, Param, Type, TypeMod, TypeModifier};
use crate::{Lexer, Parser};
use std::sync::{Arc, RwLock};

pub struct DeclarationParser {
    registry: Arc<RwLock<TypeRegistry>>,
}

impl DeclarationParser {
    pub fn new(registry: Arc<RwLock<TypeRegistry>>) -> Self {
        Self { registry }
    }

    pub fn parse_function_declaration(
        &self,
        declaration: &str,
    ) -> Result<ParsedFunction, String> {
        let lexer = Lexer::new(declaration);
        let tokens = lexer
            .tokenize()
            .map_err(|e| format!("Failed to tokenize declaration '{}': {:?}", declaration, e))?;

        let mut parser = Parser::new(tokens);
        let func = parser.parse_function_signature().map_err(|e| {
            format!(
                "Failed to parse function declaration '{}': {:?}",
                declaration, e
            )
        })?;

        Ok(ParsedFunction {
            name: func.name,
            return_type_id: self.resolve_type(&func.return_type)?,
            parameters: self.resolve_params(&func.params)?,
            is_const: func.is_const,
            is_ref: func.is_ref,
        })
    }

    pub fn parse_property_declaration(
        &self,
        declaration: &str,
    ) -> Result<ParsedProperty, String> {
        let decl_with_semi = format!("{};", declaration.trim_end_matches(';'));

        let lexer = Lexer::new(&decl_with_semi);
        let tokens = lexer
            .tokenize()
            .map_err(|e| format!("Failed to tokenize property '{}': {:?}", declaration, e))?;

        let mut parser = Parser::new(tokens);
        let var = parser.parse_var(false, false).map_err(|e| {
            format!(
                "Failed to parse property declaration '{}': {:?}",
                declaration, e
            )
        })?;

        if var.declarations.is_empty() {
            return Err("No property name found in declaration".to_string());
        }

        let decl = &var.declarations[0];

        Ok(ParsedProperty {
            name: decl.name.clone(),
            type_id: self.resolve_type_def(&var.var_type)?,
            is_const: var.var_type.is_const,
            is_handle: var
                .var_type
                .modifiers
                .iter()
                .any(|m| matches!(m, TypeModifier::Handle)),
        })
    }

    pub fn parse_behaviour_declaration(
        &self,
        declaration: &str,
    ) -> Result<ParsedFunction, String> {
        self.parse_function_declaration(declaration)
    }

    fn resolve_type(&self, type_ast: &Option<Type>) -> Result<u32, String> {
        match type_ast {
            Some(t) => self.resolve_type_def(t),
            None => Ok(TYPE_VOID),
        }
    }

    fn resolve_type_def(&self, type_def: &Type) -> Result<u32, String> {
        let type_name = match &type_def.datatype {
            DataType::PrimType(name) => name.clone(),
            DataType::Identifier(name) => name.clone(),
            DataType::Auto => return Ok(TYPE_AUTO),
            DataType::Question => return Ok(TYPE_VOID),
        };

        if let Some(prim_type_id) = self.get_primitive_type_id(&type_name) {
            return Ok(prim_type_id);
        }

        let registry = self.registry.read().unwrap();
        registry
            .lookup_type(&type_name, &[])
            .ok_or_else(|| format!("Unknown type: {}", type_name))
    }

    fn get_primitive_type_id(&self, type_name: &str) -> Option<u32> {
        match type_name {
            "void" => Some(TYPE_VOID),
            "bool" => Some(TYPE_BOOL),
            "int8" => Some(TYPE_INT8),
            "int16" => Some(TYPE_INT16),
            "int" | "int32" => Some(TYPE_INT32),
            "int64" => Some(TYPE_INT64),
            "uint8" => Some(TYPE_UINT8),
            "uint16" => Some(TYPE_UINT16),
            "uint" | "uint32" => Some(TYPE_UINT32),
            "uint64" => Some(TYPE_UINT64),
            "float" => Some(TYPE_FLOAT),
            "double" => Some(TYPE_DOUBLE),
            "string" => Some(TYPE_STRING),
            "auto" => Some(TYPE_AUTO),
            _ => None,
        }
    }

    fn resolve_params(&self, params: &[Param]) -> Result<Vec<ParameterInfo>, String> {
        params
            .iter()
            .map(|p| {
                Ok(ParameterInfo {
                    name: p.name.clone(),
                    type_id: self.resolve_type_def(&p.param_type)?,
                    flags: if p.param_type.is_const {
                        ParameterFlags::IN | ParameterFlags::CONST
                    } else {
                        match p.type_mod {
                            Some(TypeMod::Out) => ParameterFlags::OUT,
                            Some(TypeMod::InOut) => ParameterFlags::INOUT,
                            Some(TypeMod::In) | None => ParameterFlags::IN,
                        }
                    },
                    default_expr: p.default_value.as_ref().map(|expr| Arc::new(expr.clone())),
                    definition_span: None,
                })
            })
            .collect()
    }
}

pub struct ParsedFunction {
    pub name: String,
    pub return_type_id: u32,
    pub parameters: Vec<ParameterInfo>,
    pub is_const: bool,
    pub is_ref: bool,
}

pub struct ParsedProperty {
    pub name: String,
    pub type_id: u32,
    pub is_const: bool,
    pub is_handle: bool,
}
