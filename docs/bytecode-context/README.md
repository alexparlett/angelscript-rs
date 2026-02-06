# AngelScript Bytecode Context Files

Reference documentation for AngelScript language features, organized for compiler bytecode generation. Each file provides both a general language reference and compilation-specific notes.

## Statements
| File | Description |
|------|-------------|
| [variable-declarations.md](statements/variable-declarations.md) | Variable declaration, scoping, and initialization rules |
| [expression-statement.md](statements/expression-statement.md) | Standalone expressions as statements |
| [if-else.md](statements/if-else.md) | if/if-else/if-else-if conditional branching |
| [switch-case.md](statements/switch-case.md) | Switch-case for integer expressions |
| [while-loop.md](statements/while-loop.md) | While loop with pre-condition check |
| [do-while-loop.md](statements/do-while-loop.md) | Do-while loop with post-condition check |
| [for-loop.md](statements/for-loop.md) | For loop with init/condition/increment |
| [foreach-loop.md](statements/foreach-loop.md) | Foreach loop over container elements |
| [break-continue.md](statements/break-continue.md) | Loop/switch control flow interruption |
| [return-statement.md](statements/return-statement.md) | Function return with optional value |
| [statement-blocks.md](statements/statement-blocks.md) | Scoped statement blocks |
| [try-catch.md](statements/try-catch.md) | Exception handling blocks |

## Expressions
| File | Description |
|------|-------------|
| [assignments.md](expressions/assignments.md) | Assignment expressions and evaluation order |
| [function-calls.md](expressions/function-calls.md) | Function invocation, argument passing, named args |
| [math-operators.md](expressions/math-operators.md) | Arithmetic operators (+, -, *, /, %) |
| [bitwise-operators.md](expressions/bitwise-operators.md) | Bitwise operations (~, &, |, ^, shifts) |
| [compound-assignments.md](expressions/compound-assignments.md) | Compound assignment operators (+=, -=, etc.) |
| [logic-operators.md](expressions/logic-operators.md) | Logical AND, OR, NOT with short-circuit |
| [equality-comparison.md](expressions/equality-comparison.md) | Value equality (==, !=) |
| [relational-comparison.md](expressions/relational-comparison.md) | Ordered comparison (<, >, <=, >=) |
| [identity-comparison.md](expressions/identity-comparison.md) | Handle identity (is, !is) |
| [increment-operators.md](expressions/increment-operators.md) | Pre/post increment and decrement |
| [indexing-operator.md](expressions/indexing-operator.md) | Array/object indexing ([]) |
| [conditional-expression.md](expressions/conditional-expression.md) | Ternary conditional (?:) |
| [member-access.md](expressions/member-access.md) | Dot operator for member access |
| [handle-of.md](expressions/handle-of.md) | Handle-of operator (@) |
| [parenthesis.md](expressions/parenthesis.md) | Expression grouping |
| [scope-resolution.md](expressions/scope-resolution.md) | Namespace/class scope (::) |
| [type-conversions.md](expressions/type-conversions.md) | Explicit and implicit type conversions |
| [anonymous-objects.md](expressions/anonymous-objects.md) | Unnamed object construction |
| [initialization-lists.md](expressions/initialization-lists.md) | Initialization list syntax ({...}), list patterns, buffer layout |

## Types
| File | Description |
|------|-------------|
| [primitives.md](types/primitives.md) | Primitive types (bool, int, float, double variants) |
| [objects.md](types/objects.md) | Value types vs reference types, object lifecycle |
| [handles.md](types/handles.md) | Object handles (@), reference counting |
| [strings.md](types/strings.md) | String type, literals, operations |
| [arrays.md](types/arrays.md) | Array declaration, methods, multidimensional |
| [dictionary.md](types/dictionary.md) | Key-value dictionary type |
| [ref-weakref.md](types/ref-weakref.md) | Generic ref and weak references |
| [funcptr.md](types/funcptr.md) | Function pointers and delegates |
| [auto-declarations.md](types/auto-declarations.md) | Auto type inference |
| [operator-precedence.md](types/operator-precedence.md) | Full operator precedence table |

## Functions
| File | Description |
|------|-------------|
| [function-declarations.md](functions/function-declarations.md) | Function syntax, parameters, return types |
| [function-overloading.md](functions/function-overloading.md) | Overload resolution rules |
| [default-arguments.md](functions/default-arguments.md) | Default parameter values |
| [return-references.md](functions/return-references.md) | Returning references from functions |
| [anonymous-functions.md](functions/anonymous-functions.md) | Lambdas and closures |
| [function-references.md](functions/function-references.md) | Function handles and delegates |

## Classes
| File | Description |
|------|-------------|
| [class-declarations.md](classes/class-declarations.md) | Class declaration syntax and structure |
| [constructors.md](classes/constructors.md) | Constructor types and initialization |
| [destructors.md](classes/destructors.md) | Destructor semantics and ordering |
| [methods.md](classes/methods.md) | Instance methods, virtual dispatch |
| [properties.md](classes/properties.md) | Member variables and virtual properties |
| [access-modifiers.md](classes/access-modifiers.md) | private/protected access control |
| [inheritance.md](classes/inheritance.md) | Single inheritance, polymorphism |
| [member-initialization.md](classes/member-initialization.md) | Member initializer order and syntax |
| [operator-overloads.md](classes/operator-overloads.md) | Operator method overloading |
| [mixin-classes.md](classes/mixin-classes.md) | Mixin code injection |

## Globals
| File | Description |
|------|-------------|
| [global-variables.md](globals/global-variables.md) | Module-level variable declarations |
| [global-functions.md](globals/global-functions.md) | Module-level function declarations |
| [virtual-properties.md](globals/virtual-properties.md) | Global virtual properties |
| [enums.md](globals/enums.md) | Enumeration types and values |
| [typedefs.md](globals/typedefs.md) | Type aliases |
| [funcdefs.md](globals/funcdefs.md) | Function signature type definitions |
| [interfaces.md](globals/interfaces.md) | Interface declarations and implementation |
| [namespaces.md](globals/namespaces.md) | Namespace scoping |
| [imports.md](globals/imports.md) | Cross-module function imports |
| [shared-entities.md](globals/shared-entities.md) | Shared cross-module entities |

## Advanced
| File | Description |
|------|-------------|
| [coroutines.md](advanced/coroutines.md) | Coroutine suspend/resume semantics |
| [templates.md](advanced/templates.md) | Template type instantiation |
| [exceptions.md](advanced/exceptions.md) | Exception handling runtime |
