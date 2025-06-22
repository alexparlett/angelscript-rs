use angelscript_core::core::engine::Engine;
use angelscript_core::core::error::ScriptError;
use angelscript_core::core::function::Function;
use angelscript_core::core::module::Module;
use angelscript_core::types::enums::{GetModuleFlags, TypeId};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a conditional block (#if, #elif, #else)
#[derive(Debug)]
struct ConditionalBlock {
    condition: Option<String>,
    condition_met: bool,
    directive_start: usize,
    directive_end: usize,
    content_start: usize,
    content_end: usize,
    block_type: BlockType,
}

#[derive(Debug, PartialEq)]
enum BlockType {
    If,
    Elif,
    Else,
}

/// A sophisticated script builder for AngelScript with preprocessing capabilities.
///
/// This is the Rust equivalent of the C++ CScriptBuilder addon, providing:
/// - File inclusion with cycle detection
/// - Conditional compilation (#if/#endif)
/// - Pragma directive support
/// - Metadata extraction and processing (when feature enabled)
/// - Path resolution and normalization
pub struct ScriptBuilder {
    engine: Option<Engine>,
    module: Option<Module>,

    // Include management
    included_scripts: HashSet<PathBuf>,
    include_callback: Option<IncludeCallback>,

    // Pragma support
    pragma_callback: Option<PragmaCallback>,

    // Preprocessor state
    defined_words: HashSet<String>,

    // Metadata processing (when enabled)
    #[cfg(feature = "script-builder-metadata")]
    metadata_processor: metadata::MetadataProcessor,
}

/// Callback for custom include processing
pub type IncludeCallback =
    Box<dyn Fn(&str, &str, &mut ScriptBuilder) -> Result<(), ScriptBuilderError> + Send + Sync>;

/// Callback for pragma directive processing
pub type PragmaCallback =
    Box<dyn Fn(&str, &mut ScriptBuilder) -> Result<(), ScriptBuilderError> + Send + Sync>;

/// Errors that can occur during script building
#[derive(Debug, thiserror::Error)]
pub enum ScriptBuilderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Script compilation error: {0}")]
    Compilation(String),

    #[error("Include cycle detected: {0}")]
    IncludeCycle(String),

    #[error("Invalid file path: {0}")]
    InvalidPath(String),

    #[error("Pragma error: {0}")]
    Pragma(String),

    #[error("AngelScript error: {0}")]
    AngelScript(#[from] ScriptError),

    #[cfg(feature = "script-builder-metadata")]
    #[error("Metadata processing error: {0}")]
    Metadata(String),

    #[error("Parse error: {0}")]
    Parse(String),
}

pub type ScriptBuilderResult<T> = Result<T, ScriptBuilderError>;

impl ScriptBuilder {
    /// Creates a new ScriptBuilder instance
    pub fn new() -> Self {
        Self {
            engine: None,
            module: None,
            included_scripts: HashSet::new(),
            include_callback: None,
            pragma_callback: None,
            defined_words: HashSet::new(),
            #[cfg(feature = "script-builder-metadata")]
            metadata_processor: metadata::MetadataProcessor::new(),
        }
    }

    #[cfg(feature = "script-builder-metadata")]
    pub fn metadata_processor(&self) -> &metadata::MetadataProcessor {
        &self.metadata_processor
    }

    /// Sets the include callback for custom include processing
    pub fn set_include_callback<F>(&mut self, callback: F)
    where
        F: Fn(&str, &str, &mut ScriptBuilder) -> Result<(), ScriptBuilderError>
            + Send
            + Sync
            + 'static,
    {
        self.include_callback = Some(Box::new(callback));
    }

    /// Sets the pragma callback for custom pragma processing
    pub fn set_pragma_callback<F>(&mut self, callback: F)
    where
        F: Fn(&str, &mut ScriptBuilder) -> Result<(), ScriptBuilderError> + Send + Sync + 'static,
    {
        self.pragma_callback = Some(Box::new(callback));
    }

    /// Starts building a new module
    pub fn start_new_module(
        &mut self,
        engine: &Engine,
        module_name: &str,
    ) -> ScriptBuilderResult<()> {
        let module = engine.get_module(module_name, GetModuleFlags::AlwaysCreate)?;

        self.engine = Some(engine.clone());
        self.module = Some(module);
        self.clear_all();

        Ok(())
    }

    /// Gets the current engine
    pub fn get_engine(&self) -> Option<&Engine> {
        self.engine.as_ref()
    }

    /// Gets the current module
    pub fn get_module(&self) -> Option<&Module> {
        self.module.as_ref()
    }

    /// Gets the number of included script sections
    pub fn get_section_count(&self) -> usize {
        self.included_scripts.len()
    }

    /// Gets the name of a script section by index
    pub fn get_section_name(&self, index: usize) -> Option<&Path> {
        self.included_scripts.iter().nth(index).map(|p| p.as_path())
    }

    /// Adds a script section from a file
    pub fn add_section_from_file<P: AsRef<Path>>(
        &mut self,
        filename: P,
    ) -> ScriptBuilderResult<bool> {
        let full_path = normalize_path(filename.as_ref())?;

        if self.include_if_not_already_included(&full_path) {
            self.load_script_section(&full_path)?;
            Ok(true)
        } else {
            Ok(false) // Already included
        }
    }

    /// Adds a script section from memory
    pub fn add_section_from_memory(
        &mut self,
        section_name: &str,
        script_code: &str,
        line_offset: i32,
    ) -> ScriptBuilderResult<bool> {
        let section_path = PathBuf::from(section_name);

        if self.include_if_not_already_included(&section_path) {
            self.process_script_section(script_code, section_name, line_offset)?;
            Ok(true)
        } else {
            Ok(false) // Already included
        }
    }

    /// Builds the module from all added sections
    pub fn build_module(&mut self) -> ScriptBuilderResult<()> {
        if let Some(module) = &self.module {
            module.build().map_err(ScriptBuilderError::from)?;

            #[cfg(feature = "script-builder-metadata")]
            {
                // Map metadata to compiled elements after successful build
                self.metadata_processor
                    .map_metadata_to_compiled_elements(module)?;
            }

            Ok(())
        } else {
            Err(ScriptBuilderError::Compilation(
                "No module to build".to_string(),
            ))
        }
    }

    /// Defines a preprocessor word for conditional compilation
    pub fn define_word(&mut self, word: &str) {
        self.defined_words.insert(word.to_string());
    }

    /// Checks if a preprocessor word is defined
    pub fn is_word_defined(&self, word: &str) -> bool {
        self.defined_words.contains(word)
    }

    /// Removes a preprocessor word definition
    pub fn undefine_word(&mut self, word: &str) {
        self.defined_words.remove(word);
    }

    /// Gets all defined preprocessor words
    pub fn get_defined_words(&self) -> &HashSet<String> {
        &self.defined_words
    }

    /// Clears all internal state
    fn clear_all(&mut self) {
        self.included_scripts.clear();
        #[cfg(feature = "script-builder-metadata")]
        self.metadata_processor.clear();
    }

    /// Checks if a file should be included and marks it as included
    fn include_if_not_already_included(&mut self, path: &Path) -> bool {
        if self.included_scripts.contains(path) {
            false
        } else {
            self.included_scripts.insert(path.to_path_buf());
            true
        }
    }

    /// Loads and processes a script file
    fn load_script_section(&mut self, filename: &Path) -> ScriptBuilderResult<()> {
        let content = fs::read_to_string(filename).map_err(|e| ScriptBuilderError::Io(e))?;

        let section_name = filename.to_string_lossy();
        self.process_script_section(&content, &section_name, 0)
    }

    /// Processes script content with preprocessing
    fn process_script_section(
        &mut self,
        script: &str,
        section_name: &str,
        line_offset: i32,
    ) -> ScriptBuilderResult<()> {
        let mut modified_script = script.to_string();
        let mut includes = Vec::new();

        // First pass: Handle conditional compilation
        self.process_conditional_compilation(&mut modified_script)?;

        // Second pass: Handle includes, pragmas, and metadata
        self.process_directives(&mut modified_script, section_name, &mut includes)?;

        // Add the processed script to the module
        if let Some(module) = &self.module {
            module.add_script_section(section_name, &modified_script, line_offset)?;
        }

        // Process includes
        self.process_includes(&includes, section_name)?;

        Ok(())
    }

    /// Processes conditional compilation directives (#if/#elif/#else/#endif)
    fn process_conditional_compilation(&self, script: &mut String) -> ScriptBuilderResult<()> {
        let mut chars: Vec<char> = script.chars().collect();
        let mut pos = 0;

        while pos < chars.len() {
            if let Some(token) = self.parse_token_at(&chars, pos) {
                match token {
                    ParsedToken::Directive { name, start, .. } if name == "if" => {
                        pos = self.process_conditional_block(&mut chars, start)?;
                    }
                    _ => pos += 1,
                }
            } else {
                pos += 1;
            }
        }

        *script = chars.into_iter().collect();
        Ok(())
    }

    /// Processes a complete conditional block, handling nesting properly
    fn process_conditional_block(
        &self,
        chars: &mut Vec<char>,
        start_pos: usize,
    ) -> ScriptBuilderResult<usize> {
        let mut pos = start_pos;
        let mut blocks = Vec::new();
        let mut nested_level = 0;
        let mut current_block_start = start_pos;

        // Process the initial #if
        if let Some(token) = self.parse_token_at(chars, pos) {
            if let ParsedToken::Directive {
                name,
                content,
                start,
                end,
            } = token
            {
                if name == "if" {
                    let word = content.trim();
                    let condition_met = self.defined_words.contains(word);
                    blocks.push(ConditionalBlock {
                        condition: Some(word.to_string()),
                        condition_met,
                        directive_start: start,
                        directive_end: end,
                        content_start: end,
                        content_end: 0,
                        block_type: BlockType::If,
                    });
                    pos = end;
                    current_block_start = end;
                }
            }
        }

        // Process the rest of the conditional structure
        while pos < chars.len() {
            if let Some(token) = self.parse_token_at(chars, pos) {
                match token {
                    ParsedToken::Directive {
                        name,
                        content,
                        start,
                        end,
                    } => {
                        match name.as_str() {
                            "if" => {
                                // Nested #if - increment nesting level
                                nested_level += 1;
                                pos = end;
                            }
                            "elif" if nested_level == 0 => {
                                // Close the previous block
                                if let Some(last_block) = blocks.last_mut() {
                                    last_block.content_end = start;
                                }

                                let word = content.trim();
                                let condition_met = self.defined_words.contains(word);
                                blocks.push(ConditionalBlock {
                                    condition: Some(word.to_string()),
                                    condition_met,
                                    directive_start: start,
                                    directive_end: end,
                                    content_start: end,
                                    content_end: 0,
                                    block_type: BlockType::Elif,
                                });
                                pos = end;
                            }
                            "else" if nested_level == 0 => {
                                // Close the previous block
                                if let Some(last_block) = blocks.last_mut() {
                                    last_block.content_end = start;
                                }

                                blocks.push(ConditionalBlock {
                                    condition: None,
                                    condition_met: true,
                                    directive_start: start,
                                    directive_end: end,
                                    content_start: end,
                                    content_end: 0,
                                    block_type: BlockType::Else,
                                });
                                pos = end;
                            }
                            "endif" => {
                                if nested_level == 0 {
                                    // This is our matching #endif
                                    if let Some(last_block) = blocks.last_mut() {
                                        last_block.content_end = start;
                                    }

                                    // Apply the conditional logic
                                    self.apply_conditional_blocks(chars, &blocks, start, end)?;
                                    return Ok(end);
                                } else {
                                    // This belongs to a nested block
                                    nested_level -= 1;
                                    pos = end;
                                }
                            }
                            _ => pos += 1,
                        }
                    }
                    _ => pos += 1,
                }
            } else {
                pos += 1;
            }
        }

        Err(ScriptBuilderError::Parse(
            "Unmatched #if directive - missing #endif".to_string(),
        ))
    }

    /// Applies conditional compilation logic to blocks
    fn apply_conditional_blocks(
        &self,
        chars: &mut Vec<char>,
        blocks: &[ConditionalBlock],
        endif_start: usize,
        endif_end: usize,
    ) -> ScriptBuilderResult<()> {
        // Find the first block whose condition is satisfied
        let mut selected_block_index = None;

        for (i, block) in blocks.iter().enumerate() {
            match block.block_type {
                BlockType::If | BlockType::Elif => {
                    if block.condition_met {
                        selected_block_index = Some(i);
                        break;
                    }
                }
                BlockType::Else => {
                    // #else is selected only if no previous condition was met
                    if selected_block_index.is_none() {
                        selected_block_index = Some(i);
                    }
                    break; // #else is always the last option
                }
            }
        }

        // First, remove all directive lines
        for block in blocks {
            self.overwrite_range(chars, block.directive_start, block.directive_end);
        }
        self.overwrite_range(chars, endif_start, endif_end);

        // Then, exclude content from non-selected blocks
        for (i, block) in blocks.iter().enumerate() {
            if Some(i) != selected_block_index {
                self.exclude_range_preserving_newlines(
                    chars,
                    block.content_start,
                    block.content_end,
                );
            }
        }

        // If we have a selected block, we need to recursively process any nested conditionals within it
        if let Some(selected_index) = selected_block_index {
            let selected_block = &blocks[selected_index];
            let block_content_start = selected_block.content_start;
            let block_content_end = selected_block.content_end;

            // Create a substring for recursive processing
            let mut block_chars: Vec<char> = chars[block_content_start..block_content_end].to_vec();
            let mut block_script = block_chars.iter().collect::<String>();

            // Recursively process nested conditionals
            self.process_conditional_compilation(&mut block_script)?;

            // Replace the original content with the processed content
            let processed_chars: Vec<char> = block_script.chars().collect();

            // Replace the range in the original chars vector
            for (i, &ch) in processed_chars.iter().enumerate() {
                if block_content_start + i < chars.len() {
                    chars[block_content_start + i] = ch;
                }
            }

            // If the processed content is shorter, fill the rest with spaces
            for i in
                (block_content_start + processed_chars.len())..block_content_end.min(chars.len())
            {
                if chars[i] != '\n' {
                    chars[i] = ' ';
                }
            }
        }

        Ok(())
    }

    /// Excludes a range of characters while preserving newlines for line number accuracy
    fn exclude_range_preserving_newlines(&self, chars: &mut Vec<char>, start: usize, end: usize) {
        for i in start..end.min(chars.len()) {
            if chars[i] != '\n' {
                chars[i] = ' ';
            }
        }
    }

    /// Processes include directives, pragmas, shebangs, and metadata
    fn process_directives(
        &mut self,
        script: &mut String,
        section_name: &str,
        includes: &mut Vec<String>,
    ) -> ScriptBuilderResult<()> {
        let mut chars: Vec<char> = script.chars().collect();
        let mut pos = 0;

        while pos < chars.len() {
            // Update context tracking for metadata
            #[cfg(feature = "script-builder-metadata")]
            {
                pos = self.metadata_processor.update_class_context(script, pos);
                pos = self
                    .metadata_processor
                    .update_namespace_context(script, pos);
            }

            if let Some(token) = self.parse_token_at(&chars, pos) {
                match token {
                    ParsedToken::Directive {
                        name,
                        content,
                        start,
                        end,
                    } => {
                        match name.as_str() {
                            "include" => {
                                let include_file = self.parse_include_filename(&content)?;
                                includes.push(include_file);
                                self.overwrite_range(&mut chars, start, end);
                            }
                            "pragma" => {
                                self.process_pragma(&content)?;
                                self.overwrite_range(&mut chars, start, end);
                            }
                            _ => {}
                        }
                        pos = end;
                    }
                    ParsedToken::Shebang { start, end } => {
                        // Remove shebang lines completely
                        self.overwrite_range(&mut chars, start, end);
                        pos = end;
                    }
                    #[cfg(feature = "script-builder-metadata")]
                    ParsedToken::Metadata { start, end } => {
                        // Process metadata and get new position
                        pos = self
                            .metadata_processor
                            .process_metadata_at_position(script, start)
                            .map_err(|e| {
                                ScriptBuilderError::Metadata(format!(
                                    "Metadata processing failed: {:?}",
                                    e
                                ))
                            })?;
                        self.overwrite_range(&mut chars, start, end);
                    }
                    ParsedToken::Other { end, .. } => {
                        pos = end;
                    }
                    _ => pos += 1,
                }
            } else {
                pos += 1;
            }
        }

        *script = chars.into_iter().collect();
        Ok(())
    }

    /// Processes include files
    fn process_includes(
        &mut self,
        includes: &[String],
        current_section: &str,
    ) -> ScriptBuilderResult<()> {
        if self.include_callback.is_some() {
            let callback = self.include_callback.take().unwrap();

            for include in includes {
                callback(include, current_section, self)?;
            }

            self.include_callback = Some(callback);
        } else {
            // Default include processing
            let base_path = Path::new(current_section).parent().unwrap_or(Path::new(""));

            for include in includes {
                let include_path = if Path::new(include).is_relative() {
                    base_path.join(include)
                } else {
                    PathBuf::from(include)
                };

                self.add_section_from_file(&include_path)?;
            }
        }
        Ok(())
    }

    /// Processes a pragma directive
    fn process_pragma(&mut self, pragma_text: &str) -> ScriptBuilderResult<()> {
        if self.pragma_callback.is_some() {
            let callback = self.pragma_callback.take().unwrap();

            callback(pragma_text, self)?;

            self.pragma_callback = Some(callback);
        }
        Ok(())
    }

    /// Parses a token at a specific position
    fn parse_token_at(&self, chars: &[char], pos: usize) -> Option<ParsedToken> {
        if pos >= chars.len() {
            return None;
        }

        let start = pos;
        let ch = chars[pos];

        // Check for shebang lines (#!) - must be at start of line
        if ch == '#' && pos + 1 < chars.len() && chars[pos + 1] == '!' {
            // Check if this is at the start of the script or after a newline
            let is_line_start = pos == 0 || chars[pos - 1] == '\n';
            if is_line_start {
                let mut end = pos + 2;
                while end < chars.len() && chars[end] != '\n' {
                    end += 1;
                }
                // Include the newline if present
                if end < chars.len() && chars[end] == '\n' {
                    end += 1;
                }
                return Some(ParsedToken::Shebang { start, end });
            }
        }

        // Check for preprocessor directives
        if ch == '#' && pos + 1 < chars.len() {
            let mut end = pos + 1;

            // Skip whitespace after #
            while end < chars.len() && chars[end].is_whitespace() && chars[end] != '\n' {
                end += 1;
            }

            // Get directive name
            let name_start = end;
            while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                end += 1;
            }

            if end > name_start {
                let name: String = chars[name_start..end].iter().collect();

                // Get directive content (rest of line)
                while end < chars.len() && chars[end].is_whitespace() && chars[end] != '\n' {
                    end += 1;
                }

                let content_start = end;
                while end < chars.len() && chars[end] != '\n' {
                    end += 1;
                }

                let content: String = chars[content_start..end].iter().collect();

                return Some(ParsedToken::Directive {
                    name,
                    content,
                    start,
                    end,
                });
            }
        }

        #[cfg(feature = "script-builder-metadata")]
        {
            // Check for metadata
            if ch == '[' {
                let mut end = pos + 1;
                let mut level = 1;

                while level > 0 && end < chars.len() {
                    match chars[end] {
                        '[' => level += 1,
                        ']' => level -= 1,
                        _ => {}
                    }
                    end += 1;
                }

                if level == 0 {
                    return Some(ParsedToken::Metadata { start, end });
                }
            }
        }

        // Check for comments
        if ch == '/' && pos + 1 < chars.len() {
            if chars[pos + 1] == '/' {
                // Line comment
                let mut end = pos + 2;
                while end < chars.len() && chars[end] != '\n' {
                    end += 1;
                }
                return Some(ParsedToken::Comment { start, end });
            } else if chars[pos + 1] == '*' {
                // Block comment
                let mut end = pos + 2;
                while end + 1 < chars.len() {
                    if chars[end] == '*' && chars[end + 1] == '/' {
                        end += 2;
                        break;
                    }
                    end += 1;
                }
                return Some(ParsedToken::Comment { start, end });
            }
        }

        // Check for whitespace
        if ch.is_whitespace() {
            let mut end = pos;
            while end < chars.len() && chars[end].is_whitespace() {
                end += 1;
            }
            return Some(ParsedToken::Whitespace { start, end });
        }

        // Check for identifiers
        if ch.is_alphabetic() || ch == '_' {
            let mut end = pos;
            while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                end += 1;
            }
            let content: String = chars[pos..end].iter().collect();
            return Some(ParsedToken::Identifier {
                content,
                start,
                end,
            });
        }

        // Check for string literals
        if ch == '"' || ch == '\'' {
            let quote = ch;
            let mut end = pos + 1;
            let mut escaped = false;

            while end < chars.len() {
                if escaped {
                    escaped = false;
                } else if chars[end] == '\\' {
                    escaped = true;
                } else if chars[end] == quote {
                    end += 1;
                    break;
                }
                end += 1;
            }

            let content: String = chars[pos..end].iter().collect();
            return Some(ParsedToken::StringLiteral {
                content,
                start,
                end,
            });
        }

        // Check for numbers
        if ch.is_ascii_digit() {
            let mut end = pos;
            while end < chars.len()
                && (chars[end].is_ascii_digit()
                    || chars[end] == '.'
                    || chars[end] == 'f'
                    || chars[end] == 'F')
            {
                end += 1;
            }
            let content: String = chars[pos..end].iter().collect();
            return Some(ParsedToken::StringLiteral {
                content,
                start,
                end,
            });
        }

        // For other tokens, advance by one character
        Some(ParsedToken::Other {
            start,
            end: pos + 1,
        })
    }

    /// Parses include filename from directive content
    fn parse_include_filename(&self, content: &str) -> ScriptBuilderResult<String> {
        let trimmed = content.trim();
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            let filename = &trimmed[1..trimmed.len() - 1];
            if filename.contains('\n') {
                return Err(ScriptBuilderError::InvalidPath(format!(
                    "Include filename contains line break: {}",
                    filename
                )));
            }
            Ok(filename.to_string())
        } else {
            Err(ScriptBuilderError::InvalidPath(format!(
                "Invalid include syntax: {}",
                content
            )))
        }
    }

    /// Overwrites a range of characters with spaces (preserving newlines)
    fn overwrite_range(&self, chars: &mut Vec<char>, start: usize, end: usize) {
        for i in start..end {
            if i < chars.len() && chars[i] != '\n' {
                chars[i] = ' ';
            }
        }
    }

    // Metadata API (only available when feature is enabled)
    #[cfg(feature = "script-builder-metadata")]
    pub fn get_metadata_for_type(&self, type_id: TypeId) -> Option<&Vec<String>> {
        self.metadata_processor.get_metadata_for_type(type_id)
    }

    #[cfg(feature = "script-builder-metadata")]
    pub fn get_metadata_for_func(&self, func: &Function) -> Option<&Vec<String>> {
        self.metadata_processor.get_metadata_for_func(func)
    }

    #[cfg(feature = "script-builder-metadata")]
    pub fn get_metadata_for_var(&self, var_index: i32) -> Option<&Vec<String>> {
        self.metadata_processor.get_metadata_for_var(var_index)
    }

    #[cfg(feature = "script-builder-metadata")]
    pub fn get_metadata_for_type_property(
        &self,
        type_id: TypeId,
        var_index: i32,
    ) -> Option<&Vec<String>> {
        self.metadata_processor
            .get_metadata_for_type_property(type_id, var_index)
    }

    #[cfg(feature = "script-builder-metadata")]
    pub fn get_metadata_for_type_method(
        &self,
        type_id: TypeId,
        method: &Function,
    ) -> Option<&Vec<String>> {
        self.metadata_processor
            .get_metadata_for_type_method(type_id, method)
    }
}

