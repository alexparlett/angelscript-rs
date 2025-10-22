// src/core/declaration_parser.rs - New file

use crate::compiler::semantic::TYPE_AUTO;
use crate::core::engine::{EngineInner, MethodParam};
use crate::parser::ast::{DataType, Param, Type, TypeMod, TypeModifier};
use crate::{Lexer, Parser};
use std::sync::{Arc, RwLock};

/// Helper to parse AngelScript declarations using our existing parser
pub struct DeclarationParser {
    engine: Arc<RwLock<EngineInner>>,
}

impl DeclarationParser {
    pub fn new(engine: Arc<RwLock<EngineInner>>) -> Self {
        Self { engine }
    }

    /// Parse a method/function declaration: "return_type name(params) const"
    pub fn parse_function_declaration(
        &self,
        declaration: &str,
    ) -> Result<FunctionSignature, String> {
        // Tokenize the declaration
        let lexer = Lexer::new(declaration);
        let tokens = lexer
            .tokenize()
            .map_err(|e| format!("Failed to tokenize declaration '{}': {:?}", declaration, e))?;

        // Parse as a function signature
        let mut parser = Parser::new(tokens);
        let func = parser.parse_function_signature().map_err(|e| {
            format!(
                "Failed to parse function declaration '{}': {:?}",
                declaration, e
            )
        })?;

        // Convert to our internal representation
        Ok(FunctionSignature {
            name: func.name,
            return_type_id: self.resolve_type(&func.return_type)?,
            params: self.resolve_params(&func.params)?,
            is_const: func.is_const,
            is_ref: func.is_ref,
        })
    }

    pub fn parse_property_declaration(
        &self,
        declaration: &str,
    ) -> Result<PropertySignature, String> {
        // Add semicolon temporarily for parser, then remove it from result
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

        Ok(PropertySignature {
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

    /// Parse a behaviour declaration: "void f()" or "Type@ f(int x)"
    pub fn parse_behaviour_declaration(
        &self,
        declaration: &str,
    ) -> Result<BehaviourSignature, String> {
        // Behaviours are just function declarations
        let func_sig = self.parse_function_declaration(declaration)?;

        Ok(BehaviourSignature {
            return_type_id: func_sig.return_type_id,
            params: func_sig.params,
            is_ref: func_sig.is_ref,
        })
    }

    /// Resolve Type AST to TypeId (when wrapped in Option)
    fn resolve_type(&self, type_ast: &Option<Type>) -> Result<u32, String> {
        match type_ast {
            Some(t) => self.resolve_type_def(t),
            None => Ok(0), // void/no return type
        }
    }

    /// Resolve Type AST to TypeId
    /// Resolve Type AST to TypeId
    fn resolve_type_def(&self, type_def: &Type) -> Result<u32, String> {
        let type_name = match &type_def.datatype {
            DataType::PrimType(name) => name.clone(),
            DataType::Identifier(name) => name.clone(),
            DataType::Auto => return Ok(TYPE_AUTO),
            DataType::Question => return Ok(0),
        };

        // Check for primitive types FIRST (don't need engine registry)
        if let Some(prim_type_id) = self.get_primitive_type_id(&type_name) {
            return Ok(prim_type_id);
        }

        // Then check engine registry for user-defined types
        let engine = self.engine.read().unwrap();
        engine
            .get_type_id(&type_name)
            .ok_or_else(|| format!("Unknown type: {}", type_name))
    }

    /// Get primitive type ID without needing engine registry
    fn get_primitive_type_id(&self, type_name: &str) -> Option<u32> {
        use crate::compiler::semantic::*;

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

    /// Resolve parameters from AST
    fn resolve_params(&self, params: &[Param]) -> Result<Vec<MethodParam>, String> {
        params
            .iter()
            .map(|p| {
                Ok(MethodParam {
                    name: p.name.clone().unwrap_or_default(),
                    type_id: self.resolve_type_def(&p.param_type)?,
                    is_ref: matches!(p.type_mod, Some(TypeMod::InOut) | Some(TypeMod::Out)),
                    is_out: matches!(p.type_mod, Some(TypeMod::Out)),
                    is_const: p.param_type.is_const,
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub return_type_id: u32,
    pub params: Vec<MethodParam>,
    pub is_const: bool,
    pub is_ref: bool,
}

#[derive(Debug, Clone)]
pub struct PropertySignature {
    pub name: String,
    pub type_id: u32,
    pub is_const: bool,
    pub is_handle: bool,
}

#[derive(Debug, Clone)]
pub struct BehaviourSignature {
    pub return_type_id: u32,
    pub params: Vec<MethodParam>,
    pub is_ref: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::engine::{ScriptEngine, TypeFlags};

    struct MyClass {}

    #[test]
    fn test_parse_simple_function() {
        let mut engine = ScriptEngine::new();
        engine
            .register_object_type::<MyClass>("MyClass", TypeFlags::REF_TYPE)
            .unwrap();

        let parser = DeclarationParser::new(engine.inner.clone());
        let sig = parser
            .parse_function_declaration("void DoSomething()")
            .unwrap();

        assert_eq!(sig.name, "DoSomething");
        assert_eq!(sig.return_type_id, 0); // void
        assert_eq!(sig.params.len(), 0);
        assert!(!sig.is_const);
    }

    #[test]
    fn test_parse_function_with_params() {
        let engine = ScriptEngine::new();
        let parser = DeclarationParser::new(engine.inner.clone());

        let sig = parser
            .parse_function_declaration("int Add(int a, int b)")
            .unwrap();

        assert_eq!(sig.name, "Add");
        assert_eq!(sig.params.len(), 2);
        assert_eq!(sig.params[0].name, "a");
        assert_eq!(sig.params[1].name, "b");
    }

    #[test]
    fn test_parse_const_method() {
        let engine = ScriptEngine::new();
        let parser = DeclarationParser::new(engine.inner.clone());

        let sig = parser
            .parse_function_declaration("int GetValue() const")
            .unwrap();

        assert_eq!(sig.name, "GetValue");
        assert!(sig.is_const);
    }

    #[test]
    fn test_parse_property() {
        let engine = ScriptEngine::new();
        let parser = DeclarationParser::new(engine.inner.clone());

        let sig = parser.parse_property_declaration("int myProperty").unwrap();

        assert_eq!(sig.name, "myProperty");
        assert!(!sig.is_const);
        assert!(!sig.is_handle);
    }

    #[test]
    fn test_parse_handle_property() {
        let mut engine = ScriptEngine::new();
        engine
            .register_object_type::<MyClass>("MyClass", TypeFlags::REF_TYPE)
            .unwrap();

        let parser = DeclarationParser::new(engine.inner.clone());
        let sig = parser
            .parse_property_declaration("MyClass@ handle")
            .unwrap();

        assert_eq!(sig.name, "handle");
        assert!(sig.is_handle);
    }
}
