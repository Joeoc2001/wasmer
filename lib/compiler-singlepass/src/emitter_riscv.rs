//! RISC-V emitter scaffolding.

use crate::{
    codegen_error, common_decl::Size, location::Location as AbstractLocation,
    machine_riscv::AssemblerRiscv,
};
pub use crate::{
    location::Multiplier,
    machine::{Label, Offset},
    riscv_decl::{FPR, GPR},
};
use dynasm::dynasm;
use dynasmrt::{AssemblyOffset, DynamicLabel, DynasmApi, DynasmLabelApi};
use wasmer_types::{target::CpuFeature, CompileError};

/// Force `dynasm!` to use the correct arch (riscv64) when cross-compiling.
macro_rules! dynasm {
    ($a:expr ; $($tt:tt)*) => {
        dynasm::dynasm!(
            $a.inner
            ; .arch riscv64
            ; $($tt)*
        )
    };
}

/// Location abstraction specialized to RISC-V.
pub type Location = AbstractLocation<GPR, FPR>;

/// Branch conditions for RISC-V.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Condition {
    // TODO: define RISC-V branch conditions.
}

/// Emitter trait for RISC-V.
#[allow(unused)]
pub trait EmitterRiscv {
    /*/// Returns the SIMD (FPU) feature if available.
    fn get_simd_arch(&self) -> Option<&CpuFeature>;
    /// Returns the size of a jump instruction in bytes.
    fn get_jmp_instr_size(&self) -> u8;

    /// Finalize the function, e.g., resolve labels.
    fn finalize_function(&mut self) -> Result<(), CompileError>;

    // TODO: add methods for emitting RISC-V instructions (e.g., loads, stores, arithmetic, branches, etc.)*/

    /// Generates a new internal label.
    fn get_label(&mut self) -> Label;
    /// Gets the current code offset.
    fn get_offset(&self) -> Offset;

    fn emit_label(&mut self, label: Label) -> Result<(), CompileError>;
    fn emit_ret(&mut self) -> Result<(), CompileError>;
    fn emit_add(
        &mut self,
        sz: Size,
        src: GPR,
        value: Location,
        dst: GPR,
    ) -> Result<(), CompileError>;
    fn emit_mov(&mut self, sz: Size, src: Location, dst: Location) -> Result<(), CompileError>;
    fn emit_call_location(&mut self, loc: Location) -> Result<(), CompileError>;
    fn emit_break(&mut self) -> Result<(), CompileError>;
}

impl EmitterRiscv for AssemblerRiscv {
    fn get_offset(&self) -> AssemblyOffset {
        self.offset()
    }

    fn get_label(&mut self) -> DynamicLabel {
        self.new_dynamic_label()
    }

    fn emit_label(&mut self, label: Label) -> Result<(), CompileError> {
        dynasm!(self ; => label);
        Ok(())
    }

    fn emit_ret(&mut self) -> Result<(), CompileError> {
        dynasm!(self ; ret);
        Ok(())
    }

    fn emit_add(
        &mut self,
        sz: Size,
        src: GPR,
        value: Location,
        dst: GPR,
    ) -> Result<(), CompileError> {
        fn emit_addi(
            a: &mut AssemblerRiscv,
            sz: Size,
            src: GPR,
            imm: i64,
            dst: GPR,
        ) -> Result<(), CompileError> {
            // Fast path
            if imm == 0 {
                return Ok(());
            }

            if imm >= -2048 && imm < 2047 {
                // Single instr path
                let imm = imm as i32;
                match sz {
                    Size::S32 => dynasm!(a ; addiw X(dst as u8), X(src as u8), imm),
                    Size::S64 => dynasm!(a ; addi X(dst as u8), X(src as u8), imm),
                    _ => codegen_error!("singlepass can't emit ADDI with size {:?}", sz,),
                }
            } else {
                dynasm!(a ; li X(GPR::T6 as u8), imm);
                match sz {
                    Size::S32 => dynasm!(a ; addw X(dst as u8), X(src as u8), X(GPR::T6 as u8)),
                    Size::S64 => dynasm!(a ; add X(dst as u8), X(src as u8), X(GPR::T6 as u8)),
                    _ => codegen_error!("singlepass can't emit ADDI with size {:?}", sz,),
                }
            }

            Ok(())
        }

        match value {
            AbstractLocation::Imm32(imm) => emit_addi(self, sz, src, imm as i32 as i64, dst)?,
            AbstractLocation::Imm64(imm) => emit_addi(self, sz, src, imm as i64, dst)?,
            AbstractLocation::GPR(gpr) => match sz {
                Size::S32 => dynasm!(self ; addw X(dst as u8), X(src as u8), X(gpr as u8)),
                Size::S64 => dynasm!(self ; add X(dst as u8), X(src as u8), X(gpr as u8)),
                _ => codegen_error!("singlepass can't emit ADD GPRs with size {:?}", sz,),
            },
            _ => codegen_error!(
                "singlepass can't emit ADD {:?} {:?} {:?} {:?}",
                sz,
                src,
                value,
                dst
            ),
        }

        Ok(())
    }

