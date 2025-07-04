#[cfg(test)]
mod tests {
    use angelscript_addons::debugger::{BreakPoint, DebugAction, DebuggerState, DebuggerIO, Debugger, StdIO};
    use angelscript_core::core::engine::Engine;
    use angelscript_core::types::enums::GetModuleFlags;
    use std::collections::VecDeque;

    /// Mock I/O implementation for testing
    #[derive(Clone)]
    pub struct MockIO {
        pub inputs: VecDeque<String>,
        pub outputs: Vec<String>,
    }

    impl MockIO {
        pub fn new() -> Self {
            Self {
                inputs: VecDeque::new(),
                outputs: Vec::new(),
            }
        }

        pub fn queue_input(&mut self, input: &str) {
            self.inputs.push_back(input.to_string());
        }

        pub fn get_outputs(&self) -> &[String] {
            &self.outputs
        }

        pub fn output_contains(&self, text: &str) -> bool {
            self.outputs.iter().any(|output| output.contains(text))
        }
    }

    impl DebuggerIO for MockIO {
        fn output(&mut self, text: &str) {
            self.outputs.push(text.to_string());
        }

        fn input(&mut self) -> Option<String> {
            self.inputs.pop_front()
        }
    }

    pub type MockIODebugger = Debugger<MockIO>;

    // Helper function to reduce boilerplate
    fn create_test_engine() -> Engine {
        let mut engine = Engine::create().expect("Failed to create engine");
        engine
            .install(angelscript_addons::string::addon())
            .expect("Failed to install string addon");
        engine
            .set_message_callback(
                |msg, _| {
                    println!("AngelScript: {}", msg.message);
                },
                None,
            )
            .expect("Failed to set message callback");
        engine
    }

    #[test]
    fn test_interactive_continue_command() {
        let mut mock_io = MockIO::new();
        mock_io.queue_input("c");

        let engine = create_test_engine();
        let mut debugger = MockIODebugger::new(MockIO::new());
        debugger.add_function_breakpoint("test");

        let script = r#"
            int test() {
                return 42;
            }
        "#;

        let module = engine
            .get_module("TestModule", GetModuleFlags::CreateIfNotExists)
            .unwrap();
        module.add_script_section("test", script, 0).unwrap();
        module.build().unwrap();

        let func = module.get_function_by_decl("int test()").unwrap();
        let mut ctx = engine.create_context().unwrap();
        ctx.prepare(&func).unwrap();

        // Simulate the interactive session
        debugger.execute(&mut ctx).unwrap();

        // Check that prompt was shown and command was processed
        debugger.with_io(|io| {
            assert!(io.output_contains("[dbg]> "));
            assert!(io.output_contains("Reached break point 0 in file 'test' at line 3\n"));
            assert!(io.output_contains("test:3; int test()\n"));
        });
        
        debugger.with_state(|state| {
            assert_eq!(state.action, DebugAction::Continue);
            assert_eq!(state.breakpoints.len(), 1);
        });
    }

    // Test basic functionality without I/O
    #[test]
    fn test_debugger_basic_functionality() {
        let debugger = DebuggerState::with_io(StdIO); // Uses StdIO by default
        assert_eq!(debugger.action, DebugAction::Continue);
        assert_eq!(debugger.breakpoints.len(), 0);
    }

    #[test]
    fn test_breakpoint_creation() {
        // Test file breakpoint
        let file_bp = BreakPoint::new_file("test.as".to_string(), 10);
        assert_eq!(file_bp.name, "test.as");
        assert_eq!(file_bp.line_number, 10);
        assert!(!file_bp.is_function);
        assert!(file_bp.needs_adjusting);

        // Test function breakpoint
        let func_bp = BreakPoint::new_function("myFunction".to_string());
        assert_eq!(func_bp.name, "myFunction");
        assert_eq!(func_bp.line_number, 0);
        assert!(func_bp.is_function);
        assert!(!func_bp.needs_adjusting);
    }
}
