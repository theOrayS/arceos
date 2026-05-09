#[cfg(target_arch = "riscv64")]
use core::mem::size_of;

#[cfg(target_arch = "riscv64")]
use super::linux_abi::{RISCV_SIGNAL_FPSTATE_BYTES, RISCV_SIGNAL_SIGSET_RESERVED_BYTES};

#[cfg(target_arch = "riscv64")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct RiscvSignalInfo {
    pub(super) bytes: [u8; 128],
}

#[cfg(target_arch = "riscv64")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct RiscvSignalStack {
    pub(super) sp: usize,
    pub(super) stack_flags: i32,
    pub(super) stack_pad: i32,
    pub(super) size: usize,
}

#[cfg(target_arch = "riscv64")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct RiscvKernelSigset {
    pub(super) sig: [u64; 1],
    pub(super) reserved: [u8; RISCV_SIGNAL_SIGSET_RESERVED_BYTES],
}

#[cfg(target_arch = "riscv64")]
#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub(super) struct RiscvSignalFpState {
    pub(super) bytes: [u8; RISCV_SIGNAL_FPSTATE_BYTES],
}

#[cfg(target_arch = "riscv64")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct RiscvSignalSigcontext {
    pub(super) gregs: [usize; 32],
    pub(super) fpstate: RiscvSignalFpState,
}

#[cfg(target_arch = "riscv64")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct RiscvSignalUcontext {
    pub(super) flags: usize,
    pub(super) link: usize,
    pub(super) stack: RiscvSignalStack,
    pub(super) sigmask: RiscvKernelSigset,
    pub(super) mcontext: RiscvSignalSigcontext,
}

#[cfg(target_arch = "riscv64")]
#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub(super) struct RiscvSignalFrame {
    pub(super) info: RiscvSignalInfo,
    pub(super) ucontext: RiscvSignalUcontext,
    pub(super) trampoline: [u32; 3],
}

#[cfg(target_arch = "riscv64")]
const _: [(); RISCV_SIGNAL_FPSTATE_BYTES] = [(); size_of::<RiscvSignalFpState>()];
#[cfg(target_arch = "riscv64")]
const _: [(); 784] = [(); size_of::<RiscvSignalSigcontext>()];
#[cfg(target_arch = "riscv64")]
const _: [(); 960] = [(); size_of::<RiscvSignalUcontext>()];
#[cfg(target_arch = "riscv64")]
const _: [(); 1104] = [(); size_of::<RiscvSignalFrame>()];
