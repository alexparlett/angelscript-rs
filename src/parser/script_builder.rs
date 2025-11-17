use crate::core::error::*;
use crate::parser::ast::{Script, ScriptNode};
use crate::parser::lexer::Lexer;
use crate::parser::preprocessor::Preprocessor;
use std::collections::HashSet;

pub trait IncludeCallback {
    fn on_include(&mut self, include_path: &str, from_source: &str) -> ParseResult<String>;
}

pub trait PragmaCallback {
    fn on_pragma(&mut self, pragma_text: &str) -> ParseResult<()>;
}

pub struct ScriptBuilder {
    defined_words: HashSet<String>,
    include_callback: Option<Box<dyn IncludeCallback>>,
    pragma_callback: Option<Box<dyn PragmaCallback>>,
    included_sources: HashSet<String>,
    sections: Vec<(String, String)>,
}

impl ScriptBuilder {
    pub fn new() -> Self {
        Self {
            defined_words: HashSet::new(),
            include_callback: None,
            pragma_callback: None,
            included_sources: HashSet::new(),
            sections: Vec::new(),
        }
    }

    pub fn set_include_callback<C: IncludeCallback + 'static>(&mut self, callback: C) {
        self.include_callback = Some(Box::new(callback));
    }

    pub fn set_pragma_callback<C: PragmaCallback + 'static>(&mut self, callback: C) {
        self.pragma_callback = Some(Box::new(callback));
    }

    pub fn define_word(&mut self, word: String) {
        self.defined_words.insert(word);
    }

    pub fn is_defined(&self, word: &str) -> bool {
        self.defined_words.contains(word)
    }

    pub fn add_section(&mut self, name: &str, code: &str) {
        self.sections.push((name.to_string(), code.to_string()));
    }

    pub fn clear(&mut self) {
        self.defined_words.clear();
        self.included_sources.clear();
        self.sections.clear();
    }

    pub fn build(&mut self) -> ParseResult<Script> {
        if self.sections.is_empty() {
            return Err(ParseError::SyntaxError {
                span: None,
                message: "No source sections to build".to_string(),
            });
        }

        let mut all_items = Vec::new();

        for i in 0..self.sections.len() {
            let script = self.build_section(i)?;
            all_items.extend(script.items);
        }

        Ok(Script {
            items: all_items,
            span: None,
        })
    }

    fn build_section(&mut self, idx: usize) -> ParseResult<Script> {
        let section_name = &self.sections[idx].0.clone();
        let source = &self.sections[idx].1;
        
        let lexer = Lexer::new_with_name(section_name, source, true);
        let tokens = lexer.tokenize()?;

        let preprocessor = Preprocessor::new(tokens, self, section_name);
        let mut script = preprocessor.parse()?;

        script.items = self.process_items(section_name, script.items)?;

        Ok(script)
    }

    fn process_items(
        &mut self,
        current_section: &str,
        items: Vec<ScriptNode>,
    ) -> ParseResult<Vec<ScriptNode>> {
        let mut result = Vec::new();

        for item in items {
            match item {
                ScriptNode::Include(include) => {
                    let included_items = self.handle_include(&include.path, current_section)?;
                    result.extend(included_items);
                }
                ScriptNode::Namespace(mut ns) => {
                    ns.items = self.process_items(current_section, ns.items)?;
                    result.push(ScriptNode::Namespace(ns));
                }
                ScriptNode::Pragma(pragma) => {
                    if let Some(ref mut callback) = self.pragma_callback {
                        callback.on_pragma(&pragma.content)?;
                    }
                }
                ScriptNode::CustomDirective(_) => {}
                _ => {
                    result.push(item);
                }
            }
        }

        Ok(result)
    }

    fn handle_include(
        &mut self,
        include_path: &str,
        from_section: &str,
    ) -> ParseResult<Vec<ScriptNode>> {
        if self.included_sources.contains(include_path) {
            return Ok(Vec::new());
        }

        let included_source = if let Some(ref mut callback) = self.include_callback {
            callback.on_include(include_path, from_section)?
        } else {
            return Err(ParseError::SyntaxError {
                span: None,
                message: format!("No include callback set, cannot resolve: {}", include_path),
            });
        };

        self.included_sources.insert(include_path.to_string());

        let lexer = Lexer::new_with_name(include_path, &included_source, true);
        let tokens = lexer.tokenize()?;

        let preprocessor = Preprocessor::new(tokens, self, include_path);
        let included_script = preprocessor.parse()?;

        self.process_items(include_path, included_script.items)
    }
}

pub struct DefaultIncludeCallback {
    include_paths: Vec<std::path::PathBuf>,
}

impl DefaultIncludeCallback {
    pub fn new() -> Self {
        Self {
            include_paths: vec![std::path::PathBuf::from(".")],
        }
    }

    pub fn add_include_path(&mut self, path: std::path::PathBuf) {
        self.include_paths.push(path);
    }

    fn resolve_path(&self, filename: &str) -> Option<std::path::PathBuf> {
        use std::path::Path;

        let path = Path::new(filename);

        if path.is_absolute() && path.exists() {
            return Some(path.to_path_buf());
        }

        for include_path in &self.include_paths {
            let full_path = include_path.join(filename);
            if full_path.exists() {
                return Some(full_path);
            }
        }

        None
    }
}

impl IncludeCallback for DefaultIncludeCallback {
    fn on_include(&mut self, include_path: &str, _from_source: &str) -> ParseResult<String> {
        let resolved_path =
            self.resolve_path(include_path)
                .ok_or_else(|| ParseError::SyntaxError {
                    span: None,
                    message: format!("Include file not found: '{}'", include_path),
                })?;

        std::fs::read_to_string(&resolved_path).map_err(|e| ParseError::SyntaxError {
            span: None,
            message: format!("Failed to read '{}': {}", resolved_path.display(), e),
        })
    }
}

impl Default for DefaultIncludeCallback {
    fn default() -> Self {
        Self::new()
    }
}