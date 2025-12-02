# Virtual Machine Design

> **Status**: INITIAL DRAFT - Design decisions pending

## Overview

This document covers the VM execution and memory management.

## Execution Model

**Decision Pending**: Pure stack-based (current bytecode design) vs hybrid with registers

Current bytecode is stack-based:
- All operations push/pop from value stack
- No explicit registers
- Simpler but potentially less efficient than C++ AngelScript's hybrid approach

## Instruction Set Status

Our bytecode uses generic instructions (single `Add` vs type-specific `ADDi`, `ADDf`).

**Trade-off:**
- Generic: Simpler bytecode, VM must track types at runtime
- Type-specific: Larger instruction set, faster dispatch

**Current decision**: Stay generic for simplicity, optimize later if needed.

---

## Full Instruction Comparison: Rust vs C++ AngelScript

### C++ AngelScript Instructions (201 total + 6 temporary)

#### Stack Operations
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 0 | `PopPtr` | Pop pointer from stack | `Pop` | ✅ Have (generic) |
| 1 | `PshGPtr` | Push global pointer | `LoadGlobal` | ✅ Have (different approach) |
| 2 | `PshC4` | Push 4-byte constant | `PushInt` | ✅ Have |
| 3 | `PshV4` | Push 4-byte variable | `LoadLocal` | ✅ Have |
| 4 | `PSF` | Push stack frame | - | ❌ Missing |
| 5 | `SwapPtr` | Swap pointers | `Swap` | ✅ Have |
| 7 | `PshG4` | Push 4-byte global | `LoadGlobal` | ✅ Have |
| 8 | `LdGRdR4` | Load global, read to register | - | ❌ Missing (register-based) |
| 47 | `PshC8` | Push 8-byte constant | `PushDouble`/`PushInt` | ✅ Have |
| 48 | `PshVPtr` | Push variable pointer | - | ❌ Missing (pointer semantics) |
| 49 | `RDSPtr` | Read stack pointer | - | ❌ Missing |
| 73 | `PshNull` | Push null | `PushNull` | ✅ Have |
| 179 | `PshV8` | Push 8-byte variable | `LoadLocal` | ✅ Have |

#### Function Calls
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 9 | `CALL` | Call script function | `Call` | ✅ Have |
| 10 | `RET` | Return | `Return`/`ReturnVoid` | ✅ Have |
| 61 | `CALLSYS` | Call system function | - | ❌ Missing (FFI) |
| 62 | `CALLBND` | Call bound function | - | ❌ Missing |
| 139 | `CALLINTF` | Call interface method | `CallMethod` | ✅ Have (combined) |
| 176 | `CallPtr` | Call through pointer | `CallPtr` | ✅ Have |
| 200 | `Thiscall1` | Optimized this call | - | ❌ Missing (optimization) |

#### Control Flow
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 11 | `JMP` | Unconditional jump | `Jump` | ✅ Have |
| 12 | `JZ` | Jump if zero | `JumpIfFalse` | ✅ Have |
| 13 | `JNZ` | Jump if not zero | `JumpIfTrue` | ✅ Have |
| 14 | `JS` | Jump if signed/negative | - | ❌ Missing |
| 15 | `JNS` | Jump if not signed | - | ❌ Missing |
| 16 | `JP` | Jump if positive | - | ❌ Missing |
| 17 | `JNP` | Jump if not positive | - | ❌ Missing |
| 57 | `JMPP` | Jump table (switch) | - | ❌ Missing |
| 187 | `JLowZ` | Jump if low word zero | - | ❌ Missing |
| 188 | `JLowNZ` | Jump if low word not zero | - | ❌ Missing |

#### Test/Set Operations (Register-based)
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 18 | `TZ` | Test zero, set register | - | ❌ Missing (register-based) |
| 19 | `TNZ` | Test not zero | - | ❌ Missing |
| 20 | `TS` | Test signed | - | ❌ Missing |
| 21 | `TNS` | Test not signed | - | ❌ Missing |
| 22 | `TP` | Test positive | - | ❌ Missing |
| 23 | `TNP` | Test not positive | - | ❌ Missing |

