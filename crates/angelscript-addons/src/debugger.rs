// src/addons/debugger.rs

use angelscript_core::core::context::Context;
use angelscript_core::core::error::ScriptResult;
use angelscript_core::core::function::Function;
use angelscript_core::core::script_object::ScriptObject;
use angelscript_core::types::enums::{ContextState, ObjectTypeFlags, TypeId};
use angelscript_core::types::script_data::ScriptData;
use angelscript_core::types::script_memory::{ScriptMemoryLocation, Void};
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

/// Trait for debugger I/O operations
pub trait DebuggerIO {
    /// Output text to the user
    fn output(&mut self, text: &str);
    /// Get input from the user, returns None if no input available
    fn input(&mut self) -> Option<String>;
}

/// Standard I/O implementation using stdin/stdout
#[derive(Clone)]
pub struct StdIO;

impl DebuggerIO for StdIO {
    fn output(&mut self, text: &str) {
        print!("{}", text);
        io::stdout().flush().unwrap();
    }

    fn input(&mut self) -> Option<String> {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => Some(input.trim().to_string()),
            Err(_) => None,
        }
    }
}

/// Debug action types
#[derive(Debug, Clone, PartialEq)]
pub enum DebugAction {
    Continue,
    StepInto,
    StepOver,
    StepOut,
}

/// Breakpoint information
#[derive(Debug, Clone)]
pub struct BreakPoint {
    pub name: String,
    pub line_number: i32,
    pub is_function: bool,
    pub needs_adjusting: bool,
}

impl BreakPoint {
    pub fn new_file(file: String, line: i32) -> Self {
        Self {
            name: file,
            line_number: line,
            is_function: false,
            needs_adjusting: true,
        }
    }

    pub fn new_function(func_name: String) -> Self {
        Self {
            name: func_name,
            line_number: 0,
            is_function: true,
            needs_adjusting: false,
        }
    }
}

/// Type for ToString callbacks
pub type ToStringCallback<IO> =
    Box<dyn Fn(&ScriptMemoryLocation, i32, i32, &DebuggerState<IO>) -> String + Send + Sync>;

/// Internal state structure - just data, no methods
pub struct DebuggerState<IO: DebuggerIO> {
    pub action: DebugAction,
    pub last_function: Option<Function>,
    pub last_command_at_stack_level: u32,
    pub breakpoints: Vec<BreakPoint>,
    pub to_string_callbacks: HashMap<String, ToStringCallback<IO>>,
    pub io: IO,
}

impl<IO: DebuggerIO> DebuggerState<IO> {
    /// Create a new debugger state with custom I/O
    pub fn with_io(io: IO) -> Self {
        Self {
            action: DebugAction::Continue,
            last_function: None,
            last_command_at_stack_level: 0,
            breakpoints: Vec::new(),
            to_string_callbacks: HashMap::new(),
            io,
        }
    }
}

unsafe impl<IO: DebuggerIO> Send for DebuggerState<IO> {}
unsafe impl<IO: DebuggerIO> Sync for DebuggerState<IO> {}

/// Main debugger implementation - all methods here
#[derive(Clone)]
pub struct Debugger<IO: DebuggerIO + Clone> {
    state: Arc<Mutex<DebuggerState<IO>>>,
}

impl<IO: DebuggerIO + Clone> Debugger<IO> {
    pub fn new(io: IO) -> Self {
        let state = DebuggerState::with_io(io);
        Self {
            state: Arc::new(Mutex::new(state)),
        }
    }

    /// Execute context with debugging support - KEY METHOD
    pub fn execute(&mut self, context: &mut Context) -> ScriptResult<ContextState> {
        context.set_line_callback(
            |ctx: &Context, param: ScriptMemoryLocation| {
                let debugger = param.as_ref::<Self>();
                if let Ok(mut state) = debugger.state.lock() {
                    debugger.line_callback_impl(ctx, &mut state);
                }
            },
            Some(self),
        )?;

        let result = context.execute();
        context.clear_line_callback()?;
        result
    }

    /// Register a ToString callback for a specific type
    pub fn register_to_string_callback(
        &self,
        type_name: impl Into<String>,
        callback: ToStringCallback<IO>,
    ) {
        let mut state = self.state.lock().unwrap();
        state.to_string_callbacks.insert(type_name.into(), callback);
    }

