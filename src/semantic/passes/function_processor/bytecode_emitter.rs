//! Bytecode emission for type conversions.
//!
//! This module contains methods for emitting conversion bytecode
//! based on semantic type conversion information.

use crate::codegen::Instruction;
use crate::semantic::ConversionKind;
use crate::types::primitive_hashes;

use super::FunctionCompiler;

impl<'ast> FunctionCompiler<'ast> {
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
                    (primitive_hashes::INT8, primitive_hashes::FLOAT) => Instruction::ConvertI8F32,
                    (primitive_hashes::INT16, primitive_hashes::FLOAT) => Instruction::ConvertI16F32,
                    (primitive_hashes::INT32, primitive_hashes::FLOAT) => Instruction::ConvertI32F32,
                    (primitive_hashes::INT64, primitive_hashes::FLOAT) => Instruction::ConvertI64F32,
                    (primitive_hashes::INT8, primitive_hashes::DOUBLE) => Instruction::ConvertI8F64,
                    (primitive_hashes::INT16, primitive_hashes::DOUBLE) => Instruction::ConvertI16F64,
                    (primitive_hashes::INT32, primitive_hashes::DOUBLE) => Instruction::ConvertI32F64,
                    (primitive_hashes::INT64, primitive_hashes::DOUBLE) => Instruction::ConvertI64F64,

                    // Unsigned to Float conversions
                    (primitive_hashes::UINT8, primitive_hashes::FLOAT) => Instruction::ConvertU8F32,
                    (primitive_hashes::UINT16, primitive_hashes::FLOAT) => Instruction::ConvertU16F32,
                    (primitive_hashes::UINT32, primitive_hashes::FLOAT) => Instruction::ConvertU32F32,
                    (primitive_hashes::UINT64, primitive_hashes::FLOAT) => Instruction::ConvertU64F32,
                    (primitive_hashes::UINT8, primitive_hashes::DOUBLE) => Instruction::ConvertU8F64,
                    (primitive_hashes::UINT16, primitive_hashes::DOUBLE) => Instruction::ConvertU16F64,
                    (primitive_hashes::UINT32, primitive_hashes::DOUBLE) => Instruction::ConvertU32F64,
                    (primitive_hashes::UINT64, primitive_hashes::DOUBLE) => Instruction::ConvertU64F64,

                    // Float to Integer conversions
                    (primitive_hashes::FLOAT, primitive_hashes::INT8) => Instruction::ConvertF32I8,
                    (primitive_hashes::FLOAT, primitive_hashes::INT16) => Instruction::ConvertF32I16,
                    (primitive_hashes::FLOAT, primitive_hashes::INT32) => Instruction::ConvertF32I32,
                    (primitive_hashes::FLOAT, primitive_hashes::INT64) => Instruction::ConvertF32I64,
                    (primitive_hashes::DOUBLE, primitive_hashes::INT8) => Instruction::ConvertF64I8,
                    (primitive_hashes::DOUBLE, primitive_hashes::INT16) => Instruction::ConvertF64I16,
                    (primitive_hashes::DOUBLE, primitive_hashes::INT32) => Instruction::ConvertF64I32,
                    (primitive_hashes::DOUBLE, primitive_hashes::INT64) => Instruction::ConvertF64I64,

                    // Float to Unsigned conversions
                    (primitive_hashes::FLOAT, primitive_hashes::UINT8) => Instruction::ConvertF32U8,
                    (primitive_hashes::FLOAT, primitive_hashes::UINT16) => Instruction::ConvertF32U16,
                    (primitive_hashes::FLOAT, primitive_hashes::UINT32) => Instruction::ConvertF32U32,
                    (primitive_hashes::FLOAT, primitive_hashes::UINT64) => Instruction::ConvertF32U64,
                    (primitive_hashes::DOUBLE, primitive_hashes::UINT8) => Instruction::ConvertF64U8,
                    (primitive_hashes::DOUBLE, primitive_hashes::UINT16) => Instruction::ConvertF64U16,
                    (primitive_hashes::DOUBLE, primitive_hashes::UINT32) => Instruction::ConvertF64U32,
                    (primitive_hashes::DOUBLE, primitive_hashes::UINT64) => Instruction::ConvertF64U64,

