pub mod ast;
pub mod error;
pub mod expr_parser;
pub mod lexer;
pub mod parser;
pub mod script_builder;
pub mod token;

pub use ast::Script;
pub use error::Result;
pub use lexer::Lexer;
pub use parser::Parser;

/// Main entry point for parsing AngelScript code
pub fn parse(source: &str) -> Result<Script> {
    // Tokenize
    let tokens = Lexer::new(source).tokenize()?;

    // Parse
    let parser = Parser::new(tokens);
    parser.parse()
}

/// Parse with preprocessor support
pub fn parse_with_preprocessor(
    source: &str,
    builder: &mut script_builder::ScriptBuilder,
) -> Result<Script> {
    // Parse first
    let script = parse(source)?;

    // Process preprocessor directives
    builder.process_script(source, script)
}

#[cfg(test)]
mod tests {
    use crate::parser::ast::*;
    use crate::parser::error::Result;
    use crate::parser::parse_with_preprocessor;
    use crate::parser::script_builder::ScriptBuilder;

    fn parse(source: &str) -> Result<Script> {
        let mut builder = ScriptBuilder::new();
        parse_with_preprocessor(source, &mut builder)
    }

    #[test]
    fn test_simple_function() {
        let source = r#"
            void main() {
                int x = 42;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        assert_eq!(script.items.len(), 1);

        match &script.items[0] {
            ScriptItem::Func(func) => {
                assert_eq!(func.name, "main");
                assert!(func.return_type.is_some());
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_class_with_constructor() {
        let source = r#"
            class MyClass {
                int value;

                MyClass(int v) {
                    value = v;
                }

                void method() {
                    value = 10;
                }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        assert_eq!(script.items.len(), 1);

        match &script.items[0] {
            ScriptItem::Class(class) => {
                assert_eq!(class.name, "MyClass");
                assert_eq!(class.members.len(), 3); // value, constructor, method

                // Check for constructor
                let has_constructor = class.members.iter().any(|m| {
                    if let ClassMember::Func(f) = m {
                        f.name == "MyClass" && f.return_type.is_none()
                    } else {
                        false
                    }
                });
                assert!(has_constructor, "Constructor not found");
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn test_class_with_destructor() {
        let source = r#"
            class MyClass {
                ~MyClass() {
                    // cleanup
                }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        match &script.items[0] {
            ScriptItem::Class(class) => {
                let has_destructor = class.members.iter().any(|m| {
                    if let ClassMember::Func(f) = m {
                        f.name.starts_with('~')
                    } else {
                        false
                    }
                });
                assert!(has_destructor, "Destructor not found");
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn test_enum() {
        let source = r#"
            enum Color {
                RED = 0,
                GREEN = 1,
                BLUE = 2
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        match &script.items[0] {
            ScriptItem::Enum(e) => {
                assert_eq!(e.name, "Color");
                assert_eq!(e.variants.len(), 3);
                assert_eq!(e.variants[0].name, "RED");
            }
            _ => panic!("Expected enum"),
        }
    }

    #[test]
    fn test_interface() {
        let source = r#"
            interface IController {
                void OnThink();
                void OnMessage(ref @m);
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        match &script.items[0] {
            ScriptItem::Interface(i) => {
                assert_eq!(i.name, "IController");
                assert_eq!(i.members.len(), 2);
            }
            _ => panic!("Expected interface"),
        }
    }

    #[test]
    fn test_class_inheritance() {
        let source = r#"
            class Derived : Base, IInterface {
                void method() {}
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        match &script.items[0] {
            ScriptItem::Class(class) => {
                assert_eq!(class.name, "Derived");
                assert_eq!(class.extends.len(), 2);
                assert_eq!(class.extends[0], "Base");
                assert_eq!(class.extends[1], "IInterface");
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn test_handle_syntax() {
        let source = r#"
            class Test {
                CGameObj @handle;

                void method(CGameObj @param) {
                    @handle = param;
                }
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse handle syntax: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_cast_syntax() {
        let source = r#"
            void test() {
                CMessage @msg = cast<CMessage>(m);
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse cast syntax: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_operators() {
        let source = r#"
            void test() {
                int a = 1 + 2 * 3;
                bool b = a > 5 && a < 10;
                int c = a << 2;
                a += 5;
                a++;
                ++a;
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse operators: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_is_operator() {
        let source = r#"
            void test() {
                if (msg !is null && msg is CMessage) {
                    // do something
                }
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse 'is' operator: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_switch_statement() {
        let source = r#"
            void test(int value) {
                switch(value) {
                    case 1:
                        print("one");
                        break;
                    case 2:
                        print("two");
                        break;
                    default:
                        print("other");
                        break;
                }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse switch: {:?}", result.err());
    }

    #[test]
    fn test_for_loop() {
        let source = r#"
            void test() {
                for (int i = 0; i < 10; i++) {
                    print(i);
                }
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse for loop: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_while_loop() {
        let source = r#"
            void test() {
                int i = 0;
                while (i < 10) {
                    i++;
                }
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse while loop: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_do_while_loop() {
        let source = r#"
            void test() {
                int i = 0;
                do {
                    i++;
                } while (i < 10);
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse do-while loop: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_ternary_operator() {
        let source = r#"
            void test() {
                int x = condition ? 1 : 2;
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse ternary: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_array_indexing() {
        let source = r#"
            void test() {
                int value = array[index];
                array[0] = 42;
                game.actionState[UP] = true;
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse array indexing: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_member_access() {
        let source = r#"
            void test() {
                player.x = 10;
                player.Move(1, 0);
                game.actionState[UP] = true;
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse member access: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_function_call() {
        let source = r#"
            void test() {
                print("hello");
                int result = add(1, 2);
                obj.method(arg1, arg2);
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse function calls: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_lambda() {
        let source = r#"
            void test() {
                auto callback = function(int x) {
                    return x * 2;
                };
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse lambda: {:?}", result.err());
    }

    #[test]
    fn test_template_types() {
        let source = r#"
            void test() {
                array<int> numbers;
                dictionary<string, int> map;
                const_weakref<CGameObj> playerRef;
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse template types: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_namespace() {
        let source = r#"
            namespace MyNamespace {
                void function() {}

                class MyClass {}
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse namespace: {:?}",
            result.err()
        );

        let script = result.unwrap();
        match &script.items[0] {
            ScriptItem::Namespace(ns) => {
                assert_eq!(ns.name[0], "MyNamespace");
                assert_eq!(ns.items.len(), 2);
            }
            _ => panic!("Expected namespace"),
        }
    }

    #[test]
    fn test_typedef() {
        let source = r#"
            typedef float real32;
            typedef double real64;
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse typedef: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_funcdef() {
        let source = r#"
            funcdef void Callback(int value);
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse funcdef: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_multiple_variable_declarations() {
        let source = r#"
            void test() {
                int a = 1, b = 2, c = 3;
                int x, y, z;
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse multiple declarations: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_const_parameters() {
        let source = r#"
            void test(const string &in text, int &out result) {
                result = 42;
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse const parameters: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_default_parameters() {
        let source = r#"
            void test(int x = 10, string s = "hello") {
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse default parameters: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_shared_class() {
        let source = r#"
            shared class CMessage {
                string txt;
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse shared class: {:?}",
            result.err()
        );

        let script = result.unwrap();
        match &script.items[0] {
            ScriptItem::Class(class) => {
                assert!(class.modifiers.contains(&"shared".to_string()));
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn test_abstract_class() {
        let source = r#"
            abstract class Base {
                void method();
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse abstract class: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_final_class() {
        let source = r#"
            final class Sealed {
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse final class: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_property_accessor() {
        let source = r#"
            class Test {
                private int _value;

                int value {
                    get const { return _value; }
                    set { _value = value; }
                }
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse property accessor: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_try_catch() {
        let source = r#"
            void test() {
                try {
                    riskyOperation();
                } catch {
                    print("error");
                }
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse try-catch: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_string_literals() {
        let source = r#"
            void test() {
                string s1 = "double quotes";
                string s2 = 'single quotes';
                string s3 = "escaped \"quotes\"";
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse string literals: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_number_literals() {
        let source = r#"
            void test() {
                int a = 42;
                float b = 3.14;
                int c = 0xFF;
                int d = 0b1010;
                int e = 0o755;
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse number literals: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_bool_literals() {
        let source = r#"
            void test() {
                bool t = true;
                bool f = false;
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse bool literals: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_null_literal() {
        let source = r#"
            void test() {
                CGameObj @obj = null;
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse null literal: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_scope_resolution() {
        let source = r#"
            void test() {
                ::GlobalFunction();
                Namespace::Function();
                Namespace::SubNamespace::Function();
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse scope resolution: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_conditional_compilation() {
        let source = r#"
            #if DEBUG
                void debugFunction() {}
            #endif

            #if FEATURE_A
                class FeatureA {}
            #elif FEATURE_B
                class FeatureB {}
            #else
                class Default {}
            #endif
        "#;

        let mut builder = ScriptBuilder::new();
        builder.define_word("DEBUG".to_string());
        builder.define_word("FEATURE_A".to_string());

        let result = parse_with_preprocessor(source, &mut builder);
        assert!(
            result.is_ok(),
            "Failed to parse conditional compilation: {:?}",
            result.err()
        );

        let script = result.unwrap();
        // Should have debugFunction and FeatureA, but not FeatureB or Default
        assert!(script.items.len() >= 2);
    }

    #[test]
    fn test_pragma() {
        let source = r#"
            #pragma optimize(speed)

            void test() {}
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse pragma: {:?}", result.err());
    }

    #[test]
    fn test_comments() {
        let source = r#"
            // Single line comment
            void test() {
                /* Multi-line
                   comment */
                int x = 42; // inline comment
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse with comments: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_complex_expressions() {
        let source = r#"
            void test() {
                int result = (a + b) * (c - d) / e % f;
                bool check = (x > 5 && y < 10) || (z == 0);
                int bits = (value << 2) | (mask & 0xFF);
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse complex expressions: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_nested_classes() {
        let source = r#"
            class Outer {
                class Inner {
                    void method() {}
                }
            }
        "#;

        let result = parse(source);
        // Note: AngelScript doesn't actually support nested classes,
        // but we should at least not crash
        let _ = result;
    }

    #[test]
    fn test_mixin() {
        let source = r#"
            mixin class MyMixin {
                void mixinMethod() {}
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse mixin: {:?}", result.err());
    }

    #[test]
    fn test_full_player_script() {
        let source = r#"
            class CPlayer : IController
            {
                CPlayer(CGameObj @obj)
                {
                    @self = obj;
                }

                void OnThink()
                {
                    int dx = 0, dy = 0;
                    if( game.actionState[UP] )
                        dy--;
                    if( game.actionState[DOWN] )
                        dy++;
                    if( game.actionState[LEFT] )
                        dx--;
                    if( game.actionState[RIGHT] )
                        dx++;
                    if( !self.Move(dx,dy) )
                    {
                        // Couldn't move
                    }
                }

                void OnMessage(ref @m, const CGameObj @sender)
                {
                    CMessage @msg = cast<CMessage>(m);
                    if( msg !is null && msg.txt == 'Attack' )
                    {
                        self.Kill();
                        game.EndGame(false);
                    }
                }

                CGameObj @self;
            }

            enum EAction
            {
                UP = 0,
                DOWN = 1,
                LEFT = 2,
                RIGHT = 3
            }
        "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse player script: {:?}",
            result.err()
        );

        let script = result.unwrap();
        assert_eq!(script.items.len(), 2); // Class and Enum
    }

    #[test]
    fn test_empty_script() {
        let source = "";
        let result = parse(source);
        assert!(result.is_ok());

        let script = result.unwrap();
        assert_eq!(script.items.len(), 0);
    }

    #[test]
    fn test_semicolon_only() {
        let source = ";;;";
        let result = parse(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parameter_type_modifiers() {
        let source = r#"
        void test1(int &in input) {
            // input is read-only reference
        }

        void test2(int &out output) {
            // output is write-only reference
            output = 42;
        }

        void test3(int &inout value) {
            // value is read-write reference
            value += 10;
        }

        void test4(int &value) {
            // default is inout
            value = 100;
        }
    "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse parameter type modifiers: {:?}",
            result.err()
        );

        let script = result.unwrap();
        assert_eq!(script.items.len(), 4);

        // Check that parameters have the correct type modifiers
        for item in &script.items {
            if let ScriptItem::Func(func) = item {
                assert_eq!(func.params.len(), 1);
                let param = &func.params[0];

                match func.name.as_str() {
                    "test1" => {
                        assert_eq!(param.type_mod, Some(TypeMod::In));
                    }
                    "test2" => {
                        assert_eq!(param.type_mod, Some(TypeMod::Out));
                    }
                    "test3" => {
                        assert_eq!(param.type_mod, Some(TypeMod::InOut));
                    }
                    "test4" => {
                        // & without explicit modifier defaults to inout
                        assert!(param.type_mod.is_some());
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn test_const_ref_parameters() {
        let source = r#"
        void test(const string &in text, const array<int> &in numbers) {
            print(text);
        }
    "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse const ref parameters: {:?}",
            result.err()
        );

        let script = result.unwrap();
        match &script.items[0] {
            ScriptItem::Func(func) => {
                assert_eq!(func.params.len(), 2);

                // Both parameters should have &in modifier
                assert_eq!(func.params[0].type_mod, Some(TypeMod::In));
                assert_eq!(func.params[1].type_mod, Some(TypeMod::In));

                // Both should be const
                assert!(func.params[0].param_type.is_const);
                assert!(func.params[1].param_type.is_const);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_handle_parameters() {
        let source = r#"
        void test1(CGameObj @obj) {
            // Handle parameter (not a reference)
        }

        void test2(CGameObj @&in obj) {
            // Handle reference in
        }

        void test3(CGameObj @&out obj) {
            // Handle reference out
        }
    "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse handle parameters: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_mixed_parameters() {
        let source = r#"
        void complexFunction(
            int value,
            const string &in text,
            array<int> &out results,
            CGameObj @handle,
            bool &inout flag = true
        ) {
            // Mix of value, const ref, out ref, handle, and inout with default
        }
    "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse mixed parameters: {:?}",
            result.err()
        );

        let script = result.unwrap();
        match &script.items[0] {
            ScriptItem::Func(func) => {
                assert_eq!(func.params.len(), 5);

                // Check each parameter
                assert_eq!(func.params[0].name, Some("value".to_string()));
                assert_eq!(func.params[0].type_mod, None); // value parameter, no modifier

                assert_eq!(func.params[1].name, Some("text".to_string()));
                assert_eq!(func.params[1].type_mod, Some(TypeMod::In));
                assert!(func.params[1].param_type.is_const);

                assert_eq!(func.params[2].name, Some("results".to_string()));
                assert_eq!(func.params[2].type_mod, Some(TypeMod::Out));

                assert_eq!(func.params[3].name, Some("handle".to_string()));
                // Handle parameters don't have type_mod

                assert_eq!(func.params[4].name, Some("flag".to_string()));
                assert_eq!(func.params[4].type_mod, Some(TypeMod::InOut));
                assert!(func.params[4].default_value.is_some());
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_constructor_with_ref_params() {
        let source = r#"
        class CMessage {
            CMessage(const string &in t) {
                txt = t;
            }
            string txt;
        }
    "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse constructor with ref params: {:?}",
            result.err()
        );

        let script = result.unwrap();
        match &script.items[0] {
            ScriptItem::Class(class) => {
                // Find the constructor
                let constructor = class.members.iter().find_map(|m| {
                    if let ClassMember::Func(f) = m {
                        if f.name == "CMessage" { Some(f) } else { None }
                    } else {
                        None
                    }
                });

                assert!(constructor.is_some(), "Constructor not found");
                let constructor = constructor.unwrap();

                assert_eq!(constructor.params.len(), 1);
                assert_eq!(constructor.params[0].type_mod, Some(TypeMod::In));
                assert!(constructor.params[0].param_type.is_const);
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn test_interface_method_with_ref() {
        let source = r#"
        interface IController {
            void OnMessage(ref @m, const CGameObj @sender);
        }
    "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse interface with ref: {:?}",
            result.err()
        );

        let script = result.unwrap();
        match &script.items[0] {
            ScriptItem::Interface(interface) => {
                assert_eq!(interface.members.len(), 1);

                match &interface.members[0] {
                    InterfaceMember::Method(method) => {
                        assert_eq!(method.params.len(), 2);
                        // First param is ref @m
                        // Second param is const CGameObj @sender
                    }
                    _ => panic!("Expected method"),
                }
            }
            _ => panic!("Expected interface"),
        }
    }

    #[test]
    fn test_funcdef_with_ref() {
        let source = r#"
        funcdef void Callback(const string &in message);
        funcdef void EventHandler(int &out result);
    "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse funcdef with ref: {:?}",
            result.err()
        );

        let script = result.unwrap();
        assert_eq!(script.items.len(), 2);

        match &script.items[0] {
            ScriptItem::FuncDef(funcdef) => {
                assert_eq!(funcdef.params.len(), 1);
                assert_eq!(funcdef.params[0].type_mod, Some(TypeMod::In));
            }
            _ => panic!("Expected funcdef"),
        }

        match &script.items[1] {
            ScriptItem::FuncDef(funcdef) => {
                assert_eq!(funcdef.params.len(), 1);
                assert_eq!(funcdef.params[0].type_mod, Some(TypeMod::Out));
            }
            _ => panic!("Expected funcdef"),
        }
    }

    #[test]
    fn test_ref_without_modifier_defaults_to_inout() {
        let source = r#"
        void swap(int &a, int &b) {
            int temp = a;
            a = b;
            b = temp;
        }
    "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse ref without modifier: {:?}",
            result.err()
        );

        let script = result.unwrap();
        match &script.items[0] {
            ScriptItem::Func(func) => {
                assert_eq!(func.params.len(), 2);

                // Both should have type_mod (defaults to In per the grammar)
                // In AngelScript, & without explicit modifier means inout
                assert!(func.params[0].type_mod.is_some());
                assert!(func.params[1].type_mod.is_some());
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_array_ref_parameter() {
        let source = r#"
        void processArray(array<int> &inout arr) {
            arr.insertLast(42);
        }
    "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse array ref parameter: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_template_type_ref_parameter() {
        let source = r#"
        void processMap(dictionary<string, int> &inout map) {
            map["key"] = 100;
        }
    "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse template type ref parameter: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_ref_defaults_to_inout() {
        let source = r#"
        // From docs: "If no keyword is used, the compiler assumes the inout modifier"
        void swap(Object &a, Object &b) {
            Object temp = a;
            a = b;
            b = temp;
        }
    "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        match &script.items[0] {
            ScriptItem::Func(func) => {
                assert_eq!(func.params.len(), 2);

                // Both should default to InOut (not In!)
                assert_eq!(func.params[0].type_mod, Some(TypeMod::InOut));
                assert_eq!(func.params[1].type_mod, Some(TypeMod::InOut));
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_inout_only_for_reference_types() {
        let source = r#"
        // From docs: "Only reference types, i.e. that can have handles to them,
        // are allowed to be passed as inout references"
        void modifyObject(Object &inout obj) {
            obj.DoSomething();
        }

        // Primitive types can use &out or &in
        void getPrimitive(int &out result) {
            result = 42;
        }
    "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_const_ref_in_pattern() {
        let source = r#"
        // From docs: "Especially for const &in the compiler is many times
        // able to avoid a copy of the value"
        void Function(const int &in a, int &out b, Object &c) {
            b = a;
            c.DoSomething();
        }
    "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        match &script.items[0] {
            ScriptItem::Func(func) => {
                assert_eq!(func.params.len(), 3);

                // a: const int &in
                assert_eq!(func.params[0].type_mod, Some(TypeMod::In));
                assert!(func.params[0].param_type.is_const);

                // b: int &out
                assert_eq!(func.params[1].type_mod, Some(TypeMod::Out));

                // c: Object & (defaults to inout)
                assert_eq!(func.params[2].type_mod, Some(TypeMod::InOut));
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_foreach_loop() {
        let source = r#"
        void test() {
            array<int> arr = {1,2,3,4,5};
            int sum = 0;
            foreach( auto value, auto index : arr ) {
                sum += value;
                arr[index] = -value;
            }
        }
    "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse foreach: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_super_constructor_call() {
        let source = r#"
        class Base {
            Base(int x) {}
        }

        class Derived : Base {
            Derived(int x) {
                super(x);
            }
        }
    "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse super(): {:?}",
            result.err()
        );
    }

    #[test]
    fn test_this_keyword() {
        let source = r#"
        class MyClass {
            int value;

            void setValue(int value) {
                this.value = value;
            }
        }
    "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse this: {:?}", result.err());
    }

    #[test]
    fn test_array_shorthand() {
        let source = r#"
        void test() {
            int[] arr;  // Same as array<int>
            string[] names = {"Alice", "Bob"};
        }
    "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse array shorthand: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_template_function_call() {
        let source = r#"
        void test() {
            auto result = cast<MyType>(obj);
            array<int> arr = array<int>();
        }
    "#;

        let result = parse(source);
        assert!(
            result.is_ok(),
            "Failed to parse template calls: {:?}",
            result.err()
        );
    }
}