    /// Main line callback implementation
    fn line_callback_impl(&self, ctx: &Context, state: &mut DebuggerState<IO>) {
        if ctx.get_state() != ContextState::Active {
            return;
        }

        let should_break = match state.action {
            DebugAction::Continue => self.check_breakpoint(ctx, state),
            DebugAction::StepOver => {
                if ctx.get_callstack_size() > state.last_command_at_stack_level {
                    self.check_breakpoint(ctx, state)
                } else {
                    true
                }
            }
            DebugAction::StepOut => {
                if ctx.get_callstack_size() >= state.last_command_at_stack_level {
                    self.check_breakpoint(ctx, state)
                } else {
                    true
                }
            }
            DebugAction::StepInto => {
                self.check_breakpoint(ctx, state);
                true
            }
        };

        if should_break {
            self.print_current_location(ctx, state);
            self.take_commands(ctx, state);
        }
    }

    /// Check if we should break at the current location
    fn check_breakpoint(&self, ctx: &Context, state: &mut DebuggerState<IO>) -> bool {
        let (line_number, _, file_opt) = ctx.get_line_number(0);
        let file = match file_opt {
            Some(f) => self.extract_filename(f),
            None => "{unnamed}".to_string(),
        };

        if let Some(func) = ctx.get_function(0) {
            if state
                .last_function
                .as_ref()
                .map_or(true, |last| last.get_id() != func.get_id())
            {
                self.adjust_breakpoints(&func, &file, line_number, state);
                state.last_function = Some(func);
            }
        }

        for (index, bp) in state.breakpoints.iter().enumerate() {
            if !bp.is_function && bp.line_number == line_number && bp.name == file {
                state.io.output(&format!(
                    "Reached break point {} in file '{}' at line {}\n",
                    index, file, line_number
                ));
                return true;
            }
        }

        false
    }

    /// Adjust breakpoints when entering a new function
    fn adjust_breakpoints(
        &self,
        func: &Function,
        file: &str,
        line_number: i32,
        state: &mut DebuggerState<IO>,
    ) {
        for bp in &mut state.breakpoints {
            if bp.is_function {
                if let Some(func_name) = func.get_name() {
                    if bp.name == func_name {
                        state.io.output(&format!(
                            "Entering function '{}'. Transforming it into break point\n",
                            bp.name
                        ));

                        bp.name = file.to_string();
                        bp.line_number = line_number;
                        bp.is_function = false;
                        bp.needs_adjusting = false;
                    }
                }
            } else if bp.needs_adjusting && bp.name == file {
                if let Ok(next_line) = func.find_next_line_with_code(bp.line_number) {
                    bp.needs_adjusting = false;
                    if next_line != bp.line_number {
                        state.io.output(&format!(
                            "Moving break point to next line with code at line {}\n",
                            next_line
                        ));
                        bp.line_number = next_line;
                    }
                }
            }
        }
    }

    fn extract_filename(&self, path: &str) -> String {
        path.split(['\\', '/']).last().unwrap_or(path).to_string()
    }

    fn print_current_location(&self, ctx: &Context, state: &mut DebuggerState<IO>) {
        let (line_number, _, file_opt) = ctx.get_line_number(0);
        let file = file_opt.unwrap_or("{unnamed}");

        if let Some(func) = ctx.get_function(0) {
            if let Ok(decl) = func.get_declaration(false, false, false) {
                state
                    .io
                    .output(&format!("{}:{}; {}\n", file, line_number, decl));
            }
        }
    }

    fn take_commands(&self, ctx: &Context, state: &mut DebuggerState<IO>) {
        loop {
            state.io.output("[dbg]> ");

            if let Some(command) = state.io.input() {
                if self.interpret_command(&command, ctx, state) {
                    break;
                }
            } else {
                state.action = DebugAction::Continue;
                break;
            }
        }
    }

    fn interpret_command(&self, cmd: &str, ctx: &Context, state: &mut DebuggerState<IO>) -> bool {
        if cmd.is_empty() {
            return true;
        }

        let chars: Vec<char> = cmd.chars().collect();
        match chars[0] {
            'c' => {
                state.action = DebugAction::Continue;
                true
            }
            's' => {
                state.action = DebugAction::StepInto;
                true
            }
            'n' => {
                state.action = DebugAction::StepOver;
                state.last_command_at_stack_level = ctx.get_callstack_size();
                true
            }
            'o' => {
                state.action = DebugAction::StepOut;
                state.last_command_at_stack_level = ctx.get_callstack_size();
                true
            }
            'b' => {
                self.handle_breakpoint_command(cmd, state);
                false
            }
            'r' => {
                self.handle_remove_breakpoint_command(cmd, state);
                false
            }
            'l' => {
                self.handle_list_command(cmd, ctx, state);
                false
            }
            'p' => {
                self.handle_print_command(cmd, ctx, state);
                false
            }
            'w' => {
                self.print_callstack(ctx, state);
                false
            }
            'a' => {
                if ctx.abort().is_ok() {
                    state.io.output("Script execution aborted\n");
                }
                true
            }
            'h' => {
                self.print_help(state);
                false
            }
            _ => {
                state.io.output("Unknown command\n");
                false
            }
        }
    }