    fn emit_mov(&mut self, sz: Size, src: Location, dst: Location) -> Result<(), CompileError> {
        if src == dst {
            return Ok(());
        }

        match (sz, src, dst) {
            // GPR -> GPR
            (Size::S32, Location::GPR(src), Location::GPR(dst)) => {
                dynasm!(self ; addw X(dst as u8), X(src as u8), x0);
            }
            (Size::S64, Location::GPR(src), Location::GPR(dst)) => {
                dynasm!(self ; mv X(dst as u8), X(src as u8));
            }
            // Immediate -> GPR
            (Size::S32, Location::Imm32(imm), Location::GPR(dst)) => {
                let imm = (imm as i32) as i64;
                dynasm!(self ; li X(dst as u8), imm);
            }
            (Size::S64, Location::Imm32(imm), Location::GPR(dst)) => {
                let imm = (imm as i32) as i64;
                dynasm!(self ; li X(dst as u8), imm);
            }
            (Size::S32, Location::Imm64(imm), Location::GPR(dst)) => {
                let imm = imm as i64 as i32 as i64;
                dynasm!(self ; li X(dst as u8), imm);
            }
            (Size::S64, Location::Imm64(imm), Location::GPR(dst)) => {
                let imm = imm as i64;
                dynasm!(self ; li X(dst as u8), imm);
            }
            // Immediate -> Memory
            (Size::S32, Location::Imm32(imm), Location::Memory(dst, offset)) => {
                let imm = (imm as i32) as i64;
                dynasm!(self ; li X(GPR::T6 as u8), imm);
                dynasm!(self ; sw X(GPR::T6 as u8), [X(dst as u8), offset]);
            }
            (Size::S64, Location::Imm32(imm), Location::Memory(dst, offset)) => {
                let imm = (imm as i32) as i64;
                dynasm!(self ; li X(GPR::T6 as u8), imm);
                dynasm!(self ; sd X(GPR::T6 as u8), [X(dst as u8), offset]);
            }
            (Size::S32, Location::Imm64(imm), Location::Memory(dst, offset)) => {
                let imm = imm as i64 as i32 as i64;
                dynasm!(self ; li X(GPR::T6 as u8), imm);
                dynasm!(self ; sw X(GPR::T6 as u8), [X(dst as u8), offset]);
            }
            (Size::S64, Location::Imm64(imm), Location::Memory(dst, offset)) => {
                let imm = imm as i64;
                dynasm!(self ; li X(GPR::T6 as u8), imm);
                dynasm!(self ; sd X(GPR::T6 as u8), [X(dst as u8), offset]);
            }
            // GPR -> Memory
            (Size::S32, Location::GPR(src), Location::Memory(dst, offset)) => {
                dynasm!(self ; sw X(src as u8), [X(dst as u8), offset]);
            }
            (Size::S64, Location::GPR(src), Location::Memory(dst, offset)) => {
                dynasm!(self ; sd X(src as u8), [X(dst as u8), offset]);
            }
            // Memory -> GPR
            (Size::S32, Location::Memory(src, offset), Location::GPR(dst)) => {
                dynasm!(self ; lw X(dst as u8), [X(src as u8), offset]);
            }
            (Size::S64, Location::Memory(src, offset), Location::GPR(dst)) => {
                dynasm!(self ; ld X(dst as u8), [X(src as u8), offset]);
            }
            _ => codegen_error!("singlepass can't emit MOV {:?} {:?} {:?}", sz, src, dst),
        }

        Ok(())
    }

    fn emit_call_location(&mut self, loc: Location) -> Result<(), CompileError> {
        let reg = match loc {
            AbstractLocation::GPR(reg) => reg,
            AbstractLocation::Memory(src, offset) => {
                dynasm!(self ; ld X(GPR::T6 as u8), [X(src as u8), offset]);
                GPR::T6
            }
            _ => codegen_error!("singlepass can't emit CALL LOC {:?}", loc),
        };
        dynasm!(self ; jalr x1, X(reg as u8), 0);
        Ok(())
    }

    fn emit_break(&mut self) -> Result<(), CompileError> {
        dynasm!(self ; ebreak);
        Ok(())
    }
}
