# Enhanced Switch Statement Support

## Current Behavior

Switch statements only support `int` and `enum` types as the switch expression:

```angelscript
switch (intValue) {      // OK
    case 1: break;
}

switch (enumValue) {     // OK
    case MyEnum::A: break;
}

switch (floatValue) {    // ERROR: "switch expression must be integer or enum type"
    case 1.0: break;
}

switch (stringValue) {   // ERROR: "switch expression must be integer or enum type"
    case "foo": break;
}
```

## Proposed Enhancement

Extend switch to support additional types by compiling them to equivalent if-else chains.

### Float Switch

```angelscript
switch (floatValue) {
    case 0.0: doZero(); break;
    case 1.0: doOne(); break;
    default: doDefault(); break;
}
```

Compiles to:
```angelscript
if (floatValue == 0.0) {
    doZero();
} else if (floatValue == 1.0) {
    doOne();
} else {
    doDefault();
}
```

**Considerations:**
- Float equality comparison has precision issues
- Should we use approximate comparison (epsilon)?
- Original AngelScript doesn't support float switch
- May want to emit a warning about float comparison

### String Switch

```angelscript
switch (stringValue) {
    case "hello": doHello(); break;
    case "world": doWorld(); break;
    default: doDefault(); break;
}
```

Compiles to:
```angelscript
if (stringValue == "hello") {
    doHello();
} else if (stringValue == "world") {
    doWorld();
} else {
    doDefault();
}
```

**Considerations:**
- String comparison via `opEquals`
- Case sensitivity (should match normal string comparison)
- Performance: linear search vs hash table for many cases

## Implementation Approach

### Option 1: Compile to If-Else (Recommended)

In `function_processor.rs`, when checking a switch statement:
1. Check if expression type is int/enum → use existing switch bytecode
2. Check if expression type is float/string → compile to if-else chain
3. Otherwise → error

**Pros:**
- Simple implementation
- No new bytecode needed
- Semantically clear

**Cons:**
- O(n) performance for many cases
- Different codegen path

### Option 2: Hash-Based String Switch

For string switches with many cases, generate a hash table lookup:
1. Compute hash of switch expression at runtime
2. Jump to case based on hash
3. Verify string equality (handle collisions)

**Pros:**
- O(1) average case for many branches
- Matches what optimizing compilers do

**Cons:**
- Complex implementation
- Only beneficial for many cases (>5-10)

### Option 3: Reject Float/String Switch

Keep current behavior - only int/enum allowed.

**Pros:**
- Simple, no changes needed
- Avoids float comparison issues
- Matches original AngelScript

**Cons:**
- Less convenient for users

## Recommendation

Start with **Option 1** (compile to if-else) for both float and string. This is:
- Easy to implement
- Semantically clear
- Can be optimized later if needed

For float switches, consider emitting a warning about float equality comparison.

## Files to Modify

- `src/semantic/passes/function_processor.rs` - `check_switch` method
- Add helper to generate if-else chain from switch cases
- May need to handle `break` statements differently in if-else context

## Test Cases

```angelscript
// Float switch
void testFloatSwitch(float f) {
    switch (f) {
        case 0.0: print("zero"); break;
        case 1.0: print("one"); break;
        case 2.5: print("two point five"); break;
        default: print("other"); break;
    }
}

// String switch
void testStringSwitch(const string &in s) {
    switch (s) {
        case "red": return 0xFF0000;
        case "green": return 0x00FF00;
        case "blue": return 0x0000FF;
        default: return 0x000000;
    }
}

// Edge cases
void testEdgeCases() {
    // Empty switch
    switch ("") { default: break; }

    // Single case
    switch (1.0) { case 1.0: break; }

    // Fall-through (if supported)
    switch ("a") {
        case "a":
        case "b":
            print("a or b");
            break;
    }
}
```

## Related

- Current switch implementation: `src/semantic/passes/function_processor.rs`
- Switch AST: `src/ast/stmt.rs` - `SwitchStmt`