    fn handle_breakpoint_command(&self, cmd: &str, state: &mut DebuggerState<IO>) {
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        if parts.len() < 2 {
            state
                .io
                .output("Incorrect format for setting break point, expected one of:\n");
            state.io.output(" b <file name>:<line number>\n");
            state.io.output(" b <function name>\n");
            return;
        }

        let arg = parts[1].trim();
        if let Some(colon_pos) = arg.find(':') {
            let file = arg[..colon_pos].to_string();
            let line_str = &arg[colon_pos + 1..];
            if let Ok(line) = line_str.parse::<i32>() {
                self.add_file_breakpoint_impl(file, line, state);
            } else {
                state.io.output("Invalid line number\n");
            }
        } else {
            self.add_function_breakpoint_impl(arg.to_string(), state);
        }
    }

    fn handle_remove_breakpoint_command(&self, cmd: &str, state: &mut DebuggerState<IO>) {
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        if parts.len() < 2 {
            state
                .io
                .output("Incorrect format for removing break points, expected:\n");
            state.io.output(" r <all|number of break point>\n");
            return;
        }

        let arg = parts[1].trim();
        if arg == "all" {
            state.breakpoints.clear();
            state.io.output("All break points have been removed\n");
        } else if let Ok(index) = arg.parse::<usize>() {
            if index < state.breakpoints.len() {
                state.breakpoints.remove(index);
            }
            self.list_breakpoints(state);
        } else {
            state.io.output("Invalid breakpoint number\n");
        }
    }

    fn handle_list_command(&self, cmd: &str, ctx: &Context, state: &mut DebuggerState<IO>) {
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        if parts.len() < 2 {
            self.print_list_help(state);
            return;
        }

        match parts[1].trim().chars().next() {
            Some('b') => self.list_breakpoints(state),
            Some('v') => self.list_local_variables(ctx, state),
            Some('g') => self.list_global_variables(ctx, state),
            Some('m') => self.list_member_properties(ctx, state),
            Some('s') => self.list_statistics(ctx, state),
            _ => self.print_list_help(state),
        }
    }

    fn handle_print_command(&self, cmd: &str, ctx: &Context, state: &mut DebuggerState<IO>) {
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        if parts.len() < 2 {
            state.io.output("Incorrect format for print, expected:\n");
            state.io.output(" p <expression>\n");
            return;
        }

        let expression = parts[1].trim();
        self.print_value(expression, ctx, state);
    }

    /// Add a file breakpoint (public method)
    pub fn add_file_breakpoint(&self, file: impl Into<String>, line: i32) {
        let mut state = self.state.lock().unwrap();
        self.add_file_breakpoint_impl(file.into(), line, &mut state);
    }

    fn add_file_breakpoint_impl(&self, file: String, line: i32, state: &mut DebuggerState<IO>) {
        let filename = self.extract_filename(&file);
        state.io.output(&format!(
            "Setting break point in file '{}' at line {}\n",
            filename, line
        ));
        state.breakpoints.push(BreakPoint::new_file(filename, line));
    }

    /// Add a function breakpoint (public method)
    pub fn add_function_breakpoint(&self, func_name: impl Into<String>) {
        let mut state = self.state.lock().unwrap();
        self.add_function_breakpoint_impl(func_name.into(), &mut state);
    }

    fn add_function_breakpoint_impl(&self, func_name: String, state: &mut DebuggerState<IO>) {
        let trimmed = func_name.trim().to_string();
        state.io.output(&format!(
            "Adding deferred break point for function '{}'\n",
            trimmed
        ));
        state.breakpoints.push(BreakPoint::new_function(trimmed));
    }

    fn list_breakpoints(&self, state: &mut DebuggerState<IO>) {
        for (index, bp) in state.breakpoints.iter().enumerate() {
            if bp.is_function {
                state.io.output(&format!("{} - {}\n", index, bp.name));
            } else {
                state
                    .io
                    .output(&format!("{} - {}:{}\n", index, bp.name, bp.line_number));
            }
        }
    }

