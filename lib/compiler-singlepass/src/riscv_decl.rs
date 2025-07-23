//! RISC-V structures.

use crate::{
    common_decl::{MachineState, MachineValue, RegisterIndex},
    location::{CombinedRegister, Reg as AbstractReg},
};
use std::{collections::BTreeMap, slice::Iter};
use wasmer_types::target::CallingConvention;
use wasmer_types::{CompileError, Type};

/// General-purpose registers.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[allow(clippy::upper_case_acronyms)]
pub enum GPR {
    Zero,
    Ra,
    Sp,
    Gp,
    Tp,
    T0,
    T1,
    T2,
    /// AKA S0
    Fp,
    S1,
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    A7,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
    S8,
    S9,
    S10,
    S11,
    T3,
    T4,
    T5,
    T6,
}

/// Floating-point registers.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[allow(clippy::upper_case_acronyms)]
pub enum FPR {
    // TODO: define floating-point registers F0-F31.
}

impl AbstractReg for GPR {
    fn is_callee_save(self) -> bool {
        matches!(
            self,
            Self::Sp
                | Self::Fp
                | Self::S1
                | Self::S2
                | Self::S3
                | Self::S4
                | Self::S5
                | Self::S6
                | Self::S7
                | Self::S8
                | Self::S9
                | Self::S10
                | Self::S11
        )
    }
    fn is_reserved(self) -> bool {
        matches!(
            self,
            Self::Zero | Self::Ra | Self::Sp | Self::Gp | Self::Tp | Self::Fp
        )
    }
    fn into_index(self) -> usize {
        self as usize
    }
    fn from_index(n: usize) -> Result<GPR, ()> {
        match n {
            0..=31 => Ok(*Self::iterator().nth(n).unwrap()),
            _ => Err(()),
        }
    }
    fn iterator() -> Iter<'static, GPR> {
        const GPRS: [GPR; 32] = [
            GPR::Zero,
            GPR::Ra,
            GPR::Sp,
            GPR::Gp,
            GPR::Tp,
            GPR::T0,
            GPR::T1,
            GPR::T2,
            GPR::Fp,
            GPR::S1,
            GPR::A0,
            GPR::A1,
            GPR::A2,
            GPR::A3,
            GPR::A4,
            GPR::A5,
            GPR::A6,
            GPR::A7,
            GPR::S2,
            GPR::S3,
            GPR::S4,
            GPR::S5,
            GPR::S6,
            GPR::S7,
            GPR::S8,
            GPR::S9,
            GPR::S10,
            GPR::S11,
            GPR::T3,
            GPR::T4,
            GPR::T5,
            GPR::T6,
        ];
        GPRS.iter()
    }
    fn to_dwarf(self) -> u16 {
        // TODO: map register to DWARF register number.
        todo!()
    }
}

impl AbstractReg for FPR {
    fn is_callee_save(self) -> bool {
        // TODO: implement callee-save registers for FPR.
        todo!()
    }
    fn is_reserved(self) -> bool {
        // TODO: implement reserved floating-point registers.
        todo!()
    }
    fn into_index(self) -> usize {
        self as usize
    }
    fn from_index(n: usize) -> Result<FPR, ()> {
        // TODO: map index to FPR.
        todo!()
    }
    fn iterator() -> Iter<'static, FPR> {
        // TODO: return an iterator over all FPR variants.
        todo!()
    }
    fn to_dwarf(self) -> u16 {
        // TODO: map FPR register to DWARF register number.
        todo!()
    }
}

/// A combined RISC-V register.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RiscvRegister {
    /// General-purpose register.
    GPR(GPR),
    /// Floating-point register.
    FPR(FPR),
}

impl CombinedRegister for RiscvRegister {
    fn to_index(&self) -> RegisterIndex {
        match *self {
            RiscvRegister::GPR(x) => RegisterIndex(x as usize),
            RiscvRegister::FPR(x) => RegisterIndex(x as usize + /* FPR offset */ 0),
        }
    }
    fn from_gpr(x: u16) -> Self {
        RiscvRegister::GPR(GPR::from_index(x as usize).unwrap())
    }
    fn from_simd(x: u16) -> Self {
        RiscvRegister::FPR(FPR::from_index(x as usize).unwrap())
    }
    fn _from_dwarf_regnum(x: u16) -> Option<Self> {
        // TODO: map DWARF register number to RiscvRegister
        None
    }
}

/// Allocator for function argument registers according to the RISC-V ABI.
#[derive(Default)]
pub struct ArgumentRegisterAllocator {
    // TODO: track next GPR/FPR for argument passing.
}

impl ArgumentRegisterAllocator {
    /// Allocates a register for argument type `ty`. Returns `None` if no register is available.
    pub fn next(
        &mut self,
        ty: Type,
        calling_convention: CallingConvention,
    ) -> Result<Option<RiscvRegister>, CompileError> {
        // TODO: implement RISC-V calling convention register allocation.
        todo!()
    }
}

/// Create a new `MachineState` with default values for RISC-V.
pub fn new_machine_state() -> MachineState {
    MachineState {
        stack_values: vec![],
        register_values: vec![MachineValue::Undefined; /* GPR+FPR count */ 0],
        prev_frame: BTreeMap::new(),
        wasm_stack: vec![],
        wasm_inst_offset: usize::MAX,
    }
}
