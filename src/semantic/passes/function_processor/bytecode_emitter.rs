//! Bytecode emission for type conversions.
//!
//! This module contains methods for emitting conversion bytecode
//! based on semantic type conversion information.

use crate::codegen::Instruction;
use crate::semantic::ConversionKind;
use crate::semantic::types::type_def::{
    DOUBLE_TYPE, FLOAT_TYPE, INT8_TYPE, INT16_TYPE, INT32_TYPE, INT64_TYPE,
    UINT8_TYPE, UINT16_TYPE, UINT32_TYPE, UINT64_TYPE,
};

use super::FunctionCompiler;

impl<'src, 'ast> FunctionCompiler<'src, 'ast> {
    /// Emit conversion instruction based on ConversionKind.
    ///
    /// Maps semantic conversion information to the appropriate bytecode instruction.
    pub(super) fn emit_conversion(&mut self, conversion: &crate::semantic::Conversion) {
        match &conversion.kind {
            ConversionKind::Identity => {
                // No instruction needed for identity conversion
            }

            ConversionKind::NullToHandle => {
                // No instruction needed - PushNull already pushed the null value
                // The VM will interpret this as the appropriate handle type
            }

            ConversionKind::Primitive { from_type, to_type } => {
                // Select instruction based on type pair
                let instruction = match (*from_type, *to_type) {
                    // Integer to Float conversions
                    (INT8_TYPE, FLOAT_TYPE) => Instruction::ConvertI8F32,
                    (INT16_TYPE, FLOAT_TYPE) => Instruction::ConvertI16F32,
                    (INT32_TYPE, FLOAT_TYPE) => Instruction::ConvertI32F32,
                    (INT64_TYPE, FLOAT_TYPE) => Instruction::ConvertI64F32,
                    (INT8_TYPE, DOUBLE_TYPE) => Instruction::ConvertI8F64,
                    (INT16_TYPE, DOUBLE_TYPE) => Instruction::ConvertI16F64,
                    (INT32_TYPE, DOUBLE_TYPE) => Instruction::ConvertI32F64,
                    (INT64_TYPE, DOUBLE_TYPE) => Instruction::ConvertI64F64,

                    // Unsigned to Float conversions
                    (UINT8_TYPE, FLOAT_TYPE) => Instruction::ConvertU8F32,
                    (UINT16_TYPE, FLOAT_TYPE) => Instruction::ConvertU16F32,
                    (UINT32_TYPE, FLOAT_TYPE) => Instruction::ConvertU32F32,
                    (UINT64_TYPE, FLOAT_TYPE) => Instruction::ConvertU64F32,
                    (UINT8_TYPE, DOUBLE_TYPE) => Instruction::ConvertU8F64,
                    (UINT16_TYPE, DOUBLE_TYPE) => Instruction::ConvertU16F64,
                    (UINT32_TYPE, DOUBLE_TYPE) => Instruction::ConvertU32F64,
                    (UINT64_TYPE, DOUBLE_TYPE) => Instruction::ConvertU64F64,

                    // Float to Integer conversions
                    (FLOAT_TYPE, INT8_TYPE) => Instruction::ConvertF32I8,
                    (FLOAT_TYPE, INT16_TYPE) => Instruction::ConvertF32I16,
                    (FLOAT_TYPE, INT32_TYPE) => Instruction::ConvertF32I32,
                    (FLOAT_TYPE, INT64_TYPE) => Instruction::ConvertF32I64,
                    (DOUBLE_TYPE, INT8_TYPE) => Instruction::ConvertF64I8,
                    (DOUBLE_TYPE, INT16_TYPE) => Instruction::ConvertF64I16,
                    (DOUBLE_TYPE, INT32_TYPE) => Instruction::ConvertF64I32,
                    (DOUBLE_TYPE, INT64_TYPE) => Instruction::ConvertF64I64,

                    // Float to Unsigned conversions
                    (FLOAT_TYPE, UINT8_TYPE) => Instruction::ConvertF32U8,
                    (FLOAT_TYPE, UINT16_TYPE) => Instruction::ConvertF32U16,
                    (FLOAT_TYPE, UINT32_TYPE) => Instruction::ConvertF32U32,
                    (FLOAT_TYPE, UINT64_TYPE) => Instruction::ConvertF32U64,
                    (DOUBLE_TYPE, UINT8_TYPE) => Instruction::ConvertF64U8,
                    (DOUBLE_TYPE, UINT16_TYPE) => Instruction::ConvertF64U16,
                    (DOUBLE_TYPE, UINT32_TYPE) => Instruction::ConvertF64U32,
                    (DOUBLE_TYPE, UINT64_TYPE) => Instruction::ConvertF64U64,

                    // Float ↔ Double conversions
                    (FLOAT_TYPE, DOUBLE_TYPE) => Instruction::ConvertF32F64,
                    (DOUBLE_TYPE, FLOAT_TYPE) => Instruction::ConvertF64F32,

                    // Integer widening (signed)
                    (INT8_TYPE, INT16_TYPE) => Instruction::ConvertI8I16,
                    (INT8_TYPE, INT32_TYPE) => Instruction::ConvertI8I32,
                    (INT8_TYPE, INT64_TYPE) => Instruction::ConvertI8I64,
                    (INT16_TYPE, INT32_TYPE) => Instruction::ConvertI16I32,
                    (INT16_TYPE, INT64_TYPE) => Instruction::ConvertI16I64,
                    (INT32_TYPE, INT64_TYPE) => Instruction::ConvertI32I64,

                    // Integer narrowing (signed)
                    (INT64_TYPE, INT32_TYPE) => Instruction::ConvertI64I32,
                    (INT64_TYPE, INT16_TYPE) => Instruction::ConvertI64I16,
                    (INT64_TYPE, INT8_TYPE) => Instruction::ConvertI64I8,
                    (INT32_TYPE, INT16_TYPE) => Instruction::ConvertI32I16,
                    (INT32_TYPE, INT8_TYPE) => Instruction::ConvertI32I8,
                    (INT16_TYPE, INT8_TYPE) => Instruction::ConvertI16I8,

                    // Unsigned widening
                    (UINT8_TYPE, UINT16_TYPE) => Instruction::ConvertU8U16,
                    (UINT8_TYPE, UINT32_TYPE) => Instruction::ConvertU8U32,
                    (UINT8_TYPE, UINT64_TYPE) => Instruction::ConvertU8U64,
                    (UINT16_TYPE, UINT32_TYPE) => Instruction::ConvertU16U32,
                    (UINT16_TYPE, UINT64_TYPE) => Instruction::ConvertU16U64,
                    (UINT32_TYPE, UINT64_TYPE) => Instruction::ConvertU32U64,

                    // Unsigned narrowing
                    (UINT64_TYPE, UINT32_TYPE) => Instruction::ConvertU64U32,
                    (UINT64_TYPE, UINT16_TYPE) => Instruction::ConvertU64U16,
                    (UINT64_TYPE, UINT8_TYPE) => Instruction::ConvertU64U8,
                    (UINT32_TYPE, UINT16_TYPE) => Instruction::ConvertU32U16,
                    (UINT32_TYPE, UINT8_TYPE) => Instruction::ConvertU32U8,
                    (UINT16_TYPE, UINT8_TYPE) => Instruction::ConvertU16U8,

                    // Signed/Unsigned reinterpret
                    (INT8_TYPE, UINT8_TYPE) => Instruction::ConvertI8U8,
                    (INT16_TYPE, UINT16_TYPE) => Instruction::ConvertI16U16,
                    (INT32_TYPE, UINT32_TYPE) => Instruction::ConvertI32U32,
                    (INT64_TYPE, UINT64_TYPE) => Instruction::ConvertI64U64,
                    (UINT8_TYPE, INT8_TYPE) => Instruction::ConvertU8I8,
                    (UINT16_TYPE, INT16_TYPE) => Instruction::ConvertU16I16,
                    (UINT32_TYPE, INT32_TYPE) => Instruction::ConvertU32I32,
                    (UINT64_TYPE, INT64_TYPE) => Instruction::ConvertU64I64,

                    _ => {
                        // This should never happen if the semantic analyzer is correct
                        return;
                    }
                };
                self.bytecode.emit(instruction);
            }

            ConversionKind::HandleToConst => {
                self.bytecode.emit(Instruction::CastHandleToConst);
            }

            ConversionKind::DerivedToBase => {
                self.bytecode.emit(Instruction::CastHandleDerivedToBase);
            }

            ConversionKind::ClassToInterface => {
                self.bytecode.emit(Instruction::CastHandleToInterface);
            }

            ConversionKind::ConstructorConversion { constructor_id } => {
                self.bytecode.emit(Instruction::CallMethod(constructor_id.0));
            }

            ConversionKind::ImplicitConversionMethod { method_id } => {
                self.bytecode.emit(Instruction::CallMethod(method_id.0));
            }

            ConversionKind::ExplicitCastMethod { method_id } => {
                self.bytecode.emit(Instruction::CallMethod(method_id.0));
            }

            ConversionKind::ImplicitCastMethod { method_id } => {
                self.bytecode.emit(Instruction::CallMethod(method_id.0));
            }

            ConversionKind::ValueToHandle => {
                // Value type to handle conversion - the VM handles this by creating
                // a reference to the value on the stack. No additional instruction needed
                // since the value is already on the stack and can be used as a handle.
                self.bytecode.emit(Instruction::ValueToHandle);
            }
        }
    }
}