    fn list_local_variables(&self, ctx: &Context, state: &mut DebuggerState<IO>) {
        let var_count = ctx.get_var_count(0);

        for i in 0..var_count as u32 {
            if let Ok(var_info) = ctx.get_var(i, 0) {
                if let Some(name) = &var_info.name {
                    if !name.is_empty() && ctx.is_var_in_scope(i, 0) {
                        if let Some(decl) = ctx.get_var_declaration(i, 0, false) {
                            if let Some(addr) =
                                ctx.get_address_of_var::<ScriptMemoryLocation>(i, 0, false, false)
                            {
                                let value_str =
                                    self.to_string(&addr, var_info.type_id, 3, ctx, state);
                                state.io.output(&format!("{} = {}\n", decl, value_str));
                            }
                        }
                    }
                }
            }
        }
    }

    fn list_global_variables(&self, ctx: &Context, state: &mut DebuggerState<IO>) {
        if let Some(func) = ctx.get_function(0) {
            if let Some(module) = func.get_module() {
                let global_count = module.get_global_var_count();

                for i in 0..global_count {
                    if let Ok(var_info) = module.get_global_var(i) {
                        if let Some(addr) =
                            module.get_address_of_global_var::<ScriptMemoryLocation>(i)
                        {
                            let value_str = self.to_string(&addr, var_info.type_id, 3, ctx, state);
                            if let Some(decl) = module.get_global_var_declaration(i, false) {
                                state.io.output(&format!("{} = {}\n", decl, value_str));
                            }
                        }
                    }
                }
            }
        }
    }

    fn list_member_properties(&self, ctx: &Context, state: &mut DebuggerState<IO>) {
        if let Some(this_ptr) = ctx.get_this_pointer::<ScriptMemoryLocation>(0) {
            let type_id = ctx.get_this_type_id(0);
            let value_str = self.to_string(&this_ptr, type_id, 3, ctx, state);
            state.io.output(&format!("this = {}\n", value_str));
        }
    }

    fn list_statistics(&self, ctx: &Context, state: &mut DebuggerState<IO>) {
        if let Ok(engine) = ctx.get_engine() {
            let stats = engine.get_gc_statistics();
            state.io.output(&format!(
                "Garbage collector:\n current size: {}\n total destroyed: {}\n total detected: {}\n new objects: {}\n new objects destroyed: {}\n",
                stats.current_size,
                stats.total_destroyed,
                stats.total_detected,
                stats.new_objects,
                stats.total_new_destroyed
            ));
        }
    }

    fn print_callstack(&self, ctx: &Context, state: &mut DebuggerState<IO>) {
        let stack_size = ctx.get_callstack_size();

        for i in 0..stack_size {
            let (line, _, file) = ctx.get_line_number(i);
            let file_name = file.unwrap_or("{unnamed}");

            if let Some(func) = ctx.get_function(i) {
                if let Ok(decl) = func.get_declaration(false, false, false) {
                    state
                        .io
                        .output(&format!("{}:{}; {}\n", file_name, line, decl));
                }
            }
        }
    }

    // ========== ADVANCED EXPRESSION PARSING ==========

    /// Enhanced print_value with namespace resolution and member access
    fn print_value(&self, expression: &str, ctx: &Context, state: &mut DebuggerState<IO>) {
        // Check for complex expressions with member access
        if expression.contains('.') || expression.contains('[') {
            if let Some((addr, type_id)) = self.parse_and_evaluate_expression(expression, ctx) {
                let value_str = self.to_string(&addr, type_id, 3, ctx, state);
                state.io.output(&format!("{}\n", value_str));
                return;
            }
        }

        // Parse namespace and variable name
        let (scope, name) = self.parse_namespace_expression(expression, ctx);

        if name.is_empty() {
            state.io.output("Invalid expression. Expected identifier\n");
            return;
        }

        // Find the variable with namespace resolution
        if let Some((addr, type_id)) = self.find_variable_with_namespace(&scope, &name, ctx) {
            let value_str = self.to_string(&addr, type_id, 3, ctx, state);
            state.io.output(&format!("{}\n", value_str));
        } else {
            state.io.output("Invalid expression. No matching symbol\n");
        }
    }

