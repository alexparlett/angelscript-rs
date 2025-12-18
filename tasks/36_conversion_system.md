# Task 36: Conversion System

## Overview

Implement the type conversion checking system that determines if one type can be converted to another, and at what cost. This is essential for type checking and overload resolution.

## Goals

1. Check if type A can convert to type B
2. Determine if conversion is implicit or explicit only
3. Calculate conversion cost for overload resolution
4. Generate bytecode instructions for conversions

## Dependencies

- Task 31: Compiler Foundation (Conversion types)
- Task 33: Compilation Context

## Files to Create/Modify

```
crates/angelscript-compiler/src/
├── conversion/
│   ├── mod.rs             # Main conversion logic
│   ├── primitive.rs       # Primitive type conversions
│   ├── handle.rs          # Handle conversions
│   ├── user_defined.rs    # User-defined conversions
│   └── emit.rs            # Bytecode emission for conversions
└── lib.rs
```

## Detailed Implementation

### 1. Main Conversion Logic (conversion/mod.rs)

```rust
use angelscript_core::{DataType, TypeHash};

use crate::context::CompilationContext;
use crate::error::Result;

mod primitive;
mod handle;
mod user_defined;
pub mod emit;

pub use crate::conversion::{Conversion, ConversionKind};

/// Check if source type can convert to target type.
pub fn find_conversion(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<Conversion> {
    // 1. Identity check (exact match)
    if source.type_hash == target.type_hash && source.modifiers_match(target) {
        return Some(Conversion::identity());
    }

    // 2. Const relaxation (non-const to const is free)
    if source.type_hash == target.type_hash && !source.is_const && target.is_const {
        return Some(Conversion {
            kind: ConversionKind::Identity,
            cost: Conversion::COST_CONST_ADDITION,
            is_implicit: true,
        });
    }

    // 3. Primitive conversions
    if let Some(conv) = primitive::find_primitive_conversion(source, target, ctx) {
        return Some(conv);
    }

    // 4. Handle conversions
    if let Some(conv) = handle::find_handle_conversion(source, target, ctx) {
        return Some(conv);
    }

    // 5. Class hierarchy conversions
    if let Some(conv) = find_hierarchy_conversion(source, target, ctx) {
        return Some(conv);
    }

    // 6. User-defined conversions (constructor, opConv, opCast)
    if let Some(conv) = user_defined::find_user_conversion(source, target, ctx) {
        return Some(conv);
    }

    None
}

/// Check if implicit conversion is possible.
pub fn can_implicitly_convert(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> bool {
    find_conversion(source, target, ctx)
        .map(|c| c.is_implicit)
        .unwrap_or(false)
}

/// Find class hierarchy conversion (derived to base, class to interface).
fn find_hierarchy_conversion(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<Conversion> {
    let source_class = ctx.get_type(source.type_hash)?.as_class()?;
    let target_class = ctx.get_type(target.type_hash);

    // Derived to base class
    if let Some(target_class) = target_class.and_then(|t| t.as_class()) {
        if is_derived_from(source.type_hash, target.type_hash, ctx) {
            return Some(Conversion {
                kind: ConversionKind::DerivedToBase { base: target.type_hash },
                cost: Conversion::COST_DERIVED_TO_BASE,
                is_implicit: true,
            });
        }
    }

    // Class to interface
    if let Some(target_interface) = ctx.get_type(target.type_hash).and_then(|t| t.as_interface()) {
        if source_class.interfaces.contains(&target.type_hash) {
            return Some(Conversion {
                kind: ConversionKind::ClassToInterface { interface: target.type_hash },
                cost: Conversion::COST_CLASS_TO_INTERFACE,
                is_implicit: true,
            });
        }
    }

    None
}

/// Check if source is derived from target.
fn is_derived_from(source: TypeHash, target: TypeHash, ctx: &CompilationContext<'_>) -> bool {
    let mut current = source;
    while let Some(class) = ctx.get_type(current).and_then(|t| t.as_class()) {
        if let Some(base) = class.base_class {
            if base == target {
                return true;
            }
            current = base;
        } else {
            break;
        }
    }
    false
}
```

