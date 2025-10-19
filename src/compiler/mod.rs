pub mod bytecode;
pub mod codegen;
pub mod semantic;

use crate::compiler::semantic::SemanticAnalyzer;
use crate::core::module::{ClassDecl, FunctionDecl, GlobalDecl, ModuleSymbols};
use crate::parser::Script;
use crate::parser::ScriptItem;

pub struct AngelscriptCompiler;

impl AngelscriptCompiler {
    /// Extract symbol declarations from AST and store in module
    pub fn extract_symbols(ast: &Script, symbols: &mut ModuleSymbols, analyzer: &SemanticAnalyzer) {
        // Clear existing symbols
        symbols.functions.clear();
        symbols.classes.clear();
        symbols.globals.clear();

        for item in &ast.items {
            match item {
                ScriptItem::Func(func) => {
                    let type_id = if let Some(return_type) = &func.return_type {
                        analyzer.resolve_type_from_ast(return_type)
                    } else {
                        analyzer.lookup_type_id("void").unwrap_or(0)
                    };

                    symbols.functions.push(FunctionDecl {
                        name: func.name.clone(),
                        type_id,
                    });
                }

                ScriptItem::Class(class) => {
                    let type_id = analyzer.lookup_type_id(&class.name).unwrap_or(0);
                    symbols.classes.push(ClassDecl {
                        name: class.name.clone(),
                        type_id,
                    });
                }

                ScriptItem::Enum(enum_def) => {
                    let type_id = analyzer.lookup_type_id(&enum_def.name).unwrap_or(0);
                    symbols.classes.push(ClassDecl {
                        name: enum_def.name.clone(),
                        type_id,
                    });
                }

                ScriptItem::Interface(interface) => {
                    let type_id = analyzer.lookup_type_id(&interface.name).unwrap_or(0);
                    symbols.classes.push(ClassDecl {
                        name: interface.name.clone(),
                        type_id,
                    });
                }

                ScriptItem::Var(var) => {
                    let type_id = analyzer.resolve_type_from_ast(&var.var_type);
                    for decl in &var.declarations {
                        symbols.globals.push(GlobalDecl {
                            name: decl.name.clone(),
                            type_id,
                        });
                    }
                }

                ScriptItem::Namespace(ns) => {
                    Self::extract_symbols(
                        &Script {
                            items: ns.items.clone(),
                        },
                        symbols,
                        analyzer,
                    );
                }

                _ => {}
            }
        }
    }
}