    /// Parse complex expressions like "player.name" or "inventory[0].item"
    fn parse_and_evaluate_expression(
        &self,
        expression: &str,
        ctx: &Context,
    ) -> Option<(ScriptMemoryLocation, i32)> {
        let parts: Vec<&str> = expression.split('.').collect();

        if parts.is_empty() {
            return None;
        }

        // Start with the base variable (handle namespace resolution)
        let base_part = parts[0];
        let (scope, name) = if base_part.contains("::") {
            self.parse_namespace_expression(base_part, ctx)
        } else {
            (String::new(), base_part.to_string())
        };

        let mut current_value = self.find_variable_with_namespace(&scope, &name, ctx)?;

        // Process each member access
        for part in parts.iter().skip(1) {
            // Check for array indexing like "inventory[0]"
            if let Some((member_name, index)) = self.parse_array_access(part) {
                // First access the member
                current_value = self.access_object_member(
                    &current_value.0,
                    current_value.1,
                    &member_name,
                    ctx,
                )?;

                // Then handle array indexing (placeholder for future array implementation)
                current_value =
                    self.access_array_element(&current_value.0, current_value.1, index)?;
            } else {
                // Simple member access
                current_value =
                    self.access_object_member(&current_value.0, current_value.1, part, ctx)?;
            }
        }

        Some(current_value)
    }

    /// Parse namespace and variable name from expressions like "MyNamespace::variable"
    fn parse_namespace_expression(&self, expression: &str, ctx: &Context) -> (String, String) {
        let mut scope = String::new();
        let mut name = String::new();
        let mut current_token = String::new();

        let chars: Vec<char> = expression.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if i + 1 < chars.len() && chars[i] == ':' && chars[i + 1] == ':' {
                if scope.is_empty() && name.is_empty() {
                    scope = "::".to_string(); // Global scope marker
                } else if scope == "::" || scope.is_empty() {
                    scope = current_token.clone();
                } else {
                    scope = format!("{}::{}", scope, current_token);
                }
                current_token.clear();
                i += 2;
            } else {
                current_token.push(chars[i]);
                i += 1;
            }
        }

        name = current_token;

        // If no scope specified, use current function's namespace
        if scope.is_empty() {
            if let Some(func) = ctx.get_function(0) {
                if let Some(func_namespace) = func.get_namespace() {
                    scope = func_namespace.to_string();
                }
            }
        } else if scope == "::" {
            scope = String::new(); // Global namespace is empty string
        }

