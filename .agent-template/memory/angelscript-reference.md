# AngelScript Reference

Full documentation: `docs/angelscript-lang/00-overview.md`

## Quick Lookup
- Primitives: 01-primitives.md
- Handles/Objects: 02-objects-handles.md
- Statements: 03-statements.md
- Expressions: 04-expressions.md
- Operators: 05-operators.md, 06-operator-overloads.md
- Classes: 07-classes.md
- Functions: 08-functions.md
- Type conversions: 09-type-conversions.md
- Globals (enums, interfaces, namespaces): 10-globals.md
- Advanced types (strings, arrays, lambdas): 11-datatypes-advanced.md
- Shared entities: 12-shared.md
- C++ specifics: cpp-*.md files

## Key Syntax
- Handle: `obj@ handle`, `@handle = @obj`
- Identity: `a is b`, `a !is null`
- Ref params: `&in`, `&out`, `&inout`
- Scope: `Namespace::item`, `::global`
- Shared: `shared class Foo {}`
- External shared: `external shared class Foo;` (reference existing from another module)

## Operator Methods
- Assignment: `opAssign`
- Comparison: `opEquals`, `opCmp`
- Binary: `opAdd`, `opSub`, `opMul`, etc. (and `_r` variants for reversed)
- Unary: `opNeg`, `opCom`, `opPreInc`, `opPostInc`
- Index: `opIndex` or `get_opIndex`/`set_opIndex`
- Conversion: `opConv`, `opImplConv`, `opCast`, `opImplCast`

## Type Registration Behaviors
- `asBEHAVE_FACTORY` - Create reference type instance
- `asBEHAVE_CONSTRUCT` - Construct value type in-place
- `asBEHAVE_DESTRUCT` - Destroy value type
- `asBEHAVE_ADDREF` - Increment reference count
- `asBEHAVE_RELEASE` - Decrement/destroy on zero
- `asBEHAVE_LIST_FACTORY` - Create from initializer list

## Memory Management
- Reference counting is primary mechanism
- Garbage collection is backup for circular references
- GC is incremental (runs in small steps)