/// Represents different types of parsed tokens
#[derive(Debug)]
enum ParsedToken {
    Directive {
        name: String,
        content: String,
        start: usize,
        end: usize,
    },
    #[cfg(feature = "script-builder-metadata")]
    Metadata {
        start: usize,
        end: usize,
    },
    Identifier {
        content: String,
        start: usize,
        end: usize,
    },
    StringLiteral {
        content: String,
        start: usize,
        end: usize,
    },
    Comment {
        start: usize,
        end: usize,
    },
    Whitespace {
        start: usize,
        end: usize,
    },
    Shebang {
        start: usize,
        end: usize,
    },
    Other {
        start: usize,
        end: usize,
    },
}

/// Normalizes a file path to absolute form with proper separators
fn normalize_path<P: AsRef<Path>>(path: P) -> ScriptBuilderResult<PathBuf> {
    let path = path.as_ref();

    // Convert to absolute path
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(ScriptBuilderError::Io)?
            .join(path)
    };

    // Canonicalize to resolve . and .. components
    absolute.canonicalize().map_err(ScriptBuilderError::Io)
}

/// Builder pattern for easier ScriptBuilder configuration
pub struct ScriptBuilderConfig {
    include_callback: Option<IncludeCallback>,
    pragma_callback: Option<PragmaCallback>,
    defined_words: HashSet<String>,
}