                    // Float ↔ Double conversions
                    (primitive_hashes::FLOAT, primitive_hashes::DOUBLE) => Instruction::ConvertF32F64,
                    (primitive_hashes::DOUBLE, primitive_hashes::FLOAT) => Instruction::ConvertF64F32,

                    // Integer widening (signed)
                    (primitive_hashes::INT8, primitive_hashes::INT16) => Instruction::ConvertI8I16,
                    (primitive_hashes::INT8, primitive_hashes::INT32) => Instruction::ConvertI8I32,
                    (primitive_hashes::INT8, primitive_hashes::INT64) => Instruction::ConvertI8I64,
                    (primitive_hashes::INT16, primitive_hashes::INT32) => Instruction::ConvertI16I32,
                    (primitive_hashes::INT16, primitive_hashes::INT64) => Instruction::ConvertI16I64,
                    (primitive_hashes::INT32, primitive_hashes::INT64) => Instruction::ConvertI32I64,

                    // Integer narrowing (signed)
                    (primitive_hashes::INT64, primitive_hashes::INT32) => Instruction::ConvertI64I32,
                    (primitive_hashes::INT64, primitive_hashes::INT16) => Instruction::ConvertI64I16,
                    (primitive_hashes::INT64, primitive_hashes::INT8) => Instruction::ConvertI64I8,
                    (primitive_hashes::INT32, primitive_hashes::INT16) => Instruction::ConvertI32I16,
                    (primitive_hashes::INT32, primitive_hashes::INT8) => Instruction::ConvertI32I8,
                    (primitive_hashes::INT16, primitive_hashes::INT8) => Instruction::ConvertI16I8,

                    // Unsigned widening
                    (primitive_hashes::UINT8, primitive_hashes::UINT16) => Instruction::ConvertU8U16,
                    (primitive_hashes::UINT8, primitive_hashes::UINT32) => Instruction::ConvertU8U32,
                    (primitive_hashes::UINT8, primitive_hashes::UINT64) => Instruction::ConvertU8U64,
                    (primitive_hashes::UINT16, primitive_hashes::UINT32) => Instruction::ConvertU16U32,
                    (primitive_hashes::UINT16, primitive_hashes::UINT64) => Instruction::ConvertU16U64,
                    (primitive_hashes::UINT32, primitive_hashes::UINT64) => Instruction::ConvertU32U64,

                    // Unsigned narrowing
                    (primitive_hashes::UINT64, primitive_hashes::UINT32) => Instruction::ConvertU64U32,
                    (primitive_hashes::UINT64, primitive_hashes::UINT16) => Instruction::ConvertU64U16,
                    (primitive_hashes::UINT64, primitive_hashes::UINT8) => Instruction::ConvertU64U8,
                    (primitive_hashes::UINT32, primitive_hashes::UINT16) => Instruction::ConvertU32U16,
                    (primitive_hashes::UINT32, primitive_hashes::UINT8) => Instruction::ConvertU32U8,
                    (primitive_hashes::UINT16, primitive_hashes::UINT8) => Instruction::ConvertU16U8,

                    // Signed/Unsigned reinterpret
                    (primitive_hashes::INT8, primitive_hashes::UINT8) => Instruction::ConvertI8U8,
                    (primitive_hashes::INT16, primitive_hashes::UINT16) => Instruction::ConvertI16U16,
                    (primitive_hashes::INT32, primitive_hashes::UINT32) => Instruction::ConvertI32U32,
                    (primitive_hashes::INT64, primitive_hashes::UINT64) => Instruction::ConvertI64U64,
                    (primitive_hashes::UINT8, primitive_hashes::INT8) => Instruction::ConvertU8I8,
                    (primitive_hashes::UINT16, primitive_hashes::INT16) => Instruction::ConvertU16I16,
                    (primitive_hashes::UINT32, primitive_hashes::INT32) => Instruction::ConvertU32I32,
                    (primitive_hashes::UINT64, primitive_hashes::INT64) => Instruction::ConvertU64I64,

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
