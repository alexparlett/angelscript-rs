use angelscript::{ScriptEngine, GetModuleFlag, TypeFlags};

#[test]
fn test_basic_compilation() {
    let mut engine = ScriptEngine::new();

    let module = engine.get_module("test", GetModuleFlag::AlwaysCreate).unwrap();
    module.add_script_section("main", "void main() {}").unwrap();

    let result = module.build();
    assert_eq!(result, 0, "Build should succeed");
}

#[test]
fn test_simple_function() {
    let mut engine = ScriptEngine::new();

    let module = engine.get_module("test", GetModuleFlag::AlwaysCreate).unwrap();
    module.add_script_section("main", r#"
        int add(int a, int b) {
            return a + b;
        }
    "#).unwrap();

    let result = module.build();
    assert_eq!(result, 0, "Build should succeed");

    assert!(module.is_built());
    assert_eq!(module.symbols.functions.len(), 1);
    assert_eq!(module.symbols.functions[0].name, "add");
}

#[test]
fn test_class_definition() {
    let mut engine = ScriptEngine::new();

    let module = engine.get_module("test", GetModuleFlag::AlwaysCreate).unwrap();
    module.add_script_section("main", r#"
        class Player {
            int health;

            void takeDamage(int amount) {
                health = health - amount;
            }
        }
    "#).unwrap();

    let result = module.build();
    assert_eq!(result, 0, "Build should succeed");

    assert_eq!(module.symbols.classes.len(), 1);
    assert_eq!(module.symbols.classes[0].name, "Player");
}

#[test]
fn test_multiple_sections() {
    let mut engine = ScriptEngine::new();

    let module = engine.get_module("test", GetModuleFlag::AlwaysCreate).unwrap();

    module.add_script_section("types", r#"
        class Vector3 {
            float x;
            float y;
            float z;
        }
    "#).unwrap();

    module.add_script_section("main", r#"
        void main() {
            Vector3 v;
        }
    "#).unwrap();

    let result = module.build();
    assert_eq!(result, 0, "Build should succeed");
}

#[test]
fn test_registered_types() {
    let mut engine = ScriptEngine::new();

    engine.register_object_type_raw("string", 0, TypeFlags::REF_TYPE).unwrap();

    let module = engine.get_module("test", GetModuleFlag::AlwaysCreate).unwrap();
    module.add_script_section("main", r#"
        void main() {
            string s;
        }
    "#).unwrap();

    let result = module.build();
    assert_eq!(result, 0, "Build should succeed");
}

#[test]
fn test_syntax_error() {
    let mut engine = ScriptEngine::new();

    let module = engine.get_module("test", GetModuleFlag::AlwaysCreate).unwrap();
    module.add_script_section("main", r#"
        void main() {
            int x = ;
        }
    "#).unwrap();

    let result = module.build();
    assert!(result < 0, "Build should fail with syntax error");
}

#[test]
fn test_module_always_create() {
    let mut engine = ScriptEngine::new();

    let module1 = engine.get_module("test", GetModuleFlag::AlwaysCreate).unwrap();
    module1.add_script_section("main", "void func1() {}").unwrap();
    module1.build();

    let module2 = engine.get_module("test", GetModuleFlag::AlwaysCreate).unwrap();
    assert_eq!(module2.sources.len(), 0, "Module should be cleared");
}

#[test]
fn test_context_execution() {
    let mut engine = ScriptEngine::new();

    let module = engine.get_module("test", GetModuleFlag::AlwaysCreate).unwrap();
    module.add_script_section("main", r#"
        int getValue() {
            return 42;
        }
    "#).unwrap();

    let result = module.build();
    assert_eq!(result, 0);

    let mut ctx = engine.create_context();
    ctx.prepare(module, "getValue").unwrap();

    let exec_result = ctx.execute();
    assert!(exec_result.is_ok());
}