impl ScriptBuilderConfig {
    pub fn new() -> Self {
        Self {
            include_callback: None,
            pragma_callback: None,
            defined_words: HashSet::new(),
        }
    }

    pub fn with_include_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&str, &str, &mut ScriptBuilder) -> Result<(), ScriptBuilderError>
            + Send
            + Sync
            + 'static,
    {
        self.include_callback = Some(Box::new(callback));
        self
    }

    pub fn with_pragma_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&str, &mut ScriptBuilder) -> Result<(), ScriptBuilderError> + Send + Sync + 'static,
    {
        self.pragma_callback = Some(Box::new(callback));
        self
    }

    pub fn define_word<S: Into<String>>(mut self, word: S) -> Self {
        self.defined_words.insert(word.into());
        self
    }

    pub fn define_words<I, S>(mut self, words: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for word in words {
            self.defined_words.insert(word.into());
        }
        self
    }

    pub fn build(self) -> ScriptBuilder {
        let mut builder = ScriptBuilder::new();
        builder.include_callback = self.include_callback;
        builder.pragma_callback = self.pragma_callback;
        builder.defined_words = self.defined_words;
        builder
    }
}

impl Default for ScriptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ScriptBuilderConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "script-builder-metadata")]
pub mod metadata {
    use super::*;
    use angelscript_core::core::function::Function;
    use std::collections::HashMap;

