# Current Task: Compiler Implementation

**Status:** Planning Complete - Ready for Implementation
**Date:** 2025-12-08
**Branch:** task/01-unified-type-registry

---

## Current State Summary

**Parser:** 100% Complete
**FFI:** 100% Complete
**Compiler:** Task breakdown complete (17 tasks)

---

## Compiler Tasks (31-47)

All task files created in `claude/tasks/`:

| Task | File | Description | Status |
|------|------|-------------|--------|
| 31 | `31_compiler_foundation.md` | Core types, bytecode, opcodes | Pending |
| 32 | `32_string_factory.md` | String literal factory config | Pending |
| 33 | `33_compilation_context.md` | Unified type lookup | Pending |
| 34 | `34_type_resolution.md` | TypeExpr → DataType | Pending |
| 35 | `35_template_instantiation.md` | Template types, functions, cache | Pending |
| 36 | `36_conversion_system.md` | Type conversions with costs | Pending |
| 37 | `37_overload_resolution.md` | Function/operator selection | Pending |
| 38 | `38_registration_pass.md` | Pass 1 - declarations | Pending |
| 39 | `39_local_scope.md` | Variable tracking, captures | Pending |
| 40 | `40_bytecode_emitter.md` | Instruction emission, jumps | Pending |
| 41 | `41_expression_basics.md` | Literals, identifiers, operators | Pending |
| 42 | `42_expression_calls.md` | Function/method calls | Pending |
| 43 | `43_expression_advanced.md` | Cast, lambda, ternary | Pending |
| 44 | `44_statement_basics.md` | Blocks, var decl, if, while | Pending |
| 45 | `45_statement_loops.md` | For, foreach, switch | Pending |
| 46 | `46_function_compilation.md` | Pass 2 orchestration | Pending |
| 47 | `47_integration_testing.md` | All tests passing, performance | Pending |

---

## Key Design Documents

- `claude/compiler_design.md` - Master compiler design (includes Section 17: Template Instantiation)
- `claude/plans/cuddly-puzzling-newt.md` - Template instantiation detailed design

---

## Next Steps

1. Start with Task 31: Compiler Foundation
2. Each task is independently implementable and committable
3. Tasks should be implemented in order (dependencies flow forward)

---

## Architecture Overview

**Two-pass compilation:**
1. **Registration Pass (Task 38):** Walk AST, register types and function signatures
2. **Compilation Pass (Task 46):** Generate bytecode for function bodies

**Bidirectional type checking:**
- `infer(expr)` - Synthesize type from expression
- `check(expr, expected)` - Verify expression has expected type

**Key types:**
- `TypeHash` - 64-bit Copy type for O(1) lookups
- `DataType` - Type with modifiers (const, handle, ref)
- `ExprInfo` - Expression result (data_type, is_lvalue, is_mutable)
- `BytecodeChunk` - Instructions + debug info (stored in `FunctionImpl::Script`)
- `ConstantPool` - Module-level constants with deduplication
- `CompiledModule` - Shared constants + list of compiled functions (bytecode in registry)
