//! Bytecode emission for type conversions.
//!
//! This module contains methods for emitting conversion bytecode
//! based on semantic type conversion information.

use crate::codegen::Instruction;
use crate::semantic::ConversionKind;
use angelscript_core::primitives;

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
                    (primitives::INT8, primitives::FLOAT) => Instruction::ConvertI8F32,
                    (primitives::INT16, primitives::FLOAT) => Instruction::ConvertI16F32,
                    (primitives::INT32, primitives::FLOAT) => Instruction::ConvertI32F32,
                    (primitives::INT64, primitives::FLOAT) => Instruction::ConvertI64F32,
                    (primitives::INT8, primitives::DOUBLE) => Instruction::ConvertI8F64,
                    (primitives::INT16, primitives::DOUBLE) => Instruction::ConvertI16F64,
                    (primitives::INT32, primitives::DOUBLE) => Instruction::ConvertI32F64,
                    (primitives::INT64, primitives::DOUBLE) => Instruction::ConvertI64F64,

                    // Unsigned to Float conversions
                    (primitives::UINT8, primitives::FLOAT) => Instruction::ConvertU8F32,
                    (primitives::UINT16, primitives::FLOAT) => Instruction::ConvertU16F32,
                    (primitives::UINT32, primitives::FLOAT) => Instruction::ConvertU32F32,
                    (primitives::UINT64, primitives::FLOAT) => Instruction::ConvertU64F32,
                    (primitives::UINT8, primitives::DOUBLE) => Instruction::ConvertU8F64,
                    (primitives::UINT16, primitives::DOUBLE) => Instruction::ConvertU16F64,
                    (primitives::UINT32, primitives::DOUBLE) => Instruction::ConvertU32F64,
                    (primitives::UINT64, primitives::DOUBLE) => Instruction::ConvertU64F64,

                    // Float to Integer conversions
                    (primitives::FLOAT, primitives::INT8) => Instruction::ConvertF32I8,
                    (primitives::FLOAT, primitives::INT16) => Instruction::ConvertF32I16,
                    (primitives::FLOAT, primitives::INT32) => Instruction::ConvertF32I32,
                    (primitives::FLOAT, primitives::INT64) => Instruction::ConvertF32I64,
                    (primitives::DOUBLE, primitives::INT8) => Instruction::ConvertF64I8,
                    (primitives::DOUBLE, primitives::INT16) => Instruction::ConvertF64I16,
                    (primitives::DOUBLE, primitives::INT32) => Instruction::ConvertF64I32,
                    (primitives::DOUBLE, primitives::INT64) => Instruction::ConvertF64I64,

                    // Float to Unsigned conversions
                    (primitives::FLOAT, primitives::UINT8) => Instruction::ConvertF32U8,
                    (primitives::FLOAT, primitives::UINT16) => Instruction::ConvertF32U16,
                    (primitives::FLOAT, primitives::UINT32) => Instruction::ConvertF32U32,
                    (primitives::FLOAT, primitives::UINT64) => Instruction::ConvertF32U64,
                    (primitives::DOUBLE, primitives::UINT8) => Instruction::ConvertF64U8,
                    (primitives::DOUBLE, primitives::UINT16) => Instruction::ConvertF64U16,
                    (primitives::DOUBLE, primitives::UINT32) => Instruction::ConvertF64U32,
                    (primitives::DOUBLE, primitives::UINT64) => Instruction::ConvertF64U64,

                    // Float ↔ Double conversions
                    (primitives::FLOAT, primitives::DOUBLE) => Instruction::ConvertF32F64,
                    (primitives::DOUBLE, primitives::FLOAT) => Instruction::ConvertF64F32,

                    // Integer widening (signed)
                    (primitives::INT8, primitives::INT16) => Instruction::ConvertI8I16,
                    (primitives::INT8, primitives::INT32) => Instruction::ConvertI8I32,
                    (primitives::INT8, primitives::INT64) => Instruction::ConvertI8I64,
                    (primitives::INT16, primitives::INT32) => Instruction::ConvertI16I32,
                    (primitives::INT16, primitives::INT64) => Instruction::ConvertI16I64,
                    (primitives::INT32, primitives::INT64) => Instruction::ConvertI32I64,

                    // Integer narrowing (signed)
                    (primitives::INT64, primitives::INT32) => Instruction::ConvertI64I32,
                    (primitives::INT64, primitives::INT16) => Instruction::ConvertI64I16,
                    (primitives::INT64, primitives::INT8) => Instruction::ConvertI64I8,
                    (primitives::INT32, primitives::INT16) => Instruction::ConvertI32I16,
                    (primitives::INT32, primitives::INT8) => Instruction::ConvertI32I8,
                    (primitives::INT16, primitives::INT8) => Instruction::ConvertI16I8,

                    // Unsigned widening
                    (primitives::UINT8, primitives::UINT16) => Instruction::ConvertU8U16,
                    (primitives::UINT8, primitives::UINT32) => Instruction::ConvertU8U32,
                    (primitives::UINT8, primitives::UINT64) => Instruction::ConvertU8U64,
                    (primitives::UINT16, primitives::UINT32) => Instruction::ConvertU16U32,
                    (primitives::UINT16, primitives::UINT64) => Instruction::ConvertU16U64,
                    (primitives::UINT32, primitives::UINT64) => Instruction::ConvertU32U64,

                    // Unsigned narrowing
                    (primitives::UINT64, primitives::UINT32) => Instruction::ConvertU64U32,
                    (primitives::UINT64, primitives::UINT16) => Instruction::ConvertU64U16,
                    (primitives::UINT64, primitives::UINT8) => Instruction::ConvertU64U8,
                    (primitives::UINT32, primitives::UINT16) => Instruction::ConvertU32U16,
                    (primitives::UINT32, primitives::UINT8) => Instruction::ConvertU32U8,
                    (primitives::UINT16, primitives::UINT8) => Instruction::ConvertU16U8,

                    // Signed/Unsigned reinterpret
                    (primitives::INT8, primitives::UINT8) => Instruction::ConvertI8U8,
                    (primitives::INT16, primitives::UINT16) => Instruction::ConvertI16U16,
                    (primitives::INT32, primitives::UINT32) => Instruction::ConvertI32U32,
                    (primitives::INT64, primitives::UINT64) => Instruction::ConvertI64U64,
                    (primitives::UINT8, primitives::INT8) => Instruction::ConvertU8I8,
                    (primitives::UINT16, primitives::INT16) => Instruction::ConvertU16I16,
                    (primitives::UINT32, primitives::INT32) => Instruction::ConvertU32I32,
                    (primitives::UINT64, primitives::INT64) => Instruction::ConvertU64I64,

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