    /// Handles metadata extraction and processing
    #[derive(Debug)]
    pub struct MetadataProcessor {
        current_class: String,
        current_namespace: String,
        found_declarations: Vec<MetadataDeclaration>,
        type_metadata_map: HashMap<TypeId, Vec<String>>,
        func_metadata_map: HashMap<i32, Vec<String>>,
        var_metadata_map: HashMap<i32, Vec<String>>,
        class_metadata_map: HashMap<TypeId, ClassMetadata>,
    }

    #[derive(Debug, Clone)]
    pub struct MetadataDeclaration {
        pub metadata: Vec<String>,
        pub name: String,
        pub declaration: String,
        pub decl_type: DeclarationType,
        pub parent_class: String,
        pub namespace: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum DeclarationType {
        Type,
        Function,
        Variable,
        VirtualProperty,
        FunctionOrVariable,
    }

    #[derive(Debug, Clone)]
    pub struct ClassMetadata {
        pub class_name: String,
        pub func_metadata_map: HashMap<i32, Vec<String>>,
        pub var_metadata_map: HashMap<i32, Vec<String>>,
    }

    #[derive(Debug)]
    struct DeclarationInfo {
        name: String,
        declaration: String,
        decl_type: DeclarationType,
    }

    impl MetadataProcessor {
        pub fn new() -> Self {
            Self {
                current_class: String::new(),
                current_namespace: String::new(),
                found_declarations: Vec::new(),
                type_metadata_map: HashMap::new(),
                func_metadata_map: HashMap::new(),
                var_metadata_map: HashMap::new(),
                class_metadata_map: HashMap::new(),
            }
        }