        (scope, name)
    }

    /// Find variable with namespace resolution
    fn find_variable_with_namespace(
        &self,
        scope: &str,
        name: &str,
        ctx: &Context,
    ) -> Option<(ScriptMemoryLocation, i32)> {
        // Try local variables first (only if no explicit scope)
        if scope.is_empty() {
            if let Some(result) = self.find_local_variable(name, ctx) {
                return Some(result);
            }

            // Try 'this' pointer
            if name == "this" {
                if let Some(this_ptr) = ctx.get_this_pointer::<ScriptMemoryLocation>(0) {
                    let type_id = ctx.get_this_type_id(0);
                    return Some((this_ptr, type_id));
                }
            }
        }

        // Try global variables with namespace matching
        if let Some(func) = ctx.get_function(0) {
            if let Some(module) = func.get_module() {
                let global_count = module.get_global_var_count();

                for i in 0..global_count {
                    if let Ok(var_info) = module.get_global_var(i) {
                        if let Some(var_name) = &var_info.name {
                            let var_namespace = var_info.namespace.as_deref().unwrap_or("");

                            if var_name == name && var_namespace == scope {
                                if let Some(addr) =
                                    module.get_address_of_global_var::<ScriptMemoryLocation>(i)
                                {
                                    return Some((addr, var_info.type_id));
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Find local variable by name
    fn find_local_variable(
        &self,
        name: &str,
        ctx: &Context,
    ) -> Option<(ScriptMemoryLocation, i32)> {
        let var_count = ctx.get_var_count(0);

        // Search from end like C++ version
        for i in (0..var_count as u32).rev() {
            if let Ok(var_info) = ctx.get_var(i, 0) {
                if let Some(var_name) = &var_info.name {
                    if var_name == name && ctx.is_var_in_scope(i, 0) {
                        if let Some(addr) =
                            ctx.get_address_of_var::<ScriptMemoryLocation>(i, 0, false, false)
                        {
                            return Some((addr, var_info.type_id));
                        }
                    }
                }
            }
        }
        None
    }

    /// Parse array access like "inventory[0]" -> ("inventory", 0)
    fn parse_array_access(&self, part: &str) -> Option<(String, usize)> {
        if let Some(bracket_start) = part.find('[') {
            if let Some(bracket_end) = part.find(']') {
                let member_name = &part[..bracket_start];
                let index_str = &part[bracket_start + 1..bracket_end];
                if let Ok(index) = index_str.parse::<usize>() {
                    return Some((member_name.to_string(), index));
                }
            }
        }
        None
    }

    /// Enhanced access_object_member with full native object support
    fn access_object_member(
        &self,
        obj_addr: &ScriptMemoryLocation,
        type_id: i32,
        member_name: &str,
        ctx: &Context,
    ) -> Option<(ScriptMemoryLocation, i32)> {
        // Handle script objects
        if (type_id & TypeId::ScriptObject.bits() as i32) != 0 {
            let script_obj: ScriptObject = ScriptData::from_script_ptr(obj_addr.as_mut_ptr());
            if let Some(prop_index) = script_obj.find_property_by_name(member_name) {
                let prop_type_id = script_obj.get_property_type_id(prop_index);
                if let Some(prop_addr) =
                    script_obj.get_address_of_property::<ScriptMemoryLocation>(prop_index)
                {
                    return Some((prop_addr, prop_type_id));
                }
            }
        } else {
            // Handle native object types using TypeInfo API
            if let Ok(engine) = ctx.get_engine() {
                if let Some(type_info) =
                    engine.get_type_info_by_id(TypeId::from_bits_truncate(type_id as u32))
                {
                    return self.access_native_object_property(
                        obj_addr,
                        type_id,
                        &type_info,
                        member_name,
                    );
                }
            }
        }
        None
    }

    /// Access properties of native objects using TypeInfo
    fn access_native_object_property(
        &self,
        obj_addr: &ScriptMemoryLocation,
        type_id: i32,
        type_info: &TypeInfo,
        member_name: &str,
    ) -> Option<(ScriptMemoryLocation, i32)> {
        // Search through all properties to find the matching name
        let property_count = type_info.get_property_count();

        for i in 0..property_count {
            if let Ok(prop_info) = type_info.get_property(i) {
                if let Some(prop_name) = &prop_info.name {
                    if prop_name == member_name {
                        // Calculate the property address using the offset information
                        return self.calculate_property_address(obj_addr, type_id, &prop_info);
                    }
                }
            }
        }
        None
    }

    /// Calculate the actual memory address of a property
    fn calculate_property_address(
        &self,
        obj_addr: &ScriptMemoryLocation,
        type_id: i32,
        prop_info: &TypePropertyInfo,
    ) -> Option<(ScriptMemoryLocation, i32)> {
        unsafe {
            let mut base_ptr = obj_addr.as_ptr();

            // Handle object handles (dereference if needed)
            if (type_id & TypeId::ObjHandle.bits() as i32) != 0 {
                base_ptr = *(base_ptr as *const *const Void);
                if base_ptr.is_null() {
                    return None;
                }
            }

            // Apply composite offset (for inheritance)
            if prop_info.composite_offset != 0 {
                base_ptr =
                    (base_ptr as *const u8).add(prop_info.composite_offset as usize) as *const Void;

                // Handle indirect composite access
                if prop_info.is_composite_indirect {
                    base_ptr = *(base_ptr as *const *const Void);
                    if base_ptr.is_null() {
                        return None;
                    }
                }
            }

            // Apply property offset
            let prop_ptr = (base_ptr as *const u8).add(prop_info.offset as usize) as *const Void;

            // Handle reference properties
            let final_ptr = if prop_info.is_reference {
                *(prop_ptr as *const *const Void)
            } else {
                prop_ptr
            };

            if final_ptr.is_null() {
                return None;
            }

            let prop_addr = ScriptMemoryLocation::from_const(final_ptr);
            Some((prop_addr, prop_info.type_id))
        }
    }

    /// Access array element - placeholder for future array implementation
    fn access_array_element(
        &self,
        _array_addr: &ScriptMemoryLocation,
        _type_id: i32,
        _index: usize,
    ) -> Option<(ScriptMemoryLocation, i32)> {
        // TODO: Implement when AngelScript array types are available
        // This would need to:
        // 1. Determine if it's a built-in array type or custom container
        // 2. Call the appropriate indexing method or calculate offset
        // 3. Handle bounds checking
        None
    }

    // ========== TO_STRING IMPLEMENTATION ==========

    /// Convert a value to string representation
    fn to_string(
        &self,
        value: &ScriptMemoryLocation,
        type_id: i32,
        expand_members: i32,
        ctx: &Context,
        state: &DebuggerState<IO>,
    ) -> String {
        if value.is_null() {
            return "<null>".to_string();
        }

        // Handle primitive types
        match type_id {
            id if id == TypeId::Void.bits() as i32 => "<void>".to_string(),
            id if id == TypeId::Bool.bits() as i32 => {
                let val = value.as_ref::<bool>();
                if *val { "true" } else { "false" }.to_string()
            }
            id if id == TypeId::Int8.bits() as i32 => {
                let val = value.as_ref::<i8>();
                (*val as i32).to_string()
            }
            id if id == TypeId::Int16.bits() as i32 => {
                let val = value.as_ref::<i16>();
                (*val as i32).to_string()
            }
            id if id == TypeId::Int32.bits() as i32 => {
                let val = value.as_ref::<i32>();
                val.to_string()
            }
            id if id == TypeId::Int64.bits() as i32 => {
                let val = value.as_ref::<i64>();
                val.to_string()
            }
            id if id == TypeId::Uint8.bits() as i32 => {
                let val = value.as_ref::<u8>();
                (*val as u32).to_string()
            }
            id if id == TypeId::Uint16.bits() as i32 => {
                let val = value.as_ref::<u16>();
                (*val as u32).to_string()
            }
            id if id == TypeId::Uint32.bits() as i32 => {
                let val = value.as_ref::<u32>();
                val.to_string()
            }
            id if id == TypeId::Uint64.bits() as i32 => {
                let val = value.as_ref::<u64>();
                val.to_string()
            }
            id if id == TypeId::Float.bits() as i32 => {
                let val = value.as_ref::<f32>();
                val.to_string()
            }
            id if id == TypeId::Double.bits() as i32 => {
                let val = value.as_ref::<f64>();
                val.to_string()
            }
            _ => {
                // Handle complex types
                if (type_id & TypeId::MaskObject.bits() as i32) == 0 {
                    // Enum type - show value and try to find enum name
                    let val = value.as_ref::<u32>();
                    let mut result = val.to_string();

                    if let Ok(engine) = ctx.get_engine() {
                        if let Some(type_info) =
                            engine.get_type_info_by_id(TypeId::from_bits_truncate(type_id as u32))
                        {
                            if let Some(type_name) = type_info.get_name() {
                                let enum_count = engine.get_enum_count();
                                for i in 0..enum_count {
                                    if let Some(enum_type) = engine.get_enum_by_index(i) {
                                        if let Some(enum_name) = enum_type.get_name() {
                                            if enum_name == type_name {
                                                let value_count = enum_type.get_enum_value_count();
                                                for j in 0..value_count {
                                                    if let Some((enum_val_name, enum_val)) =
                                                        enum_type.get_enum_value_by_index(j)
                                                    {
                                                        if enum_val == *val as i32 {
                                                            result.push_str(&format!(
                                                                ", {}",
                                                                enum_val_name
                                                            ));
                                                            break;
                                                        }
                                                    }
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    result
                } else if (type_id & TypeId::ScriptObject.bits() as i32) != 0 {
                    // Script objects
                    let mut obj_ptr = value.as_mut_ptr();

                    if (type_id & TypeId::ObjHandle.bits() as i32) != 0 {
                        obj_ptr = unsafe { *(obj_ptr as *const *mut Void) };
                    }

                    let mut result = format!("{{{:p}}}", obj_ptr);

                    // Expand script object members if requested
                    if !obj_ptr.is_null() && expand_members > 0 {
                        let script_obj: ScriptObject = ScriptData::from_script_ptr(obj_ptr);
                        let properties = script_obj.get_all_properties();

                        for (i, prop) in properties.iter().enumerate() {
                            if i == 0 {
                                result.push(' ');
                            } else {
                                result.push_str(", ");
                            }

                            let prop_name = prop.name.as_deref().unwrap_or("unknown");
                            let prop_value = self.to_string(
                                &prop.address,
                                prop.type_id,
                                expand_members - 1,
                                ctx,
                                state,
                            );
                            result.push_str(&format!("{} = {}", prop_name, prop_value));
                        }
                    }

                    result
                } else {
                    // Other object types
                    let mut obj_ptr = value.as_ptr();

                    if (type_id & TypeId::ObjHandle.bits() as i32) != 0 {
                        obj_ptr = unsafe { *(obj_ptr as *const *const Void) };
                    }

                    let mut result = String::new();

                    if let Ok(engine) = ctx.get_engine() {
                        if let Some(type_info) =
                            engine.get_type_info_by_id(TypeId::from_bits_truncate(type_id as u32))
                        {
                            let flags = type_info.get_flags();
                            if flags.contains(ObjectTypeFlags::REF) {
                                result.push_str(&format!("{{{:p}}}", obj_ptr));
                            }

                            if !obj_ptr.is_null() {
                                if let Some(type_name) = type_info.get_name() {
                                    // Check for registered ToString callback first
                                    if let Some(callback) = state.to_string_callbacks.get(type_name)
                                    {
                                        if !result.is_empty() {
                                            result.push(' ');
                                        }
                                        let obj_memory = ScriptMemoryLocation::from_const(obj_ptr);
                                        result.push_str(&callback(
                                            &obj_memory,
                                            type_id,
                                            expand_members,
                                            state,
                                        ));
                                        return result;
                                    }
                                    // Handle template types
                                    else if flags.contains(ObjectTypeFlags::TEMPLATE) {
                                        if let Some(template_pos) = type_name.find('<') {
                                            let base_name = &type_name[..template_pos];
                                            if let Some(callback) =
                                                state.to_string_callbacks.get(base_name)
                                            {
                                                if !result.is_empty() {
                                                    result.push(' ');
                                                }
                                                let obj_memory =
                                                    ScriptMemoryLocation::from_const(obj_ptr);
                                                result.push_str(&callback(
                                                    &obj_memory,
                                                    type_id,
                                                    expand_members,
                                                    state,
                                                ));
                                                return result;
                                            }
                                        }
                                    }
                                }

                                // Expand native object members using TypeInfo
                                if expand_members > 0 {
                                    let properties = self
                                        .get_native_object_properties(&type_info, obj_ptr, type_id);

                                    for (i, (prop_name, prop_addr, prop_type_id)) in
                                        properties.iter().enumerate()
                                    {
                                        if i == 0 {
                                            if !result.is_empty() {
                                                result.push(' ');
                                            }
                                        } else {
                                            result.push_str(", ");
                                        }

                                        let prop_value = self.to_string(
                                            prop_addr,
                                            *prop_type_id,
                                            expand_members - 1,
                                            ctx,
                                            state,
                                        );
                                        result.push_str(&format!("{} = {}", prop_name, prop_value));
                                    }
                                }
                            }
                        }
                    }

                    if result.is_empty() {
                        result = "{no engine}".to_string();
                    }

                    result
                }
            }
        }
    }

    /// Get all properties of a native object
    fn get_native_object_properties(
        &self,
        type_info: &TypeInfo,
        obj_ptr: *const Void,
        type_id: i32,
    ) -> Vec<(String, ScriptMemoryLocation, i32)> {
        let mut properties = Vec::new();
        let property_count = type_info.get_property_count();

        for i in 0..property_count {
            if let Ok(prop_info) = type_info.get_property(i) {
                if let Some(prop_name) = &prop_info.name {
                    // Only include public properties in expansion (optional)
                    if prop_info.is_public() {
                        let obj_addr = ScriptMemoryLocation::from_const(obj_ptr);
                        if let Some((prop_addr, prop_type_id)) =
                            self.calculate_property_address(&obj_addr, type_id, &prop_info)
                        {
                            properties.push((prop_name.clone(), prop_addr, prop_type_id));
                        }
                    }
                }
            }
        }

        properties
    }

    fn print_help(&self, state: &mut DebuggerState<IO>) {
        state.io.output(
            " c - Continue\n\
             s - Step into\n\
             n - Next step\n\
             o - Step out\n\
             b - Set break point\n\
             l - List various things\n\
             r - Remove break point\n\
             p - Print value\n\
             w - Where am I?\n\
             a - Abort execution\n\
             h - Print this help text\n",
        );
    }

    fn print_list_help(&self, state: &mut DebuggerState<IO>) {
        state.io.output(
            "Expected format:\n\
             l <list option>\n\
             Available options:\n\
             b - breakpoints\n\
             v - local variables\n\
             m - member properties\n\
             g - global variables\n\
             s - statistics\n",
        );
    }

    /// Access IO directly (if needed)
    pub fn with_io<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut IO) -> R,
    {
        let mut state = self.state.lock().unwrap();
        f(&mut state.io)
    }

    /// Access state directly (if needed)
    pub fn with_state<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&DebuggerState<IO>) -> R,
    {
        let mut state = self.state.lock().unwrap();
        f(&mut state)
    }
}

impl<IO: DebuggerIO + Clone> ScriptData for Debugger<IO> {
    fn to_script_ptr(&mut self) -> *mut Void {
        self as *mut Self as *mut Void
    }

    fn from_script_ptr(ptr: *mut Void) -> Self
    where
        Self: Sized,
    {
        unsafe { ptr.cast::<Self>().read() }
    }
}

// Type aliases
pub type StdIODebugger = Debugger<StdIO>;

// Convenience constructors
impl StdIODebugger {
    /// Create a new debugger with standard I/O
    pub fn new_stdio() -> Self {
        Self::new(StdIO)
    }
}

impl Default for StdIODebugger {
    fn default() -> Self {
        Self::new_stdio()
    }
}

// Need to import TypePropertyInfo from the TypeInfo module
use angelscript_core::core::typeinfo::{TypeInfo, TypePropertyInfo};