### 2. Primitive Conversions (conversion/primitive.rs)

```rust
use angelscript_core::{primitives, DataType, TypeHash};

use crate::context::CompilationContext;
use super::{Conversion, ConversionKind};

/// Find primitive type conversion.
pub fn find_primitive_conversion(
    source: &DataType,
    target: &DataType,
    _ctx: &CompilationContext<'_>,
) -> Option<Conversion> {
    let from = source.type_hash;
    let to = target.type_hash;

    // Integer widening (always implicit, low cost)
    if is_integer_widening(from, to) {
        return Some(Conversion {
            kind: ConversionKind::Primitive { from, to },
            cost: Conversion::COST_PRIMITIVE_WIDENING,
            is_implicit: true,
        });
    }

    // Integer narrowing (implicit but higher cost)
    if is_integer_narrowing(from, to) {
        return Some(Conversion {
            kind: ConversionKind::Primitive { from, to },
            cost: Conversion::COST_PRIMITIVE_NARROWING,
            is_implicit: true,  // AngelScript allows implicit narrowing
        });
    }

    // Integer to float (implicit)
    if is_int_to_float(from, to) {
        return Some(Conversion {
            kind: ConversionKind::Primitive { from, to },
            cost: Conversion::COST_PRIMITIVE_WIDENING,
            is_implicit: true,
        });
    }

    // Float to integer (implicit with truncation)
    if is_float_to_int(from, to) {
        return Some(Conversion {
            kind: ConversionKind::Primitive { from, to },
            cost: Conversion::COST_PRIMITIVE_NARROWING + 1,  // Extra cost for truncation
            is_implicit: true,
        });
    }

    // Float widening/narrowing
    if is_float_conversion(from, to) {
        return Some(Conversion {
            kind: ConversionKind::Primitive { from, to },
            cost: Conversion::COST_PRIMITIVE_WIDENING,
            is_implicit: true,
        });
    }

    None
}

fn is_integer_widening(from: TypeHash, to: TypeHash) -> bool {
    use primitives::*;

    matches!(
        (from, to),
        // Signed widening
        (INT8, INT16) | (INT8, INT32) | (INT8, INT64) |
        (INT16, INT32) | (INT16, INT64) |
        (INT32, INT64) |
        // Unsigned widening
        (UINT8, UINT16) | (UINT8, UINT32) | (UINT8, UINT64) |
        (UINT16, UINT32) | (UINT16, UINT64) |
        (UINT32, UINT64) |
        // Unsigned to larger signed
        (UINT8, INT16) | (UINT8, INT32) | (UINT8, INT64) |
        (UINT16, INT32) | (UINT16, INT64) |
        (UINT32, INT64)
    )
}

fn is_integer_narrowing(from: TypeHash, to: TypeHash) -> bool {
    use primitives::*;

    matches!(
        (from, to),
        // Signed narrowing
        (INT64, INT32) | (INT64, INT16) | (INT64, INT8) |
        (INT32, INT16) | (INT32, INT8) |
        (INT16, INT8) |
        // Unsigned narrowing
        (UINT64, UINT32) | (UINT64, UINT16) | (UINT64, UINT8) |
        (UINT32, UINT16) | (UINT32, UINT8) |
        (UINT16, UINT8) |
        // Cross-sign conversions
        (INT32, UINT32) | (UINT32, INT32) |
        (INT64, UINT64) | (UINT64, INT64)
    )
}

fn is_int_to_float(from: TypeHash, to: TypeHash) -> bool {
    use primitives::*;

    let is_int = matches!(from, INT8 | INT16 | INT32 | INT64 | UINT8 | UINT16 | UINT32 | UINT64);
    let is_float = matches!(to, FLOAT | DOUBLE);
    is_int && is_float
}

fn is_float_to_int(from: TypeHash, to: TypeHash) -> bool {
    use primitives::*;

    let is_float = matches!(from, FLOAT | DOUBLE);
    let is_int = matches!(to, INT8 | INT16 | INT32 | INT64 | UINT8 | UINT16 | UINT32 | UINT64);
    is_float && is_int
}

fn is_float_conversion(from: TypeHash, to: TypeHash) -> bool {
    use primitives::*;
    matches!((from, to), (FLOAT, DOUBLE) | (DOUBLE, FLOAT))
}

/// Get the opcode for a primitive conversion.
pub fn get_conversion_opcode(from: TypeHash, to: TypeHash) -> Option<crate::bytecode::OpCode> {
    use crate::bytecode::OpCode;
    use primitives::*;

    Some(match (from, to) {
        // Integer widening
        (INT8, INT16) => OpCode::I8toI16,
        (INT8, INT32) => OpCode::I8toI32,
        (INT8, INT64) => OpCode::I8toI64,
        (INT16, INT32) => OpCode::I16toI32,
        (INT16, INT64) => OpCode::I16toI64,
        (INT32, INT64) => OpCode::I32toI64,

        // Integer narrowing
        (INT64, INT32) => OpCode::I64toI32,
        (INT64, INT16) => OpCode::I64toI16,
        (INT64, INT8) => OpCode::I64toI8,
        (INT32, INT16) => OpCode::I32toI16,
        (INT32, INT8) => OpCode::I32toI8,
        (INT16, INT8) => OpCode::I16toI8,

        // Int to float
        (INT32, FLOAT) => OpCode::I32toF32,
        (INT32, DOUBLE) => OpCode::I32toF64,
        (INT64, FLOAT) => OpCode::I64toF32,
        (INT64, DOUBLE) => OpCode::I64toF64,

        // Float to int
        (FLOAT, INT32) => OpCode::F32toI32,
        (FLOAT, INT64) => OpCode::F32toI64,
        (DOUBLE, INT32) => OpCode::F64toI32,
        (DOUBLE, INT64) => OpCode::F64toI64,

        // Float conversion
        (FLOAT, DOUBLE) => OpCode::F32toF64,
        (DOUBLE, FLOAT) => OpCode::F64toF32,

        _ => return None,
    })
}
```