#### Unary Operations
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 6 | `NOT` | Logical NOT | `Not` | ✅ Have |
| 24 | `NEGi` | Negate int | `Negate` | ✅ Have (generic) |
| 25 | `NEGf` | Negate float | `Negate` | ✅ Have (generic) |
| 26 | `NEGd` | Negate double | `Negate` | ✅ Have (generic) |
| 156 | `NEGi64` | Negate int64 | `Negate` | ✅ Have (generic) |
| 39 | `BNOT` | Bitwise NOT | `BitNot` | ✅ Have |
| 159 | `BNOT64` | Bitwise NOT 64-bit | `BitNot` | ✅ Have (generic) |

#### Increment/Decrement (type-specific in C++)
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 27 | `INCi16` | Increment int16 | `PreIncrement` | ✅ Have (generic) |
| 28 | `INCi8` | Increment int8 | `PreIncrement` | ✅ Have (generic) |
| 29 | `DECi16` | Decrement int16 | `PreDecrement` | ✅ Have (generic) |
| 30 | `DECi8` | Decrement int8 | `PreDecrement` | ✅ Have (generic) |
| 31 | `INCi` | Increment int | `PreIncrement` | ✅ Have (generic) |
| 32 | `DECi` | Decrement int | `PreDecrement` | ✅ Have (generic) |
| 33 | `INCf` | Increment float | `PreIncrement` | ✅ Have (generic) |
| 34 | `DECf` | Decrement float | `PreDecrement` | ✅ Have (generic) |
| 35 | `INCd` | Increment double | `PreIncrement` | ✅ Have (generic) |
| 36 | `DECd` | Decrement double | `PreDecrement` | ✅ Have (generic) |
| 37 | `IncVi` | Increment variable int | `PreIncrement` | ✅ Have (generic) |
| 38 | `DecVi` | Decrement variable int | `PreDecrement` | ✅ Have (generic) |
| 157 | `INCi64` | Increment int64 | `PreIncrement` | ✅ Have (generic) |
| 158 | `DECi64` | Decrement int64 | `PreDecrement` | ✅ Have (generic) |

#### Bitwise Operations
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 40 | `BAND` | Bitwise AND | `BitAnd` | ✅ Have |
| 41 | `BOR` | Bitwise OR | `BitOr` | ✅ Have |
| 42 | `BXOR` | Bitwise XOR | `BitXor` | ✅ Have |
| 43 | `BSLL` | Bit shift left logical | `ShiftLeft` | ✅ Have |
| 44 | `BSRL` | Bit shift right logical | `ShiftRightUnsigned` | ✅ Have |
| 45 | `BSRA` | Bit shift right arithmetic | `ShiftRight` | ✅ Have |
| 165 | `BAND64` | Bitwise AND 64-bit | `BitAnd` | ✅ Have (generic) |
| 166 | `BOR64` | Bitwise OR 64-bit | `BitOr` | ✅ Have (generic) |
| 167 | `BXOR64` | Bitwise XOR 64-bit | `BitXor` | ✅ Have (generic) |
| 168 | `BSLL64` | Bit shift left 64-bit | `ShiftLeft` | ✅ Have (generic) |
| 169 | `BSRL64` | Bit shift right logical 64-bit | `ShiftRightUnsigned` | ✅ Have (generic) |
| 170 | `BSRA64` | Bit shift right arithmetic 64-bit | `ShiftRight` | ✅ Have (generic) |

#### Comparison Operations
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 50 | `CMPd` | Compare double | `Equal`/`LessThan`/etc | ✅ Have (split) |
| 51 | `CMPu` | Compare unsigned | Same | ✅ Have (split) |
| 52 | `CMPf` | Compare float | Same | ✅ Have (split) |
| 53 | `CMPi` | Compare int | Same | ✅ Have (split) |
| 54 | `CMPIi` | Compare immediate int | - | ❌ Missing (immediate) |
| 55 | `CMPIf` | Compare immediate float | - | ❌ Missing (immediate) |
| 56 | `CMPIu` | Compare immediate unsigned | - | ❌ Missing (immediate) |
| 99 | `CmpPtr` | Compare pointers | `Equal` | ✅ Have |
| 171 | `CMPi64` | Compare int64 | Same | ✅ Have |
| 172 | `CMPu64` | Compare unsigned64 | Same | ✅ Have |

