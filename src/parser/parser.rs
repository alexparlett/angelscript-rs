use pest::Parser;
use pest_derive::Parser;
use crate::parser::ast::Script;
use crate::parser::ast_builder;

#[derive(Parser)]
#[grammar = "parser/angelscript.pest"]
pub struct AngelScriptParser;

impl AngelScriptParser {
    pub fn from_source(source: &str) -> Result<Script, pest::error::Error<Rule>> {
        let pairs = AngelScriptParser::parse(Rule::SCRIPT, source)?;

        let mut items = Vec::new();
        for pair in pairs {
            if pair.as_rule() == Rule::EOI {
                break;
            }
            if let Some(item) = ast_builder::build_script_item(pair) {
                items.push(item);
            }
        }

        Ok(Script { items })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;

    #[test]
    fn test_simple_function() {
        let source = r#"
            void main() {
                int x = 42;
            }
        "#;

        let pairs = AngelScriptParser::parse(Rule::SCRIPT, source);
        assert!(pairs.is_ok(), "Failed to parse: {:?}", pairs.err());
    }

    #[test]
    fn test_class() {
        let source = r#"
            class MyClass {
                int value;
                void setValue(int v) {
                    value = v;
                }
            }
        "#;

        let pairs = AngelScriptParser::parse(Rule::SCRIPT, source);
        assert!(pairs.is_ok(), "Failed to parse: {:?}", pairs.err());
    }

    #[test]
    fn test_expression() {
        let source = "x + y * 2";

        let pairs = AngelScriptParser::parse(Rule::EXPR, source);
        assert!(pairs.is_ok(), "Failed to parse: {:?}", pairs.err());
    }

    #[test]
    fn test_unknown_types() {
        let source = r#"
            // All these types are "unknown" to the compiler, but should parse fine
            CustomType myVar;
            SomeNamespace::SomeClass obj;
            dictionary<string, MyCustomType> map;
            array<UnknownType@> handles;

            void useCustomTypes(CustomType1 arg1, CustomType2 @arg2) {
                CustomType3 local = CustomType3();
                AnotherType::StaticMethod();
            }
        "#;

        let pairs = AngelScriptParser::parse(Rule::SCRIPT, source);
        assert!(pairs.is_ok(), "Failed to parse unknown types: {:?}", pairs.err());
    }

    #[test]
    fn test_ref_as_type() {
        let source = r#"
            // ref is a type, not a keyword
            ref<MyType> myRef;
            ref @genericRef;
            const ref<int> constRef;

            void OnMessage(ref @m, const CGameObj @sender) {
                // ref is used as a type here
            }

            // Other addon types should work the same way
            array<int> myArray;
            dictionary<string, int> myDict;
            weakref<Player> weakPlayer;
        "#;

        let pairs = AngelScriptParser::parse(Rule::SCRIPT, source);
        assert!(pairs.is_ok(), "Failed to parse ref as type: {:?}", pairs.err());
    }

    #[test]
    fn test_player_script() {
        let source = r#"
        // Include the shared code
        #include 'shared.as'

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

        let pairs = AngelScriptParser::parse(Rule::SCRIPT, source);
        assert!(pairs.is_ok(), "Failed to parse: {:?}", pairs.err());
    }
}