### 3. Handle Conversions (conversion/handle.rs)

```rust
use angelscript_core::DataType;

use crate::context::CompilationContext;
use super::{Conversion, ConversionKind};

/// Find handle-related conversions.
pub fn find_handle_conversion(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<Conversion> {
    // Null to any handle type
    if source.is_null() && target.is_handle {
        return Some(Conversion {
            kind: ConversionKind::NullToHandle,
            cost: Conversion::COST_CONST_ADDITION,
            is_implicit: true,
        });
    }

    // Handle to const handle (same type)
    if source.type_hash == target.type_hash
        && source.is_handle
        && target.is_handle
        && !source.is_handle_to_const
        && target.is_handle_to_const
    {
        return Some(Conversion {
            kind: ConversionKind::HandleToConst,
            cost: Conversion::COST_CONST_ADDITION,
            is_implicit: true,
        });
    }

    // Value to handle (@value) - explicit only
    if !source.is_handle && target.is_handle && source.type_hash == target.type_hash {
        return Some(Conversion {
            kind: ConversionKind::ValueToHandle,
            cost: Conversion::COST_EXPLICIT_ONLY,
            is_implicit: false,
        });
    }

    None
}
```

### 4. User-Defined Conversions (conversion/user_defined.rs)

```rust
use angelscript_core::{DataType, TypeHash};

use crate::context::CompilationContext;
use super::{Conversion, ConversionKind};

/// Find user-defined conversions (constructor, opImplConv, opCast).
pub fn find_user_conversion(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<Conversion> {
    // Try implicit conversion method on source (opImplConv)
    if let Some(method) = find_implicit_conv_method(source.type_hash, target.type_hash, ctx) {
        return Some(Conversion {
            kind: ConversionKind::ImplicitConvMethod { method },
            cost: Conversion::COST_USER_IMPLICIT,
            is_implicit: true,
        });
    }

    // Try constructor conversion on target
    if let Some(ctor) = find_converting_constructor(source.type_hash, target.type_hash, ctx) {
        return Some(Conversion {
            kind: ConversionKind::ConstructorConversion { constructor: ctor },
            cost: Conversion::COST_USER_IMPLICIT,
            is_implicit: true,
        });
    }

    // Try explicit cast method (opCast)
    if let Some(method) = find_cast_method(source.type_hash, target.type_hash, ctx) {
        return Some(Conversion {
            kind: ConversionKind::ExplicitCastMethod { method },
            cost: Conversion::COST_EXPLICIT_ONLY,
            is_implicit: false,
        });
    }

    None
}

fn find_implicit_conv_method(
    source: TypeHash,
    target: TypeHash,
    ctx: &CompilationContext<'_>,
) -> Option<TypeHash> {
    let class = ctx.get_type(source)?.as_class()?;

    for method_hash in &class.methods {
        if let Some(func) = ctx.get_function(*method_hash) {
            let def = func.def();
            // opImplConv with return type matching target
            if def.name == "opImplConv" && def.return_type.type_hash == target {
                return Some(*method_hash);
            }
        }
    }

    None
}

fn find_converting_constructor(
    source: TypeHash,
    target: TypeHash,
    ctx: &CompilationContext<'_>,
) -> Option<TypeHash> {
    let target_class = ctx.get_type(target)?.as_class()?;

    // Look for single-argument constructor taking source type
    for ctor_hash in &target_class.behaviors.constructors {
        if let Some(func) = ctx.get_function(*ctor_hash) {
            let def = func.def();
            if def.params.len() == 1 && def.params[0].data_type.type_hash == source {
                return Some(*ctor_hash);
            }
        }
    }

    None
}

fn find_cast_method(
    source: TypeHash,
    target: TypeHash,
    ctx: &CompilationContext<'_>,
) -> Option<TypeHash> {
    let class = ctx.get_type(source)?.as_class()?;

    for method_hash in &class.methods {
        if let Some(func) = ctx.get_function(*method_hash) {
            let def = func.def();
            // opCast with return type matching target
            if def.name == "opCast" && def.return_type.type_hash == target {
                return Some(*method_hash);
            }
        }
    }

    None
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_conversion() {
        let dt = DataType::simple(primitives::INT32);
        let conv = find_conversion(&dt, &dt, &ctx);
        assert!(conv.is_some());
        assert!(conv.unwrap().is_exact());
    }

    #[test]
    fn integer_widening() {
        let from = DataType::simple(primitives::INT32);
        let to = DataType::simple(primitives::INT64);
        let conv = find_conversion(&from, &to, &ctx);
        assert!(conv.is_some());
        assert!(conv.unwrap().is_implicit);
    }

    #[test]
    fn handle_to_const() {
        let from = DataType::simple(player_hash).as_handle();
        let to = DataType::simple(player_hash).as_handle().as_const_handle();
        let conv = find_conversion(&from, &to, &ctx);
        assert!(conv.is_some());
        assert!(conv.unwrap().is_implicit);
    }
}
```

## Acceptance Criteria

- [ ] Identity conversions detected correctly
- [ ] All primitive conversions work (int widening/narrowing, float conversions)
- [ ] Handle conversions work (null to handle, handle to const)
- [ ] Class hierarchy conversions work (derived to base, class to interface)
- [ ] User-defined conversions detected (opImplConv, opCast, constructors)
- [ ] Conversion costs are correct for overload resolution
- [ ] All tests pass

## Next Phase

Task 37: Overload Resolution - select best function from candidates