#### Arithmetic (type-specific in C++)
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 115-119 | `ADDi/SUBi/MULi/DIVi/MODi` | Int arithmetic | `Add/Sub/Mul/Div/Mod` | ✅ Have (generic) |
| 120-124 | `ADDf/SUBf/MULf/DIVf/MODf` | Float arithmetic | Same | ✅ Have (generic) |
| 125-129 | `ADDd/SUBd/MULd/DIVd/MODd` | Double arithmetic | Same | ✅ Have (generic) |
| 130-132 | `ADDIi/SUBIi/MULIi` | Immediate int ops | - | ❌ Missing (immediate) |
| 133-135 | `ADDIf/SUBIf/MULIf` | Immediate float ops | - | ❌ Missing (immediate) |
| 160-164 | `ADDi64/SUBi64/MULi64/DIVi64/MODi64` | Int64 arithmetic | Same | ✅ Have (generic) |
| 180-181 | `DIVu/MODu` | Unsigned div/mod | `Div/Mod` | ✅ Have (generic) |
| 182-183 | `DIVu64/MODu64` | Unsigned64 div/mod | Same | ✅ Have (generic) |

#### Power Operations
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 193 | `POWi` | Power int | `Pow` | ✅ Have |
| 194 | `POWu` | Power unsigned | `Pow` | ✅ Have (generic) |
| 195 | `POWf` | Power float | `Pow` | ✅ Have (generic) |
| 196 | `POWd` | Power double | `Pow` | ✅ Have (generic) |
| 197 | `POWdi` | Power double/int | `Pow` | ✅ Have (generic) |
| 198 | `POWi64` | Power int64 | `Pow` | ✅ Have (generic) |
| 199 | `POWu64` | Power unsigned64 | `Pow` | ✅ Have (generic) |

#### Memory/Copy Operations
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 46 | `COPY` | Copy memory | - | ❌ Missing |
| 60 | `STR` | String constant | `PushString` | ✅ Have |
| 63 | `SUSPEND` | Suspend execution | - | ❌ Missing (coroutines) |

#### Object Operations
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 64 | `ALLOC` | Allocate object | `CallConstructor` | ✅ Have (different) |
| 65 | `FREE` | Free object | - | ❌ Missing (GC handles) |
| 66 | `LOADOBJ` | Load object to register | - | ❌ Missing (register-based) |
| 67 | `STOREOBJ` | Store object from register | - | ❌ Missing (register-based) |
| 68 | `GETOBJ` | Get object | - | ❌ Missing |
| 69 | `REFCPY` | Reference copy | - | ❌ Missing |
| 70 | `CHKREF` | Check reference | - | ❌ Missing (null check) |
| 71 | `GETOBJREF` | Get object reference | - | ❌ Missing |
| 72 | `GETREF` | Get reference | - | ❌ Missing |
| 74 | `ClrVPtr` | Clear variable pointer | - | ❌ Missing |
| 75 | `OBJTYPE` | Object type | - | ❌ Missing (RTTI) |
| 76 | `TYPEID` | Type ID | - | ❌ Missing (RTTI) |
| 137 | `ChkRefS` | Check ref from stack | - | ❌ Missing |
| 138 | `ChkNullV` | Check null variable | - | ❌ Missing |
| 173 | `ChkNullS` | Check null from stack | - | ❌ Missing |
| 178 | `LoadThisR` | Load this to register | `LoadThis` | ✅ Have (different) |
| 184 | `LoadRObjR` | Load register object to register | - | ❌ Missing |
| 185 | `LoadVObjR` | Load variable object to register | - | ❌ Missing |
| 186 | `RefCpyV` | Reference copy to variable | - | ❌ Missing |

#### Variable Set/Copy Operations
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 77 | `SetV4` | Set 4-byte variable | `StoreLocal` | ✅ Have |
| 78 | `SetV8` | Set 8-byte variable | `StoreLocal` | ✅ Have |
| 79 | `ADDSi` | Add signed immediate | - | ❌ Missing (immediate) |
| 80-87 | `CpyVtoV*`, `CpyVtoR*`, `CpyRtoV*`, `CpyGtoV*` | Copy operations | - | ❌ Missing (register) |
| 88-91 | `WRTV1-8` | Write to variable (1-8 bytes) | - | ❌ Missing |
| 92-95 | `RDR1-8` | Read from register (1-8 bytes) | - | ❌ Missing |
| 96 | `LDG` | Load global address | `LoadGlobal` | ✅ Have (different) |
| 97 | `LDV` | Load variable address | `LoadLocal` | ✅ Have (different) |
| 98 | `PGA` | Push global address | - | ❌ Missing |
| 100 | `VAR` | Variable offset | - | ❌ Missing |
| 136 | `SetG4` | Set global 4-byte | `StoreGlobal` | ✅ Have |
| 142 | `SetV1` | Set variable 1-byte | `StoreLocal` | ✅ Have (generic) |
| 143 | `SetV2` | Set variable 2-byte | `StoreLocal` | ✅ Have (generic) |
| 174 | `ClrHi` | Clear high bits | - | ❌ Missing |