        pub fn clear(&mut self) {
            self.current_class.clear();
            self.current_namespace.clear();
            self.found_declarations.clear();
            self.type_metadata_map.clear();
            self.func_metadata_map.clear();
            self.var_metadata_map.clear();
            self.class_metadata_map.clear();
        }

        /// Processes metadata found in the script
        pub fn process_metadata_at_position(
            &mut self,
            script: &str,
            position: usize,
        ) -> ScriptBuilderResult<usize> {
            let chars: Vec<char> = script.chars().collect();
            let mut pos = position;

            // Extract metadata strings
            let metadata = self.extract_metadata(&chars, &mut pos)?;

            // Extract the following declaration
            let declaration_info = self.extract_declaration(&chars, &mut pos)?;

            // Store the metadata declaration if it's valid
            if declaration_info.decl_type != DeclarationType::Type
                || !declaration_info.name.is_empty()
            {
                self.found_declarations.push(MetadataDeclaration {
                    metadata,
                    name: declaration_info.name,
                    declaration: declaration_info.declaration,
                    decl_type: declaration_info.decl_type,
                    parent_class: self.current_class.clone(),
                    namespace: self.current_namespace.clone(),
                });
            }

            Ok(pos)
        }

        /// Updates the current class context when parsing
        pub fn update_class_context(&mut self, script: &str, position: usize) -> usize {
            let chars: Vec<char> = script.chars().collect();
            let mut pos = position;

            // Skip decorators first
            while pos < chars.len() {
                if let Some(token) = self.parse_token_at(&chars, pos) {
                    match token {
                        ParsedToken::Whitespace { end, .. } | ParsedToken::Comment { end, .. } => {
                            pos = end;
                        }
                        ParsedToken::Identifier { content, end, .. } => {
                            if content == "shared"
                                || content == "abstract"
                                || content == "mixin"
                                || content == "external"
                            {
                                pos = end;
                            } else if content == "class" || content == "interface" {
                                pos = end;

                                // Skip whitespace and comments to get class name
                                loop {
                                    if let Some(next_token) = self.parse_token_at(&chars, pos) {
                                        match next_token {
                                            ParsedToken::Whitespace { end, .. }
                                            | ParsedToken::Comment { end, .. } => {
                                                pos = end;
                                            }
                                            ParsedToken::Identifier { content, end, .. } => {
                                                self.current_class = content;
                                                pos = end;
                                                break;
                                            }
                                            _ => break,
                                        }
                                    } else {
                                        break;
                                    }
                                }

                                // Search until first { or ; is encountered
                                while pos < chars.len() {
                                    let ch = chars[pos];
                                    if ch == '{' {
                                        // Start of class body
                                        return pos + 1;
                                    } else if ch == ';' {
                                        // Forward declaration only
                                        self.current_class.clear();
                                        return pos + 1;
                                    }
                                    pos += 1;
                                }
                                return pos;
                            } else {
                                break;
                            }
                        }
                        _ => break,
                    }
                } else {
                    break;
                }
            }

            // Check for end of class
            if !self.current_class.is_empty() && chars.get(position) == Some(&'}') {
                self.current_class.clear();
                return position + 1;
            }

            position
        }

        /// Updates the current namespace context
        pub fn update_namespace_context(&mut self, script: &str, position: usize) -> usize {
            let chars: Vec<char> = script.chars().collect();
            let mut pos = position;

            if let Some(token) = self.parse_token_at(&chars, pos) {
                if let ParsedToken::Identifier { content, end, .. } = token {
                    if content == "namespace" {
                        pos = end;

                        // Skip whitespace and get namespace name
                        while pos < chars.len() {
                            if let Some(next_token) = self.parse_token_at(&chars, pos) {
                                match next_token {
                                    ParsedToken::Whitespace { end, .. }
                                    | ParsedToken::Comment { end, .. } => {
                                        pos = end;
                                    }
                                    ParsedToken::Identifier { content, end, .. } => {
                                        if !self.current_namespace.is_empty() {
                                            self.current_namespace.push_str("::");
                                        }
                                        self.current_namespace.push_str(&content);
                                        pos = end;
                                        break;
                                    }
                                    _ => break,
                                }
                            } else {
                                break;
                            }
                        }

                        // Find opening brace
                        while pos < chars.len() {
                            if chars[pos] == '{' {
                                return pos + 1;
                            }
                            pos += 1;
                        }
                        return pos;
                    }
                }
            }

            // Check for end of namespace
            if !self.current_namespace.is_empty() && chars.get(position) == Some(&'}') {
                if let Some(last_scope) = self.current_namespace.rfind("::") {
                    self.current_namespace.truncate(last_scope);
                } else {
                    self.current_namespace.clear();
                }
                return position + 1;
            }

            position
        }

