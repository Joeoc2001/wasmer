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
        lhs: Location,
        rhs: Location,
        dst: Location,
    ) -> Result<(), CompileError>;
    fn emit_movzx(&mut self, sz: Size, src: Location, dst: Location) -> Result<(), CompileError>;
    fn emit_movsx(&mut self, sz: Size, src: Location, dst: Location) -> Result<(), CompileError>;
    fn emit_call_location(&mut self, loc: Location) -> Result<(), CompileError>;
    fn emit_break(&mut self) -> Result<(), CompileError>;

    /// Finalize the function, e.g., resolve labels.
    fn finalize_function(&mut self) -> Result<(), CompileError>;
}

impl AssemblerRiscv {
    fn emit_addi(&mut self, sz: Size, src: GPR, imm: i64, dst: GPR) -> Result<(), CompileError> {
        // Fast path
        if imm == 0 && src == dst {
            return Ok(());
        }

        if imm >= -2048 && imm < 2047 {
            // Single instr path
            let imm = imm as i32;
            match sz {
                Size::S32 => dynasm!(self ; addiw X(dst as u8), X(src as u8), imm),
                Size::S64 => dynasm!(self ; addi X(dst as u8), X(src as u8), imm),
                _ => codegen_error!("singlepass can't emit ADDI with size {:?}", sz),
            }
        } else {
            dynasm!(self ; li X(GPR::T6 as u8), imm);
            self.emit_add(sz, GPR::T6, src, dst)?;
        }

        Ok(())
    }

    fn emit_add(&mut self, sz: Size, lhs: GPR, rhs: GPR, dst: GPR) -> Result<(), CompileError> {
        match sz {
            Size::S32 => dynasm!(self ; addw X(dst as u8), X(lhs as u8), X(rhs as u8)),
            Size::S64 => dynasm!(self ; add X(dst as u8), X(lhs as u8), X(rhs as u8)),
            _ => codegen_error!("singlepass can't emit ADD GPRs with size {:?}", sz,),
        }

        Ok(())
    }

    fn emit_load_ze(
        &mut self,
        sz: Size,
        mem: GPR,
        offset: i32,
        dst: GPR,
    ) -> Result<(), CompileError> {
        match sz {
            Size::S32 => dynasm!(self ; lwu X(dst as u8), [X(mem as u8), offset]),
            Size::S64 => dynasm!(self ; ld X(dst as u8), [X(mem as u8), offset]),
            _ => codegen_error!("singlepass can't emit load zero extend with size {:?}", sz),
        }

        Ok(())
    }

    fn emit_load_se(
        &mut self,
        sz: Size,
        mem: GPR,
        offset: i32,
        dst: GPR,
    ) -> Result<(), CompileError> {
        match sz {
            Size::S32 => dynasm!(self ; lw X(dst as u8), [X(mem as u8), offset]),
            Size::S64 => dynasm!(self ; ld X(dst as u8), [X(mem as u8), offset]),
            _ => codegen_error!("singlepass can't emit load sign extend with size {:?}", sz),
        }

        Ok(())
    }