#### Type Conversions
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 101 | `iTOf` | Int to float | `ConvertI32F32` | ✅ Have |
| 102 | `fTOi` | Float to int | `ConvertF32I32` | ✅ Have |
| 103 | `uTOf` | Unsigned to float | `ConvertU32F32` | ✅ Have |
| 104 | `fTOu` | Float to unsigned | `ConvertF32U32` | ✅ Have |
| 105 | `sbTOi` | Signed byte to int | `ConvertI8I32` | ✅ Have |
| 106 | `swTOi` | Signed word to int | `ConvertI16I32` | ✅ Have |
| 107 | `ubTOi` | Unsigned byte to int | `ConvertU8U32` | ✅ Have |
| 108 | `uwTOi` | Unsigned word to int | `ConvertU16U32` | ✅ Have |
| 109 | `dTOi` | Double to int | `ConvertF64I32` | ✅ Have |
| 110 | `dTOu` | Double to unsigned | `ConvertF64U32` | ✅ Have |
| 111 | `dTOf` | Double to float | `ConvertF64F32` | ✅ Have |
| 112 | `iTOd` | Int to double | `ConvertI32F64` | ✅ Have |
| 113 | `uTOd` | Unsigned to double | `ConvertU32F64` | ✅ Have |
| 114 | `fTOd` | Float to double | `ConvertF32F64` | ✅ Have |
| 140 | `iTOb` | Int to byte | `ConvertI32I8` | ✅ Have |
| 141 | `iTOw` | Int to word | `ConvertI32I16` | ✅ Have |
| 144 | `Cast` | Cast (opCast) | `Cast` | ✅ Have |
| 145-155 | 64-bit conversions | Int64/float conversions | `ConvertI64*`, etc | ✅ Have |

#### Initialization Lists
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 189 | `AllocMem` | Allocate memory for list | `AllocListBuffer` | ✅ Have |
| 190 | `SetListSize` | Set list size | `SetListSize` | ✅ Have |
| 191 | `PshListElmnt` | Push list element | `PushListElement` | ✅ Have |
| 192 | `SetListType` | Set list type | `SetListType` | ✅ Have |
| - | `FREE` (for list buffer) | Free list buffer | `FreeListBuffer` | ✅ Have |

#### JIT/Debug
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 175 | `JitEntry` | JIT entry point | - | ❌ Missing (JIT) |
| 177 | `FuncPtr` | Function pointer | `FuncPtr` | ✅ Have |

#### Temporary/Debug Tokens
| # | C++ Instruction | Description | Rust Equivalent | Status |
|---|----------------|-------------|-----------------|--------|
| 250 | `TryBlock` | Try block marker | `TryStart` | ✅ Have |
| 251 | `VarDecl` | Variable declaration | - | ❌ Missing (debug) |
| 252 | `Block` | Block marker | - | ❌ Missing (debug) |
| 253 | `ObjInfo` | Object info | - | ❌ Missing (debug) |
| 254 | `LINE` | Line number | - | ❌ Missing (debug) |
| 255 | `LABEL` | Label | - | ❌ Missing (internal) |

---

### Instructions We Have That C++ Doesn't

| Instruction | Purpose |
|-------------|---------|
| `LogicalAnd`, `LogicalOr`, `LogicalXor` | Explicit logical operators (C++ uses JZ/JNZ) |
| `PostIncrement`, `PostDecrement` | Separate post-increment ops |
| `LoadField`, `StoreField` | Direct field access |
| `Dup` | Stack duplication |
| `CastHandleToConst` | T@ → const T@ |
| `CastHandleDerivedToBase` | Derived@ → Base@ |
| `CastHandleToInterface` | Class@ → Interface@ |
| `CastHandleExplicit` | Explicit handle cast via opCast() |
| Extensive `Convert*` variants | 56 conversion instructions (vs ~25 in C++) |

### Instructions To Remove

| Instruction | Reason |
|-------------|--------|
| `CreateArray` | ✅ Removed - Use `CallConstructor` for `array<T>` instead |
| `Index`, `StoreIndex` | Use method calls to `opIndex` instead |

