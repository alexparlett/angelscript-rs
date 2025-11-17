pub mod ast;
pub mod declaration_parser;
pub mod expr_parser;
pub mod lexer;
pub mod parser;
mod preprocessor;
pub mod script_builder;
pub mod token;

#[cfg(test)]
mod tests {
    use crate::core::error::ParseResult;
    use crate::parser::ast::*;
    use crate::parser::script_builder::{IncludeCallback, PragmaCallback, ScriptBuilder};
    use std::collections::HashMap;

    fn parse(source: &str) -> ParseResult<Script> {
        let mut builder = ScriptBuilder::new();
        builder.add_section("test", source);
        builder.build()
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
            ScriptNode::Func(func) => {
                assert_eq!(func.name, "main");
                assert!(func.return_type.is_some());
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_global_variables() {
        let source = r#"
            int globalVar = 42;
            const float PI = 3.14159;
            string message = "Hello";
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        assert_eq!(script.items.len(), 3);
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
            ScriptNode::Class(class) => {
                assert_eq!(class.name, "MyClass");
                assert_eq!(class.members.len(), 3);

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
                }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        match &script.items[0] {
            ScriptNode::Class(class) => {
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
            ScriptNode::Class(class) => {
                assert_eq!(class.name, "Derived");
                assert_eq!(class.extends.len(), 2);
                assert_eq!(class.extends[0], "Base");
                assert_eq!(class.extends[1], "IInterface");
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn test_shared_class() {
        let source = r#"
            shared class CMessage {
                string txt;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        match &script.items[0] {
            ScriptNode::Class(class) => {
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_final_class() {
        let source = r#"
            final class Sealed {
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_external_class_declaration() {
        let source = r#"
            external shared class ExternalClass;
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
            ScriptNode::Interface(i) => {
                assert_eq!(i.name, "IController");
                assert_eq!(i.members.len(), 2);
            }
            _ => panic!("Expected interface"),
        }
    }

    #[test]
    fn test_interface_inheritance() {
        let source = r#"
            interface IDerived : IBase1, IBase2 {
                void method();
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_shared_interface() {
        let source = r#"
            shared interface IShared {
                void method();
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_external_interface() {
        let source = r#"
            external shared interface IExternal;
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
            ScriptNode::Enum(e) => {
                assert_eq!(e.name, "Color");
                assert_eq!(e.variants.len(), 3);
                assert_eq!(e.variants[0].name, "RED");
            }
            _ => panic!("Expected enum"),
        }
    }

    #[test]
    fn test_enum_without_values() {
        let source = r#"
            enum Direction {
                UP,
                DOWN,
                LEFT,
                RIGHT
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_shared_enum() {
        let source = r#"
            shared enum EAction {
                UP = 0,
                DOWN = 1
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_typedef() {
        let source = r#"
            typedef float real32;
            typedef double real64;
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_funcdef() {
        let source = r#"
            funcdef void Callback(int value);
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_funcdef_with_return_reference() {
        let source = r#"
            funcdef string& GetStringRef();
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_shared_funcdef() {
        let source = r#"
            shared funcdef void EventHandler(int code);
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parameter_type_modifiers() {
        let source = r#"
            void test1(int &in input) {}
            void test2(int &out output) { output = 42; }
            void test3(int &inout value) { value += 10; }
            void test4(int &value) { value = 100; }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        assert_eq!(script.items.len(), 4);

        for item in &script.items {
            if let ScriptNode::Func(func) = item {
                assert_eq!(func.params.len(), 1);
                let param = &func.params[0];

                match func.name.as_str() {
                    "test1" => assert_eq!(param.type_mod, Some(TypeMod::In)),
                    "test2" => assert_eq!(param.type_mod, Some(TypeMod::Out)),
                    "test3" => assert_eq!(param.type_mod, Some(TypeMod::InOut)),
                    "test4" => assert_eq!(param.type_mod, Some(TypeMod::InOut)),
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        match &script.items[0] {
            ScriptNode::Func(func) => {
                assert_eq!(func.params.len(), 2);
                assert_eq!(func.params[0].type_mod, Some(TypeMod::In));
                assert_eq!(func.params[1].type_mod, Some(TypeMod::In));
                assert!(func.params[0].param_type.is_const);
                assert!(func.params[1].param_type.is_const);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_default_parameters() {
        let source = r#"
            void test(int x = 10, string s = "hello") {
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
            ) {}
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        match &script.items[0] {
            ScriptNode::Func(func) => {
                assert_eq!(func.params.len(), 5);
                assert_eq!(func.params[0].type_mod, None);
                assert_eq!(func.params[1].type_mod, Some(TypeMod::In));
                assert_eq!(func.params[2].type_mod, Some(TypeMod::Out));
                assert_eq!(func.params[4].type_mod, Some(TypeMod::InOut));
                assert!(func.params[4].default_value.is_some());
            }
            _ => panic!("Expected function"),
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_const_handle() {
        let source = r#"
            void test(CGameObj @const handle) {
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_handle_assignment() {
        let source = r#"
        void test() {
            CGameObj@ obj1, obj2;
            @obj1 = @obj2;
        }
    "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_property_accessor_in_interface() {
        let source = r#"
            interface ITest {
                int value {
                    get const;
                    set;
                }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_property_with_only_getter() {
        let source = r#"
            class Test {
                int readonly {
                    get const { return 42; }
                }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_is_operator() {
        let source = r#"
            void test() {
                if (msg !is null && msg is CMessage) {
                }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_ternary_operator() {
        let source = r#"
            void test() {
                int x = condition ? 1 : 2;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_logical_operators() {
        let source = r#"
            void test() {
                bool a = true and false;
                bool b = true or false;
                bool c = true xor false;
                bool d = not true;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_power_operator() {
        let source = r#"
            void test() {
                int result = 2 ** 8;
                result **= 2;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_if_statement() {
        let source = r#"
            void test() {
                if (x > 0) {
                    print("positive");
                } else if (x < 0) {
                    print("negative");
                } else {
                    print("zero");
                }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_return_statement() {
        let source = r#"
            int getValue() {
                return 42;
            }

            void noReturn() {
                return;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_break_continue() {
        let source = r#"
            void test() {
                for (int i = 0; i < 10; i++) {
                    if (i == 5) break;
                    if (i % 2 == 0) continue;
                    print(i);
                }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_function_call_with_multiple_args() {
        let source = r#"
        void test() {
            function(value1, value2);
        }
    "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_cast_syntax() {
        let source = r#"
            void test() {
                CMessage @msg = cast<CMessage>(m);
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_lambda_without_parameters() {
        let source = r#"
            void test() {
                auto callback = function() {
                    print("called");
                };
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_lambda_with_typed_parameters() {
        let source = r#"
            void test() {
                auto callback = function(int x, string s) {
                    print(s + x);
                };
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_template_types() {
        let source = r#"
            void test() {
                array<int> numbers;
                dictionary<string, int> map;
                dictionary<string, array<dictionary<string, array<array<int>>>>> complexNest;
                const_weakref<CGameObj> playerRef;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_nested_template_types() {
        let source = r#"
            void test() {
                array<array<int>> matrix;
                dictionary<string, array<int>> data;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_template_constructor_call() {
        let source = r#"
            void test() {
                array<int> arr = array<int>();
                dictionary<string, int> dict = dictionary<string, int>();
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        match &script.items[0] {
            ScriptNode::Namespace(ns) => {
                assert_eq!(ns.name[0], "MyNamespace");
                assert_eq!(ns.items.len(), 2);
            }
            _ => panic!("Expected namespace"),
        }
    }

    #[test]
    fn test_nested_namespace() {
        let source = r#"
            namespace Outer::Inner {
                void function() {}
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
                double f = 1.5e10;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_null_literal() {
        let source = r#"
            void test() {
                CGameObj @obj = null;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_mixin() {
        let source = r#"
            mixin class MyMixin {
                void mixinMethod() {}
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_import() {
        let source = r#"
            import void ExternalFunction(int x) from "module";
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_conditional_compilation_if() {
        let source = r#"
            #if DEBUG
                void debugFunction() {}
            #endif
        "#;

        let mut builder = ScriptBuilder::new();
        builder.define_word("DEBUG".to_string());

        builder.add_section("test", source);
        let result = builder.build();
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        assert_eq!(script.items.len(), 1);
    }

    #[test]
    fn test_conditional_compilation_if_false() {
        let source = r#"
            #if UNDEFINED
                void shouldNotAppear() {}
            #endif
        "#;

        let mut builder = ScriptBuilder::new();
        builder.add_section("test", source);
        let result = builder.build();
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        assert_eq!(script.items.len(), 0);
    }

    #[test]
    fn test_conditional_compilation_elif() {
        let source = r#"
            #if FEATURE_A
                class FeatureA {}
            #elif FEATURE_B
                class FeatureB {}
            #else
                class Default {}
            #endif
        "#;

        let mut builder = ScriptBuilder::new();
        builder.define_word("FEATURE_B".to_string());

        builder.add_section("test", source);
        let result = builder.build();
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        assert_eq!(script.items.len(), 1);
        match &script.items[0] {
            ScriptNode::Class(class) => {
                assert_eq!(class.name, "FeatureB");
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn test_conditional_compilation_else() {
        let source = r#"
            #if UNDEFINED
                class NotThis {}
            #else
                class ThisOne {}
            #endif
        "#;

        let mut builder = ScriptBuilder::new();

        builder.add_section("test", source);
        let result = builder.build();
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        assert_eq!(script.items.len(), 1);
        match &script.items[0] {
            ScriptNode::Class(class) => {
                assert_eq!(class.name, "ThisOne");
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn test_nested_conditionals() {
        let source = r#"
            #if OUTER
                #if INNER
                    void both() {}
                #else
                    void onlyOuter() {}
                #endif
            #endif
        "#;

        let mut builder = ScriptBuilder::new();
        builder.define_word("OUTER".to_string());
        builder.define_word("INNER".to_string());

        builder.add_section("test", source);
        let result = builder.build();
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        assert_eq!(script.items.len(), 1);
    }

    struct TestIncludeCallback {
        files: HashMap<String, String>,
    }

    impl TestIncludeCallback {
        fn new() -> Self {
            let mut files = HashMap::new();
            files.insert(
                "common.as".to_string(),
                "void commonFunction() {}".to_string(),
            );
            files.insert(
                "types.as".to_string(),
                "class CommonType { int value; }".to_string(),
            );
            Self { files }
        }
    }

    impl IncludeCallback for TestIncludeCallback {
        fn on_include(&mut self, include_path: &str, _from_source: &str) -> ParseResult<String> {
            self.files.get(include_path).cloned().ok_or_else(|| {
                crate::core::error::ParseError::SyntaxError {
                    span: None,
                    message: format!("File not found: {}", include_path),
                }
            })
        }
    }

    #[test]
    fn test_include_directive() {
        let source = r#"
            #include "common.as"

            void main() {
                commonFunction();
            }
        "#;

        let mut builder = ScriptBuilder::new();
        builder.set_include_callback(TestIncludeCallback::new());

        builder.add_section("test", source);
        let result = builder.build();
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        assert_eq!(script.items.len(), 2);
    }

    #[test]
    fn test_multiple_includes() {
        let source = r#"
            #include "common.as"
            #include "types.as"

            void main() {}
        "#;

        let mut builder = ScriptBuilder::new();
        builder.set_include_callback(TestIncludeCallback::new());

        builder.add_section("test", source);
        let result = builder.build();
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        assert_eq!(script.items.len(), 3);
    }

    struct TestPragmaCallback {
        pragmas: Vec<String>,
    }

    impl TestPragmaCallback {
        fn new() -> Self {
            Self {
                pragmas: Vec::new(),
            }
        }
    }

    impl PragmaCallback for TestPragmaCallback {
        fn on_pragma(&mut self, pragma_text: &str) -> ParseResult<()> {
            self.pragmas.push(pragma_text.to_string());
            Ok(())
        }
    }

    #[test]
    fn test_pragma() {
        let source = r#"
            #pragma optimize(speed)

            void test() {}
        "#;

        let mut builder = ScriptBuilder::new();
        builder.set_pragma_callback(TestPragmaCallback::new());

        builder.add_section("test", source);
        let result = builder.build();
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_complete_game_script() {
        let source = r#"
            #if DEBUG
                void DebugLog(const string &in msg) {
                    print("[DEBUG] " + msg);
                }
            #endif

            shared interface IEntity {
                void Update(float deltaTime);
                void Render();
            }
        "#;

        let mut builder = ScriptBuilder::new();
        builder.define_word("DEBUG".to_string());

        builder.add_section("test", source);
        let result = builder.build();
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_custom_directive() {
        let source = r#"
            #custom_directive some content here

            void test() {}
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        assert_eq!(script.items.len(), 2);
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
    fn test_empty_class() {
        let source = "class Empty {}";
        let result = parse(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_namespace() {
        let source = "namespace Empty {}";
        let result = parse(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_void_expression() {
        let source = r#"
            void test() {
                void;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_foreach_simple() {
        let source = r#"
        void main() {
            foreach(int val : arr) {
                print(val);
            }
        }
    "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        match &script.items[0] {
            ScriptNode::Func(func) => {
                assert_eq!(func.name, "main");
                if let Some(body) = &func.body {
                    match &body.statements[0] {
                        Statement::ForEach(foreach_stmt) => {
                            assert_eq!(foreach_stmt.variables.len(), 1);
                            assert_eq!(foreach_stmt.variables[0].1, "val");
                            match &foreach_stmt.variables[0].0.datatype {
                                DataType::PrimType(t) => assert_eq!(t, "int"),
                                _ => panic!("Expected int type"),
                            }
                        }
                        _ => panic!("Expected ForEach statement"),
                    }
                }
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_foreach_with_index() {
        let source = r#"
        void main() {
            foreach(auto val, auto idx : arr) {
                print(idx);
            }
        }
    "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        match &script.items[0] {
            ScriptNode::Func(func) => {
                if let Some(body) = &func.body {
                    match &body.statements[0] {
                        Statement::ForEach(foreach_stmt) => {
                            assert_eq!(foreach_stmt.variables.len(), 2);
                            assert_eq!(foreach_stmt.variables[0].1, "val");
                            assert_eq!(foreach_stmt.variables[1].1, "idx");
                            match &foreach_stmt.variables[0].0.datatype {
                                DataType::Auto => {}
                                _ => panic!("Expected auto type"),
                            }
                        }
                        _ => panic!("Expected ForEach statement"),
                    }
                }
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_foreach_auto() {
        let source = r#"
        void main() {
            foreach(auto x : container) {
                x++;
            }
        }
    "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        match &script.items[0] {
            ScriptNode::Func(func) => {
                if let Some(body) = &func.body {
                    match &body.statements[0] {
                        Statement::ForEach(foreach_stmt) => {
                            assert_eq!(foreach_stmt.variables.len(), 1);
                            assert_eq!(foreach_stmt.variables[0].1, "x");
                            match &foreach_stmt.variables[0].0.datatype {
                                DataType::Auto => {}
                                _ => panic!("Expected Auto type"),
                            }
                        }
                        _ => panic!("Expected ForEach statement"),
                    }
                }
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_foreach_multiple_types() {
        let source = r#"
        void main() {
            foreach(string val, uint idx : arr) {
            }
        }
    "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        match &script.items[0] {
            ScriptNode::Func(func) => {
                if let Some(body) = &func.body {
                    match &body.statements[0] {
                        Statement::ForEach(foreach_stmt) => {
                            assert_eq!(foreach_stmt.variables.len(), 2);
                            assert_eq!(foreach_stmt.variables[0].1, "val");
                            assert_eq!(foreach_stmt.variables[1].1, "idx");
                            match &foreach_stmt.variables[0].0.datatype {
                                DataType::Identifier(name) => assert_eq!(name, "string"),
                                _ => panic!("Expected string type"),
                            }
                            match &foreach_stmt.variables[1].0.datatype {
                                DataType::PrimType(t) => assert_eq!(t, "uint"),
                                _ => panic!("Expected uint type"),
                            }
                        }
                        _ => panic!("Expected ForEach statement"),
                    }
                }
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_foreach_nested() {
        let source = r#"
        void main() {
            foreach(auto row : matrix) {
                foreach(auto val : row) {
                    print(val);
                }
            }
        }
    "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let script = result.unwrap();
        match &script.items[0] {
            ScriptNode::Func(func) => {
                if let Some(body) = &func.body {
                    match &body.statements[0] {
                        Statement::ForEach(outer_foreach) => {
                            assert_eq!(outer_foreach.variables[0].1, "row");
                            match outer_foreach.body.as_ref() {
                                Statement::Block(block) => match &block.statements[0] {
                                    Statement::ForEach(inner_foreach) => {
                                        assert_eq!(inner_foreach.variables[0].1, "val");
                                    }
                                    _ => panic!("Expected nested ForEach statement"),
                                },
                                _ => panic!("Expected block statement"),
                            }
                        }
                        _ => panic!("Expected ForEach statement"),
                    }
                }
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_number_separator_decimal() {
        let source = r#"
            void test() {
                int million = 1'000'000;
                int billion = 1'000'000'000;
                int mixed = 12'34'567;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_number_separator_hex() {
        let source = r#"
            void test() {
                int hex1 = 0xFF'FF'FF'FF;
                int hex2 = 0x00'11'22'33;
                int hex3 = 0xDEAD'BEEF;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_number_separator_binary() {
        let source = r#"
            void test() {
                int bin1 = 0b1111'0000'1010'0101;
                int bin2 = 0b1'0'1'0;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_number_separator_octal() {
        let source = r#"
            void test() {
                int oct1 = 0o755'644;
                int oct2 = 0o1'2'3'4;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_number_separator_float() {
        let source = r#"
            void test() {
                float f1 = 1'234.567'890;
                float f2 = 3'141'592.653'589;
                double d = 1'000.0e10;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_heredoc_simple() {
        let source = r####"
            void test() {
                string sql = """SELECT * FROM users WHERE id = 1""";
            }
        "####;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_heredoc_multiline() {
        let source = r####"
            void test() {
                string query = """
                    SELECT u.name, u.email
                    FROM users u
                    WHERE u.active = 1
                    ORDER BY u.created_at DESC
                """;
            }
        "####;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_heredoc_with_quotes() {
        let source = r####"
            void test() {
                string html = """
                    <div class="container">
                        <p>Hello "World"</p>
                        <a href="/test">Link</a>
                    </div>
                """;
            }
        "####;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_bit_formats_lowercase() {
        let source = r#"
            void test() {
                int b1 = 0b1010;
                int o1 = 0o755;
                int d1 = 0d123;
                int h1 = 0xff;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_bit_formats_uppercase() {
        let source = r#"
            void test() {
                int b2 = 0B1111;
                int o2 = 0O644;
                int d2 = 0D999;
                int h2 = 0XFF;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_bit_formats_mixed_case() {
        let source = r#"
            void test() {
                int h1 = 0xAbCdEf;
                int h2 = 0XFEDCBA;
                int h3 = 0xDeAdBeEf;
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_escape_sequences_basic() {
        let source = r#"
            void test() {
                string newline = "Line1\nLine2";
                string tab = "Col1\tCol2";
                string carriage = "Return\rHere";
                string backslash = "Path\\to\\file";
                string quote = "Say \"Hello\"";
                string singleQuote = 'It\'s working';
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_escape_sequences_extended() {
        let source = r#"
            void test() {
                string nullEscape = "Null\0char";
                string bell = "Alert\a";
                string backspace = "Back\bspace";
                string formfeed = "Form\ffeed";
                string vtab = "Vertical\vtab";
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_escape_sequences_hex() {
        let source = r#"
            void test() {
                string hex1 = "Hex\x41"; // 'A'
                string hex2 = "Hex\x0A"; // newline
                string hex3 = "Hex\xFF"; // max byte
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_escape_sequences_unicode() {
        let source = r#"
            void test() {
                string unicode1 = "Unicode\u0041"; // 'A'
                string unicode2 = "Unicode\u263A"; // smiley ☺
                string unicode3 = "Unicode\u00A9"; // copyright ©
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_simple_init_list() {
        let source = r#"
            void test() {
                int[] arr = {1, 2, 3, 4, 5};
                array<int> nums = {10, 20, 30};
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_nested_init_list_2d() {
        let source = r#"
            void test() {
                int[][] matrix = {
                    {1, 2, 3},
                    {4, 5, 6},
                    {7, 8, 9}
                };
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_nested_init_list_complex() {
        let source = r#"
            void test() {
                array<array<int>> data = {
                    {1, 2},
                    {3, 4, 5},
                    {6}
                };
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_nested_init_list_mixed() {
        let source = r#"
            void test() {
                auto complex = {
                    {1, 2, 3},
                    {4, 5},
                    {6, 7, 8, 9}
                };

                auto strings = {
                    {"hello", "world"},
                    {"foo", "bar", "baz"}
                };
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_combined_features() {
        let source = r####"
            void test() {
                // Number separators in various formats
                int count = 1'000'000;
                int flags = 0xFF'00'AA'BB;
                float precise = 3'141.592'653;

                // Heredoc with embedded content
                string template = """
                    SELECT * FROM users
                    WHERE id IN (1, 2, 3)
                    AND status = 'active'
                """;

                // Escaped characters
                string path = "C:\\Users\\Name\\Documents";
                string json = "{\"key\": \"value\"}";
                string unicode = "Copyright \u00A9 2024";

                // Nested init lists
                int[][] grid = {
                    {1, 2, 3},
                    {4, 5, 6}
                };

                configure(1'920, 1'080, true);
            }

            void configure(int width = 800, int height = 600, bool fullscreen = false) {}
        "####;

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_reserved_as_variable() {
        assert!(parse("void f() { int null = 5; }").is_err());
        assert!(parse("void f() { int void = 5; }").is_err());
        assert!(parse("void f() { int class = 5; }").is_err());
        assert!(parse("void f() { int if = 5; }").is_err());
        assert!(parse("void f() { int for = 5; }").is_err());
        assert!(parse("void f() { int while = 5; }").is_err());
        assert!(parse("void f() { bool true = false; }").is_err());
        assert!(parse("void f() { bool false = true; }").is_err());
    }

    #[test]
    fn test_reserved_as_function() {
        assert!(parse("void null() {}").is_err());
        assert!(parse("void class() {}").is_err());
        assert!(parse("void if() {}").is_err());
        assert!(parse("int return() { return 0; }").is_err());
    }

    #[test]
    fn test_reserved_as_parameter() {
        assert!(parse("void f(int null) {}").is_err());
        assert!(parse("void f(int void) {}").is_err());
        assert!(parse("void f(bool true) {}").is_err());
    }

    #[test]
    fn test_reserved_as_class() {
        assert!(parse("class null {}").is_err());
        assert!(parse("class void {}").is_err());
        assert!(parse("class if {}").is_err());
        assert!(parse("class for {}").is_err());
    }

    #[test]
    fn test_reserved_as_interface() {
        assert!(parse("interface null {}").is_err());
        assert!(parse("interface void {}").is_err());
    }

    #[test]
    fn test_reserved_as_enum() {
        assert!(parse("enum null { A, B }").is_err());
        assert!(parse("enum void { A, B }").is_err());
        assert!(parse("enum MyEnum { null, B }").is_err());
        assert!(parse("enum MyEnum { A, void }").is_err());
    }

    #[test]
    fn test_reserved_as_namespace() {
        assert!(parse("namespace null {}").is_err());
        assert!(parse("namespace void {}").is_err());
    }

    #[test]
    fn test_reserved_as_typedef() {
        assert!(parse("typedef int null;").is_err());
        assert!(parse("typedef float void;").is_err());
    }

    #[test]
    fn test_reserved_as_funcdef() {
        assert!(parse("funcdef void null();").is_err());
        assert!(parse("funcdef void class();").is_err());
    }

    #[test]
    fn test_reserved_as_property() {
        assert!(parse("class C { int null { get { return 0; } } }").is_err());
    }

    #[test]
    fn test_contextual_keywords_allowed() {
        assert!(parse("void f() { int shared = 5; }").is_ok());
        assert!(parse("void f() { int override = 5; }").is_ok());
        assert!(parse("void f() { int property = 5; }").is_ok());
        assert!(parse("void f() { int final = 5; }").is_ok());
        assert!(parse("void f() { int abstract = 5; }").is_ok());
        assert!(parse("class shared {}").is_ok());
        assert!(parse("void override() {}").is_ok());
    }

    #[test]
    fn test_valid_similar_identifiers() {
        assert!(parse("void f() { int nullable = 5; }").is_ok());
        assert!(parse("void f() { int my_null = 5; }").is_ok());
        assert!(parse("void f() { int null_ = 5; }").is_ok());
        assert!(parse("void f() { int _void = 5; }").is_ok());
        assert!(parse("class MyClass {}").is_ok());
        assert!(parse("void myFunction() {}").is_ok());
    }

    #[test]
    fn test_all_reserved_keywords() {
        let keywords = vec![
            "class", "enum", "interface", "namespace", "typedef", "funcdef",
            "import", "mixin", "void", "int", "int8", "int16", "int32", "int64",
            "uint", "uint8", "uint16", "uint32", "uint64", "float", "double",
            "bool", "auto", "const", "private", "protected", "if", "else",
            "for", "foreach", "while", "do", "switch", "case", "default",
            "break", "continue", "return", "try", "catch", "in", "out",
            "inout", "cast", "true", "false", "null", "and", "or", "xor",
            "not", "is",
        ];

        for keyword in keywords {
            let source = format!("void f() {{ int {} = 5; }}", keyword);
            assert!(
                parse(&source).is_err(),
                "Keyword '{}' should be rejected as variable name",
                keyword
            );
        }
    }
}