    fn emit_store(
        &mut self,
        sz: Size,
        src: GPR,
        mem: GPR,
        offset: i32,
    ) -> Result<(), CompileError> {
        match sz {
            Size::S32 => dynasm!(self ; sw X(src as u8), [X(mem as u8), offset]),
            Size::S64 => dynasm!(self ; sd X(src as u8), [X(mem as u8), offset]),
            _ => codegen_error!("singlepass can't emit store with size {:?}", sz),
        }

        Ok(())
    }
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
        lhs: Location,
        rhs: Location,
        dst: Location,
    ) -> Result<(), CompileError> {
        let r_dest = match dst {
            Location::GPR(r_dest) => r_dest,
            Location::Memory(..) => GPR::T5,
            _ => codegen_error!("singlepass can't emit ADD with destination {:?}", dst),
        };

        match (lhs, rhs) {
            // Imm + GPR
            (Location::Imm32(imm), Location::GPR(src))
            | (Location::GPR(src), Location::Imm32(imm)) => {
                self.emit_addi(sz, src, imm as i32 as i64, r_dest)?
            }
            (Location::Imm64(imm), Location::GPR(src))
            | (Location::GPR(src), Location::Imm64(imm)) => {
                self.emit_addi(sz, src, imm as i64, r_dest)?
            }
            // GPR + GPR
            (Location::GPR(lhs), Location::GPR(rhs)) => self.emit_add(sz, lhs, rhs, r_dest)?,
            // Mem + Imm
            (Location::Imm32(imm), Location::Memory(mem, offset))
            | (Location::Memory(mem, offset), Location::Imm32(imm)) => {
                self.emit_load_ze(sz, mem, offset, GPR::T5)?;
                self.emit_addi(sz, GPR::T5, imm as i32 as i64, r_dest)?
            }
            (Location::Imm64(imm), Location::Memory(mem, offset))
            | (Location::Memory(mem, offset), Location::Imm64(imm)) => {
                self.emit_load_ze(sz, mem, offset, GPR::T5)?;
                self.emit_addi(sz, GPR::T5, imm as i64, r_dest)?
            }
            // Mem + GPR
            (Location::Memory(mem, offset), Location::GPR(src))
            | (Location::GPR(src), Location::Memory(mem, offset)) => {
                self.emit_load_ze(sz, mem, offset, GPR::T5)?;
                self.emit_add(sz, GPR::T5, src, r_dest)?
            }
            // Mem + Mem
            (Location::Memory(mem1, offset1), Location::Memory(mem2, offset2)) => {
                self.emit_load_ze(sz, mem1, offset1, GPR::T5)?;
                self.emit_load_ze(sz, mem2, offset2, GPR::T6)?;
                self.emit_add(sz, GPR::T5, GPR::T6, r_dest)?
            }
            _ => codegen_error!(
                "singlepass can't emit ADD {:?} {:?} {:?} {:?}",
                sz,
                lhs,
                rhs,
                dst
            ),
        }

        match dst {
            Location::Memory(dst, offset) => self.emit_store(sz, r_dest, dst, offset)?,
            _ => {}
        };

        Ok(())
    }

    fn emit_movzx(&mut self, sz: Size, src: Location, dst: Location) -> Result<(), CompileError> {
        if src == dst {
            return Ok(());
        }

        fn emit_movzx_inner(
            a: &mut AssemblerRiscv,
            sz: Size,
            src: Location,
            dst: Location,
            outer: bool,
        ) -> Result<(), CompileError> {
            match (sz, src, dst) {
                // GPR -> GPR
                (Size::S32, Location::GPR(src), Location::GPR(dst)) => {
                    dynasm!(a ; ld X(GPR::T6 as u8), >const_u32_max);
                    dynasm!(a ; and X(dst as u8), X(src as u8), X(GPR::T6 as u8));
                }
                (Size::S64, Location::GPR(src), Location::GPR(dst)) => {
                    dynasm!(a ; mv X(dst as u8), X(src as u8));
                }
                // Immediate -> GPR
                (_, Location::Imm32(imm), Location::GPR(dst)) => {
                    let imm = (imm as u64) as i64;
                    dynasm!(a ; li X(dst as u8), imm);
                }
                (Size::S32, Location::Imm64(imm), Location::GPR(dst)) => {
                    let imm = imm as u32 as i64;
                    dynasm!(a ; li X(dst as u8), imm);
                }
                (Size::S64, Location::Imm64(imm), Location::GPR(dst)) => {
                    let imm = imm as i64;
                    dynasm!(a ; li X(dst as u8), imm);
                }
                // GPR -> Memory
                (_, Location::GPR(src), Location::Memory(dst, offset)) => {
                    a.emit_store(sz, src, dst, offset)?
                }
                // Memory -> GPR
                (_, Location::Memory(src, offset), Location::GPR(dst)) => {
                    a.emit_load_ze(sz, src, offset, dst)?
                }
                // Try to compose two moves (for example for Mem -> Mem)
                _ => {
                    if outer {
                        emit_movzx_inner(a, sz, src, Location::GPR(GPR::T6), false)?;
                        emit_movzx_inner(a, sz, Location::GPR(GPR::T6), dst, false)?;
                    } else {
                        codegen_error!("singlepass can't emit movzx {:?} {:?} {:?}", sz, src, dst)
                    }
                }
            }

            Ok(())
        }

        emit_movzx_inner(self, sz, src, dst, true)
    }

    fn emit_movsx(&mut self, sz: Size, src: Location, dst: Location) -> Result<(), CompileError> {
        if src == dst {
            return Ok(());
        }

        fn emit_movsx_inner(
            a: &mut AssemblerRiscv,
            sz: Size,
            src: Location,
            dst: Location,
            outer: bool,
        ) -> Result<(), CompileError> {
            match (sz, src, dst) {
                // GPR -> GPR
                (Size::S32, Location::GPR(src), Location::GPR(dst)) => {
                    dynasm!(a ; addw X(dst as u8), X(src as u8), x0);
                }
                (Size::S64, Location::GPR(src), Location::GPR(dst)) => {
                    dynasm!(a ; mv X(dst as u8), X(src as u8));
                }
                // Immediate -> GPR
                (Size::S32, Location::Imm32(imm), Location::GPR(dst)) => {
                    let imm = (imm as i32) as i64;
                    dynasm!(a ; li X(dst as u8), imm);
                }
                (Size::S64, Location::Imm32(imm), Location::GPR(dst)) => {
                    let imm = (imm as i32) as i64;
                    dynasm!(a ; li X(dst as u8), imm);
                }
                (Size::S32, Location::Imm64(imm), Location::GPR(dst)) => {
                    let imm = imm as i64 as i32 as i64;
                    dynasm!(a ; li X(dst as u8), imm);
                }
                (Size::S64, Location::Imm64(imm), Location::GPR(dst)) => {
                    let imm = imm as i64;
                    dynasm!(a ; li X(dst as u8), imm);
                }
                // GPR -> Memory
                (_, Location::GPR(src), Location::Memory(dst, offset)) => {
                    a.emit_store(sz, src, dst, offset)?
                }
                // Memory -> GPR
                (_, Location::Memory(src, offset), Location::GPR(dst)) => {
                    a.emit_load_se(sz, src, offset, dst)?
                }
                // Try to compose two moves (for example for Mem -> Mem)
                _ => {
                    if outer {
                        emit_movsx_inner(a, sz, src, Location::GPR(GPR::T6), false)?;
                        emit_movsx_inner(a, sz, Location::GPR(GPR::T6), dst, false)?;
                    } else {
                        codegen_error!("singlepass can't emit movzx {:?} {:?} {:?}", sz, src, dst)
                    }
                }
            }

            Ok(())
        }

        emit_movsx_inner(self, sz, src, dst, true)
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

    fn finalize_function(&mut self) -> Result<(), CompileError> {
        dynasm!(
            self
            ; const_u32_max:
            ; .i64 0xFFFFFFFFi64
        );

        Ok(())
    }
}