---

### Summary: What We Need to Add

#### Critical for Functionality
| Category | Instructions | Purpose |
|----------|-------------|---------|
| FFI Calls | `CallSystem` | Call registered Rust host functions |
| Null Safety | `CheckNull` | Runtime null pointer checks |
| Object Lifecycle | `IncRef`, `DecRef` | Reference counting |

#### May Not Need (Architectural Differences)
- **Register-based ops** (`CpyVtoR`, `CpyRtoV`, etc.) - we're pure stack
- **Immediate operand variants** - optimization, not required
- **Type-specific arithmetic** - we use generic instructions
- **JIT support** - not planned initially
- **Debug tokens** - can add later for stack traces

---

## Initialization List Approach

### Two Strategies

#### 1. Stack-Based (Current - for simple homogeneous arrays)
```
// array<int> a = {1, 2, 3};
PushInt(1)
PushInt(2)
PushInt(3)
PushInt(3)  // count
CallConstructor { type_id, func_id }  // $array_init pops count+elements
```

**Pros:** Simple, no allocation needed
**Cons:** Only works for homogeneous arrays, constructor must know to pop from stack

#### 2. Buffer-Based (Required for dictionaries, nested lists)
```
// dictionary d = {{"key1", 1}, {"key2", 2}};
AllocListBuffer { buffer_var: 0, size: calculated_size }
SetListSize { buffer_var: 0, offset: 0, count: 2 }
// For each element pair:
PushListElement { buffer_var: 0, offset: 4 }   // string slot
// ... evaluate "key1", store in buffer ...
SetListType { buffer_var: 0, offset: X, type_id: int_id }  // for '?' pattern
PushListElement { buffer_var: 0, offset: Y }   // value slot
// ... evaluate 1, store in buffer ...
// ... repeat for remaining elements ...
LoadLocal(0)  // push buffer pointer as constructor argument
CallConstructor { type_id, func_id }  // list constructor receives buffer ptr
FreeListBuffer { buffer_var: 0, pattern_type_id }
```

**Buffer Layout for array `{repeat T}`:**
```
[count: u32][elem0: T][elem1: T]...[elemN: T]
```

**Buffer Layout for dictionary `{repeat {string, ?}}`:**
```
[count: u32]
[{string_ptr: ptr, type_id: u32, value: varies}]
[{string_ptr: ptr, type_id: u32, value: varies}]
...
```

### asBEHAVE_LIST_CONSTRUCT / asBEHAVE_LIST_FACTORY

In C++ AngelScript, list constructors are registered with special behaviors:
- `asBEHAVE_LIST_FACTORY` for reference types (returns handle)
- `asBEHAVE_LIST_CONSTRUCT` for value types (initializes in-place)

The pattern string describes buffer layout:
- `{repeat T}` - repeated elements of type T
- `{repeat {string, ?}}` - repeated pairs of string key + any type value
- `{repeat {repeat_same T}}` - for grid (2D array)

### Current Status

- ✅ Stack-based approach implemented for simple arrays
- ✅ Buffer-based instructions added (not yet used)
- ⏳ TODO: Implement buffer-based codegen for dictionary init lists
- ⏳ TODO: Register asBEHAVE_LIST_FACTORY for array/dictionary in FFI

---

## Memory Management

### Reference Counting

**Decision**: Use reference counting (similar to C++ AngelScript)

Implementation approach:
- Objects in `Runtime::ObjectPool` have refcount
- `IncRef` instruction: increment count
- `DecRef` instruction: decrement, free if zero
- Handle assignment automatically adjusts refcounts

### When Refcount Changes
- Handle assignment: `DecRef` old, `IncRef` new
- Function parameter passing: `IncRef` on entry (for handles)
- Function return: Caller takes ownership
- Scope exit: `DecRef` all handles in scope

### Object Pool
From architecture.md - objects stored in `Runtime::ObjectPool`:
- Generational handles prevent dangling references
- Pool recycles slots for efficiency

---

## Open Questions

1. **Coroutines/Suspension**: Support `SUSPEND` for async scripts?
2. **Debug Info**: Embed line numbers for stack traces?
3. **JIT**: Consider JIT compilation path?

---

## References
- [architecture.md](../docs/architecture.md) - Overall system design
- C++ AngelScript bytecode: `reference/angelscript/include/angelscript.h:1434-1646`
