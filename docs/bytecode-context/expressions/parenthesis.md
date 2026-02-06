# Parentheses

## Overview
Parentheses group sub-expressions to override the default operator precedence. They do not produce any bytecode themselves but control the order in which sub-expressions are compiled and evaluated.

## Syntax
```angelscript
(expression)

// Override precedence
a * (b + c)

// Clarify complex logic
if ((a || b) && c) { }

// Conditional lvalue
(condition ? x : y) = value;
```

## Semantics
- **Grouping:** Parentheses force the enclosed expression to be evaluated as a unit before the surrounding operators apply. This overrides the default operator precedence.
- **No type change:** The type and value category (lvalue/rvalue) of the parenthesized expression are the same as the enclosed expression.
- **Nesting:** Parentheses can be arbitrarily nested.
- **Required contexts:** Some contexts require parentheses for clarity or correctness:
  - Conditional lvalue expressions: `(cond ? a : b) = val`
  - Complex boolean conditions in `if`/`while` where the default precedence would produce unintended grouping.

## Examples
```angelscript
// Without parentheses: * has higher precedence than +
int r1 = 2 + 3 * 4;       // 14 (3*4=12, then 2+12)

// With parentheses: + is evaluated first
int r2 = (2 + 3) * 4;     // 20 (2+3=5, then 5*4)

// Boolean grouping
bool a = true, b = false, c = true;
if (a || b && c) { }      // && binds tighter: a || (b && c) -> true
if ((a || b) && c) { }    // OR first: (true || false) && true -> true

// Nested
int x = ((a + b) * (c - d)) / e;

// Conditional lvalue requires parentheses
int p, q;
(flag ? p : q) = 10;
```

## Compilation Notes
- **Stack behavior:** Parentheses themselves generate no bytecode instructions. They only affect the order in which the compiler processes sub-expressions. The enclosed expression is compiled first, producing its value on the stack, and then the surrounding operator uses that stack value.
- **Type considerations:** No type changes or conversions are introduced by parentheses.
- **Control flow:** No branching introduced by parentheses themselves.
- **Special cases:**
  - Parentheses are purely a syntactic construct for the parser. The AST (abstract syntax tree) captures the grouping, and from that point forward, the parentheses have no distinct representation -- the tree structure encodes the evaluation order.
  - The compiler should ensure that redundant parentheses (e.g., `((x))`) do not affect performance or code generation.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Paren` | Parenthesized expression variant | Wraps `&ParenExpr` |
| `ParenExpr` | Parenthesized grouping | `expr: &Expr`, `span: Span` |

**Notes:**
- Unlike the doc's statement that "the AST captures the grouping, and from that point forward, the parentheses have no distinct representation," this parser **does** preserve parentheses as a distinct `Expr::Paren` node. This is useful for span tracking, diagnostics, and accurate source roundtripping.
- `ParenExpr` simply wraps the inner expression with no type or value modification.
- Redundant parentheses (`((x))`) produce nested `Expr::Paren` nodes.

## Related Features
- [math-operators.md](math-operators.md) - Arithmetic precedence that parentheses override
- [logic-operators.md](logic-operators.md) - Boolean precedence grouping
- [conditional-expression.md](conditional-expression.md) - Ternary operator often wrapped in parentheses