        /// Maps stored metadata to compiled AngelScript elements
        pub fn map_metadata_to_compiled_elements(
            &mut self,
            module: &Module,
        ) -> ScriptBuilderResult<()> {
            for decl in &self.found_declarations {
                // Set the namespace for lookups
                module.set_default_namespace(&decl.namespace)?;

                match decl.decl_type {
                    DeclarationType::Type => {
                        if let Some(type_id) = module.get_type_id_by_decl(&decl.declaration) {
                            self.type_metadata_map
                                .insert(type_id, decl.metadata.clone());
                        }
                    }
                    DeclarationType::Function => {
                        if decl.parent_class.is_empty() {
                            // Global function
                            if let Some(func) = module.get_function_by_decl(&decl.declaration) {
                                self.func_metadata_map
                                    .insert(func.get_id(), decl.metadata.clone());
                            }
                        } else {
                            // Method
                            if let Some(type_id) = module.get_type_id_by_decl(&decl.parent_class) {
                                if !self.class_metadata_map.contains_key(&type_id) {
                                    self.class_metadata_map.insert(
                                        type_id,
                                        ClassMetadata {
                                            class_name: decl.parent_class.clone(),
                                            func_metadata_map: HashMap::new(),
                                            var_metadata_map: HashMap::new(),
                                        },
                                    );
                                }

                                if let Some(type_info) =
                                    module.get_type_info_by_decl(&decl.parent_class)
                                {
                                    if let Some(method) =
                                        type_info.get_method_by_decl(&decl.declaration, true)
                                    {
                                        if let Some(class_meta) =
                                            self.class_metadata_map.get_mut(&type_id)
                                        {
                                            class_meta
                                                .func_metadata_map
                                                .insert(method.get_id(), decl.metadata.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    DeclarationType::VirtualProperty => {
                        if decl.parent_class.is_empty() {
                            // Global virtual property
                            let getter_name = format!("get_{}", decl.declaration);
                            let setter_name = format!("set_{}", decl.declaration);

                            if let Some(getter) = module.get_function_by_name(&getter_name) {
                                self.func_metadata_map
                                    .insert(getter.get_id(), decl.metadata.clone());
                            }
                            if let Some(setter) = module.get_function_by_name(&setter_name) {
                                self.func_metadata_map
                                    .insert(setter.get_id(), decl.metadata.clone());
                            }
                        } else {
                            // Method virtual property
                            if let Some(type_id) = module.get_type_id_by_decl(&decl.parent_class) {
                                if !self.class_metadata_map.contains_key(&type_id) {
                                    self.class_metadata_map.insert(
                                        type_id,
                                        ClassMetadata {
                                            class_name: decl.parent_class.clone(),
                                            func_metadata_map: HashMap::new(),
                                            var_metadata_map: HashMap::new(),
                                        },
                                    );
                                }

                                if let Some(type_info) =
                                    module.get_type_info_by_decl(&decl.parent_class)
                                {
                                    let getter_name = format!("get_{}", decl.declaration);
                                    let setter_name = format!("set_{}", decl.declaration);

                                    if let Some(getter) =
                                        type_info.get_method_by_name(&getter_name, true)
                                    {
                                        if let Some(class_meta) =
                                            self.class_metadata_map.get_mut(&type_id)
                                        {
                                            class_meta
                                                .func_metadata_map
                                                .insert(getter.get_id(), decl.metadata.clone());
                                        }
                                    }
                                    if let Some(setter) =
                                        type_info.get_method_by_name(&setter_name, true)
                                    {
                                        if let Some(class_meta) =
                                            self.class_metadata_map.get_mut(&type_id)
                                        {
                                            class_meta
                                                .func_metadata_map
                                                .insert(setter.get_id(), decl.metadata.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    DeclarationType::Variable => {
                        if decl.parent_class.is_empty() {
                            // Global variable
                            if let Some(var_index) =
                                module.get_global_var_index_by_name(&decl.declaration)
                            {
                                self.var_metadata_map
                                    .insert(var_index, decl.metadata.clone());
                            }
                        } else {
                            // Class member variable
                            if let Some(type_id) = module.get_type_id_by_decl(&decl.parent_class) {
                                if !self.class_metadata_map.contains_key(&type_id) {
                                    self.class_metadata_map.insert(
                                        type_id,
                                        ClassMetadata {
                                            class_name: decl.parent_class.clone(),
                                            func_metadata_map: HashMap::new(),
                                            var_metadata_map: HashMap::new(),
                                        },
                                    );
                                }

                                if let Some(type_info) =
                                    module.get_type_info_by_decl(&decl.parent_class)
                                {
                                    // Find property by name
                                    for i in 0..type_info.get_property_count() {
                                        if let Some(prom_name) =
                                            type_info.get_property(i).ok().and_then(|p| p.name)
                                        {
                                            if prom_name == decl.declaration {
                                                if let Some(class_meta) =
                                                    self.class_metadata_map.get_mut(&type_id)
                                                {
                                                    class_meta
                                                        .var_metadata_map
                                                        .insert(i as i32, decl.metadata.clone());
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    DeclarationType::FunctionOrVariable => {
                        if decl.parent_class.is_empty() {
                            // Try global variable first
                            if let Some(var_index) = module.get_global_var_index_by_name(&decl.name)
                            {
                                self.var_metadata_map
                                    .insert(var_index, decl.metadata.clone());
                            } else if let Some(func) =
                                module.get_function_by_decl(&decl.declaration)
                            {
                                self.func_metadata_map
                                    .insert(func.get_id(), decl.metadata.clone());
                            }
                        } else {
                            // Try class member variable first, then method
                            if let Some(type_id) = module.get_type_id_by_decl(&decl.parent_class) {
                                if !self.class_metadata_map.contains_key(&type_id) {
                                    self.class_metadata_map.insert(
                                        type_id,
                                        ClassMetadata {
                                            class_name: decl.parent_class.clone(),
                                            func_metadata_map: HashMap::new(),
                                            var_metadata_map: HashMap::new(),
                                        },
                                    );
                                }

                                if let Some(type_info) =
                                    module.get_type_info_by_decl(&decl.parent_class)
                                {
                                    let mut found = false;

                                    // Try property first
                                    for i in 0..type_info.get_property_count() {
                                        if let Some(prop_name) =
                                            type_info.get_property(i).ok().and_then(|p| p.name)
                                        {
                                            if prop_name == decl.name {
                                                if let Some(class_meta) =
                                                    self.class_metadata_map.get_mut(&type_id)
                                                {
                                                    class_meta
                                                        .var_metadata_map
                                                        .insert(i as i32, decl.metadata.clone());
                                                }
                                                found = true;
                                                break;
                                            }
                                        }
                                    }

                                    // Try method if property not found
                                    if !found {
                                        if let Some(method) =
                                            type_info.get_method_by_decl(&decl.declaration, true)
                                        {
                                            if let Some(class_meta) =
                                                self.class_metadata_map.get_mut(&type_id)
                                            {
                                                class_meta
                                                    .func_metadata_map
                                                    .insert(method.get_id(), decl.metadata.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Reset namespace
            module.set_default_namespace("")?;
            Ok(())
        }

        /// Extracts metadata strings from brackets
        fn extract_metadata(
            &self,
            chars: &[char],
            pos: &mut usize,
        ) -> ScriptBuilderResult<Vec<String>> {
            let mut metadata = Vec::new();

            loop {
                if *pos >= chars.len() || chars[*pos] != '[' {
                    break;
                }

                let mut metadata_string = String::new();
                *pos += 1; // Skip opening '['

                let mut level = 1;
                while level > 0 && *pos < chars.len() {
                    let ch = chars[*pos];
                    match ch {
                        '[' => level += 1,
                        ']' => level -= 1,
                        _ => {}
                    }

                    if level > 0 {
                        metadata_string.push(ch);
                    }
                    *pos += 1;
                }

                metadata.push(metadata_string);

                // Skip whitespace and comments to check for more metadata
                while *pos < chars.len() {
                    if let Some(token) = self.parse_token_at(chars, *pos) {
                        match token {
                            ParsedToken::Whitespace { end, .. }
                            | ParsedToken::Comment { end, .. } => {
                                *pos = end;
                            }
                            _ => break,
                        }
                    } else {
                        break;
                    }
                }
            }

            Ok(metadata)
        }

        /// Extracts declaration information following metadata
        fn extract_declaration(
            &self,
            chars: &[char],
            pos: &mut usize,
        ) -> ScriptBuilderResult<DeclarationInfo> {
            let mut declaration = String::new();
            let mut name = String::new();

            // Skip leading decorators and whitespace
            while *pos < chars.len() {
                if let Some(token) = self.parse_token_at(chars, *pos) {
                    match token {
                        ParsedToken::Whitespace { end, .. } | ParsedToken::Comment { end, .. } => {
                            *pos = end;
                        }
                        ParsedToken::Identifier { content, end, .. } => {
                            if self.is_decorator(&content) {
                                *pos = end;
                            } else {
                                break;
                            }
                        }
                        _ => break,
                    }
                } else {
                    break;
                }
            }

            // Check for type declarations (class, interface, enum)
            if let Some(token) = self.parse_token_at(chars, *pos) {
                if let ParsedToken::Identifier { content, end, .. } = token {
                    if content == "interface" || content == "class" || content == "enum" {
                        *pos = end;

                        // Skip whitespace and comments
                        while *pos < chars.len() {
                            if let Some(next_token) = self.parse_token_at(chars, *pos) {
                                match next_token {
                                    ParsedToken::Whitespace { end, .. }
                                    | ParsedToken::Comment { end, .. } => {
                                        *pos = end;
                                    }
                                    ParsedToken::Identifier { content, end, .. } => {
                                        declaration = content.clone();
                                        name = content;
                                        *pos = end;
                                        return Ok(DeclarationInfo {
                                            name,
                                            declaration,
                                            decl_type: DeclarationType::Type,
                                        });
                                    }
                                    _ => break,
                                }
                            } else {
                                break;
                            }
                        }
                    }
                }
            }

            // Parse function or variable declaration
            let mut has_parenthesis = false;
            let mut nested_parenthesis = 0;

            while *pos < chars.len() {
                let ch = chars[*pos];

                match ch {
                    '{' if nested_parenthesis == 0 => {
                        return if has_parenthesis {
                            // Function with body
                            Ok(DeclarationInfo {
                                name,
                                declaration,
                                decl_type: DeclarationType::Function,
                            })
                        } else {
                            // Virtual property
                            Ok(DeclarationInfo {
                                name: name.clone(),
                                declaration: name.clone(),
                                decl_type: DeclarationType::VirtualProperty,
                            })
                        };
                    }
                    '=' | ';' if !has_parenthesis => {
                        // Variable declaration
                        return Ok(DeclarationInfo {
                            name: name.clone(),
                            declaration: name.clone(),
                            decl_type: DeclarationType::Variable,
                        });
                    }
                    '=' | ';' if has_parenthesis => {
                        // Ambiguous: could be function prototype or variable with initialization
                        return Ok(DeclarationInfo {
                            name,
                            declaration,
                            decl_type: DeclarationType::FunctionOrVariable,
                        });
                    }
                    '(' => {
                        nested_parenthesis += 1;
                        has_parenthesis = true;
                        declaration.push(ch);
                    }
                    ')' => {
                        nested_parenthesis -= 1;
                        declaration.push(ch);
                    }
                    _ => {
                        // Check if this is an identifier token
                        if let Some(token) = self.parse_token_at(chars, *pos) {
                            match token {
                                ParsedToken::Identifier { content, end, .. } => {
                                    name = content.clone();

                                    // Skip trailing decorators if we're not in parentheses or at top level
                                    if !has_parenthesis
                                        || nested_parenthesis > 0
                                        || !self.is_trailing_decorator(&content)
                                    {
                                        declaration.push_str(&content);
                                    }
                                    *pos = end;
                                    continue;
                                }
                                ParsedToken::Whitespace { end, .. } => {
                                    declaration.push(' ');
                                    *pos = end;
                                    continue;
                                }
                                ParsedToken::StringLiteral { content, end, .. } => {
                                    declaration.push_str(&content);
                                    *pos = end;
                                    continue;
                                }
                                _ => {
                                    declaration.push(ch);
                                }
                            }
                        } else {
                            declaration.push(ch);
                        }
                    }
                }

                *pos += 1;
            }

            // Default to variable if we reach end
            Ok(DeclarationInfo {
                name,
                declaration,
                decl_type: DeclarationType::Variable,
            })
        }

        /// Checks if a token is a decorator
        fn is_decorator(&self, token: &str) -> bool {
            matches!(
                token,
                "private" | "protected" | "shared" | "external" | "final" | "abstract" | "mixin"
            )
        }

        /// Checks if a token is a trailing decorator
        fn is_trailing_decorator(&self, token: &str) -> bool {
            matches!(token, "final" | "override" | "delete" | "property")
        }

        /// Simple token parser for metadata processing
        fn parse_token_at(&self, chars: &[char], pos: usize) -> Option<ParsedToken> {
            if pos >= chars.len() {
                return None;
            }

            let start = pos;
            let ch = chars[pos];

            // Whitespace
            if ch.is_whitespace() {
                let mut end = pos;
                while end < chars.len() && chars[end].is_whitespace() {
                    end += 1;
                }
                return Some(ParsedToken::Whitespace { start, end });
            }

            // Comments
            if ch == '/' && pos + 1 < chars.len() {
                if chars[pos + 1] == '/' {
                    // Line comment
                    let mut end = pos + 2;
                    while end < chars.len() && chars[end] != '\n' {
                        end += 1;
                    }
                    return Some(ParsedToken::Comment { start, end });
                } else if chars[pos + 1] == '*' {
                    // Block comment
                    let mut end = pos + 2;
                    while end + 1 < chars.len() {
                        if chars[end] == '*' && chars[end + 1] == '/' {
                            end += 2;
                            break;
                        }
                        end += 1;
                    }
                    return Some(ParsedToken::Comment { start, end });
                }
            }

            // Identifiers
            if ch.is_alphabetic() || ch == '_' {
                let mut end = pos;
                while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                    end += 1;
                }
                let content: String = chars[pos..end].iter().collect();
                return Some(ParsedToken::Identifier {
                    content,
                    start,
                    end,
                });
            }

            // String literals
            if ch == '"' || ch == '\'' {
                let quote = ch;
                let mut end = pos + 1;
                let mut escaped = false;

                while end < chars.len() {
                    if escaped {
                        escaped = false;
                    } else if chars[end] == '\\' {
                        escaped = true;
                    } else if chars[end] == quote {
                        end += 1;
                        break;
                    }
                    end += 1;
                }

                let content: String = chars[pos..end].iter().collect();
                return Some(ParsedToken::StringLiteral {
                    content,
                    start,
                    end,
                });
            }

            // Numbers
            if ch.is_ascii_digit() {
                let mut end = pos;
                while end < chars.len()
                    && (chars[end].is_ascii_digit()
                        || chars[end] == '.'
                        || chars[end] == 'f'
                        || chars[end] == 'F')
                {
                    end += 1;
                }
                let content: String = chars[pos..end].iter().collect();
                return Some(ParsedToken::StringLiteral {
                    content,
                    start,
                    end,
                });
            }

            // Single character tokens
            Some(ParsedToken::Other {
                start,
                end: pos + 1,
            })
        }

        // Public API for retrieving metadata

        /// Gets metadata for a type by type ID
        pub fn get_metadata_for_type(&self, type_id: TypeId) -> Option<&Vec<String>> {
            self.type_metadata_map.get(&type_id)
        }

        /// Gets metadata for a function by function object
        pub fn get_metadata_for_func(&self, func: &Function) -> Option<&Vec<String>> {
            self.func_metadata_map.get(&func.get_id())
        }

        /// Gets metadata for a global variable by index
        pub fn get_metadata_for_var(&self, var_index: i32) -> Option<&Vec<String>> {
            self.var_metadata_map.get(&var_index)
        }

        /// Gets metadata for a type property
        pub fn get_metadata_for_type_property(
            &self,
            type_id: TypeId,
            var_index: i32,
        ) -> Option<&Vec<String>> {
            self.class_metadata_map
                .get(&type_id)
                .and_then(|class_meta| class_meta.var_metadata_map.get(&var_index))
        }

        /// Gets metadata for a type method
        pub fn get_metadata_for_type_method(
            &self,
            type_id: TypeId,
            method: &Function,
        ) -> Option<&Vec<String>> {
            self.class_metadata_map
                .get(&type_id)
                .and_then(|class_meta| class_meta.func_metadata_map.get(&method.get_id()))
        }

        /// Gets all metadata declarations found during processing
        pub fn get_all_declarations(&self) -> &[MetadataDeclaration] {
            &self.found_declarations
        }

        /// Gets the current class context
        pub fn get_current_class(&self) -> &str {
            &self.current_class
        }

        /// Gets the current namespace context
        pub fn get_current_namespace(&self) -> &str {
            &self.current_namespace
        }

        /// Gets all type metadata
        pub fn get_all_type_metadata(&self) -> &HashMap<TypeId, Vec<String>> {
            &self.type_metadata_map
        }

        /// Gets all function metadata
        pub fn get_all_function_metadata(&self) -> &HashMap<i32, Vec<String>> {
            &self.func_metadata_map
        }

        /// Gets all variable metadata
        pub fn get_all_variable_metadata(&self) -> &HashMap<i32, Vec<String>> {
            &self.var_metadata_map
        }

        /// Gets all class metadata
        pub fn get_all_class_metadata(&self) -> &HashMap<TypeId, ClassMetadata> {
            &self.class_metadata_map
        }

        /// Checks if a type has metadata
        pub fn has_type_metadata(&self, type_id: TypeId) -> bool {
            self.type_metadata_map.contains_key(&type_id)
        }

        /// Checks if a function has metadata
        pub fn has_function_metadata(&self, func: &Function) -> bool {
            self.func_metadata_map.contains_key(&func.get_id())
        }

        /// Checks if a variable has metadata
        pub fn has_variable_metadata(&self, var_index: i32) -> bool {
            self.var_metadata_map.contains_key(&var_index)
        }

        /// Gets metadata count for debugging
        pub fn get_metadata_stats(&self) -> MetadataStats {
            MetadataStats {
                declarations_found: self.found_declarations.len(),
                types_with_metadata: self.type_metadata_map.len(),
                functions_with_metadata: self.func_metadata_map.len(),
                variables_with_metadata: self.var_metadata_map.len(),
                classes_with_metadata: self.class_metadata_map.len(),
            }
        }
    }

    /// Statistics about metadata processing
    #[derive(Debug, Clone)]
    pub struct MetadataStats {
        pub declarations_found: usize,
        pub types_with_metadata: usize,
        pub functions_with_metadata: usize,
        pub variables_with_metadata: usize,
        pub classes_with_metadata: usize,
    }

    /// Token types for parsing
    #[derive(Debug, Clone)]
    pub enum ParsedToken {
        Identifier {
            content: String,
            start: usize,
            end: usize,
        },
        StringLiteral {
            content: String,
            start: usize,
            end: usize,
        },
        Whitespace {
            start: usize,
            end: usize,
        },
        Comment {
            start: usize,
            end: usize,
        },
        Other {
            start: usize,
            end: usize,
        },
    }

    impl Default for MetadataProcessor {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::script_builder::{ScriptBuilder, ScriptBuilderConfig};

    #[test]
    fn test_script_builder_creation() {
        let builder = ScriptBuilder::new();
        assert_eq!(builder.get_section_count(), 0);
        assert!(builder.get_engine().is_none());
        assert!(builder.get_module().is_none());
    }

    #[test]
    fn test_define_words() {
        let mut builder = ScriptBuilder::new();
        builder.define_word("DEBUG");
        builder.define_word("RELEASE");

        assert!(builder.is_word_defined("DEBUG"));
        assert!(builder.is_word_defined("RELEASE"));
        assert!(!builder.is_word_defined("UNKNOWN"));

        builder.undefine_word("DEBUG");
        assert!(!builder.is_word_defined("DEBUG"));
        assert!(builder.is_word_defined("RELEASE"));
    }

    #[test]
    fn test_config_builder() {
        let builder = ScriptBuilderConfig::new()
            .define_word("TEST")
            .define_words(vec!["DEBUG", "FEATURE_X"])
            .build();

        assert!(builder.is_word_defined("TEST"));
        assert!(builder.is_word_defined("DEBUG"));
        assert!(builder.is_word_defined("FEATURE_X"));
    }
}
