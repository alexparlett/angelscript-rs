use crate::compiler::bytecode::BytecodeModule;
use crate::core::engine::EngineInner;
use std::sync::{Arc, RwLock, Weak};
use crate::compiler::AngelscriptCompiler;
use crate::compiler::codegen::CodeGenerator;
use crate::compiler::semantic::SemanticAnalyzer;
use crate::parser::AngelScriptParser;

/// A module contains compiled scripts and their metadata
///
/// Note: Module itself is not thread-safe. Thread safety is the user's responsibility.
pub struct Module {
    /// Module name
    pub name: String,

    /// Compiled bytecode (populated after build())
    pub bytecode: Option<BytecodeModule>,

    /// Source code sections (added via add_script_section)
    pub sources: Vec<SourceSection>,

    /// Module-level symbols
    pub symbols: ModuleSymbols,

    /// Compilation state
    pub state: ModuleState,

    /// Reference to the engine that owns this module
    engine: Weak<RwLock<EngineInner>>,
}

#[derive(Clone)]
pub struct SourceSection {
    pub name: String,
    pub code: String,
}

#[derive(Default)]
pub struct ModuleSymbols {
    /// Functions defined in this module
    pub functions: Vec<FunctionDecl>,

    /// Classes defined in this module
    pub classes: Vec<ClassDecl>,

    /// Global variables in this module
    pub globals: Vec<GlobalDecl>,
}

#[derive(Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub type_id: u32,
}

#[derive(Clone)]
pub struct ClassDecl {
    pub name: String,
    pub type_id: u32,
}

#[derive(Clone)]
pub struct GlobalDecl {
    pub name: String,
    pub type_id: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    /// Module created but not compiled
    Empty,

    /// Currently being compiled
    Building,

    /// Successfully compiled
    Built,

    /// Compilation failed
    Failed,
}

impl Module {
    /// Create a new empty module
    pub(crate) fn new(name: String, engine: Weak<RwLock<EngineInner>>) -> Self {
        Self {
            name,
            bytecode: None,
            sources: Vec::new(),
            symbols: ModuleSymbols::default(),
            state: ModuleState::Empty,
            engine,
        }
    }

    /// Add a script section to the module
    ///
    /// Multiple sections can be added before calling build().
    /// This is useful for splitting large scripts into multiple files.
    pub fn add_script_section(&mut self, name: &str, code: &str) -> Result<(), String> {
        // Check if we can add sections
        match self.state {
            ModuleState::Building => {
                return Err("Cannot add sections while building".to_string());
            }
            ModuleState::Built => {
                // Allow adding more sections, but will need to rebuild
                self.state = ModuleState::Empty;
                self.bytecode = None;
                self.symbols = ModuleSymbols::default();
            }
            _ => {}
        }

        self.sources.push(SourceSection {
            name: name.to_string(),
            code: code.to_string(),
        });

        Ok(())
    }

    /// Build (compile) this module
    ///
    /// Returns 0 on success, negative value on error.
    pub fn build(&mut self) -> i32 {
        // Get engine reference
        let engine_ref = match self.engine.upgrade() {
            Some(engine) => engine,
            None => {
                eprintln!("Build error: Engine has been destroyed");
                return -1;
            }
        };

        match self.build_internal(engine_ref) {
            Ok(()) => 0,
            Err(errors) => {
                eprintln!("Build errors:");
                for error in errors {
                    eprintln!("  {}", error);
                }
                -1
            }
        }
    }

    /// Internal build implementation
    fn build_internal(
        &mut self,
        engine: Arc<RwLock<crate::core::engine::EngineInner>>,
    ) -> Result<(), Vec<String>> {
        // Check state
        match self.state {
            ModuleState::Built => {
                // Already built
                return Ok(());
            }
            ModuleState::Building => {
                return Err(vec!["Module is already being built".to_string()]);
            }
            _ => {}
        }

        // Mark as building
        self.state = ModuleState::Building;

        // Get all source code
        let source = self.get_full_source();

        if source.is_empty() {
            self.state = ModuleState::Failed;
            return Err(vec!["Module has no source code".to_string()]);
        }

        let module_name = self.name.clone();

        // Phase 1: Parse
        let ast = match AngelScriptParser::from_source(&source) {
            Ok(ast) => ast,
            Err(e) => {
                self.state = ModuleState::Failed;
                return Err(vec![format!("Parse error: {}", e)]);
            }
        };

        // Phase 2: Semantic Analysis
        let mut analyzer = SemanticAnalyzer::new(engine);

        if let Err(errors) = analyzer.analyze(&ast) {
            self.state = ModuleState::Failed;
            return Err(errors);
        }

        // Phase 3: Code Generation
        let mut codegen = CodeGenerator::new();
        let bytecode = codegen.generate(&ast, &analyzer);

        // Phase 4: Store results
        AngelscriptCompiler::extract_symbols(&ast, &mut self.symbols, &analyzer);

        self.bytecode = Some(bytecode);
        self.state = ModuleState::Built;

        // Log warnings
        if !analyzer.warnings.is_empty() {
            eprintln!("Build warnings for module '{}':", module_name);
            for warning in &analyzer.warnings {
                eprintln!("  Warning: {}", warning);
            }
        }

        Ok(())
    }

    /// Get all source code concatenated
    pub fn get_full_source(&self) -> String {
        self.sources
            .iter()
            .map(|s| s.code.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Check if module is built (compiled)
    pub fn is_built(&self) -> bool {
        self.state == ModuleState::Built
    }

    /// Get a function by name
    pub fn get_function_by_name(&self, name: &str) -> Option<&FunctionDecl> {
        self.symbols.functions.iter().find(|f| f.name == name)
    }

    /// Get a function by declaration (e.g., "int add(int, int)")
    pub fn get_function_by_decl(&self, declaration: &str) -> Option<&FunctionDecl> {
        // TODO: Parse declaration and match
        // For now, just extract the function name
        let name = declaration
            .split('(')
            .next()
            .and_then(|s| s.split_whitespace().last())?;

        self.get_function_by_name(name)
    }

    /// Get a class by name
    pub fn get_class(&self, name: &str) -> Option<&ClassDecl> {
        self.symbols.classes.iter().find(|c| c.name == name)
    }

    /// Discard the module (clear all data)
    pub fn discard(&mut self) {
        self.sources.clear();
        self.bytecode = None;
        self.symbols = ModuleSymbols::default();
        self.state = ModuleState::Empty;
    }
}
