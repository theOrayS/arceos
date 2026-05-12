#[cfg(target_arch = "riscv64")]
use core::mem::{offset_of, size_of};

#[cfg(target_arch = "riscv64")]
use axhal::context::TrapFrame;

use axerrno::LinuxError;

#[cfg(target_arch = "riscv64")]
use super::linux_abi::{RISCV_SIGNAL_FPSTATE_BYTES, RISCV_SIGNAL_SIGSET_RESERVED_BYTES};

pub(super) fn validate_signal_target(sig: i32) -> Result<(), LinuxError> {
    if sig < 0 || sig > 64 {
        return Err(LinuxError::EINVAL);
    }
    Ok(())
}

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
pub(super) struct RiscvSignalFrameOffsets {
    pub(super) info: usize,
    pub(super) ucontext: usize,
    pub(super) trampoline: usize,
}

#[cfg(target_arch = "riscv64")]
pub(super) fn riscv_signal_frame_size() -> usize {
    size_of::<RiscvSignalFrame>()
}

#[cfg(target_arch = "riscv64")]
pub(super) fn riscv_signal_frame_offsets() -> RiscvSignalFrameOffsets {
    RiscvSignalFrameOffsets {
        info: offset_of!(RiscvSignalFrame, info),
        ucontext: offset_of!(RiscvSignalFrame, ucontext),
        trampoline: offset_of!(RiscvSignalFrame, trampoline),
    }
}

#[cfg(target_arch = "riscv64")]
pub(super) fn trap_frame_to_riscv_sigcontext(tf: &TrapFrame) -> RiscvSignalSigcontext {
    RiscvSignalSigcontext {
        gregs: [
            tf.sepc,
            tf.regs.ra,
            tf.regs.sp,
            tf.regs.gp,
            tf.regs.tp,
            tf.regs.t0,
            tf.regs.t1,
            tf.regs.t2,
            tf.regs.s0,
            tf.regs.s1,
            tf.regs.a0,
            tf.regs.a1,
            tf.regs.a2,
            tf.regs.a3,
            tf.regs.a4,
            tf.regs.a5,
            tf.regs.a6,
            tf.regs.a7,
            tf.regs.s2,
            tf.regs.s3,
            tf.regs.s4,
            tf.regs.s5,
            tf.regs.s6,
            tf.regs.s7,
            tf.regs.s8,
            tf.regs.s9,
            tf.regs.s10,
            tf.regs.s11,
            tf.regs.t3,
            tf.regs.t4,
            tf.regs.t5,
            tf.regs.t6,
        ],
        fpstate: RiscvSignalFpState {
            bytes: [0; RISCV_SIGNAL_FPSTATE_BYTES],
        },
    }
}

#[cfg(target_arch = "riscv64")]
pub(super) fn apply_riscv_sigcontext(tf: &mut TrapFrame, sigcontext: &RiscvSignalSigcontext) {
    tf.sepc = sigcontext.gregs[0];
    tf.regs.zero = 0;
    tf.regs.ra = sigcontext.gregs[1];
    tf.regs.sp = sigcontext.gregs[2];
    tf.regs.gp = sigcontext.gregs[3];
    tf.regs.tp = sigcontext.gregs[4];
    tf.regs.t0 = sigcontext.gregs[5];
    tf.regs.t1 = sigcontext.gregs[6];
    tf.regs.t2 = sigcontext.gregs[7];
    tf.regs.s0 = sigcontext.gregs[8];
    tf.regs.s1 = sigcontext.gregs[9];
    tf.regs.a0 = sigcontext.gregs[10];
    tf.regs.a1 = sigcontext.gregs[11];
    tf.regs.a2 = sigcontext.gregs[12];
    tf.regs.a3 = sigcontext.gregs[13];
    tf.regs.a4 = sigcontext.gregs[14];
    tf.regs.a5 = sigcontext.gregs[15];
    tf.regs.a6 = sigcontext.gregs[16];
    tf.regs.a7 = sigcontext.gregs[17];
    tf.regs.s2 = sigcontext.gregs[18];
    tf.regs.s3 = sigcontext.gregs[19];
    tf.regs.s4 = sigcontext.gregs[20];
    tf.regs.s5 = sigcontext.gregs[21];
    tf.regs.s6 = sigcontext.gregs[22];
    tf.regs.s7 = sigcontext.gregs[23];
    tf.regs.s8 = sigcontext.gregs[24];
    tf.regs.s9 = sigcontext.gregs[25];
    tf.regs.s10 = sigcontext.gregs[26];
    tf.regs.s11 = sigcontext.gregs[27];
    tf.regs.t3 = sigcontext.gregs[28];
    tf.regs.t4 = sigcontext.gregs[29];
    tf.regs.t5 = sigcontext.gregs[30];
    tf.regs.t6 = sigcontext.gregs[31];
}

#[cfg(target_arch = "riscv64")]
fn make_riscv_siginfo(sig: i32, code: i32, tid: i32) -> RiscvSignalInfo {
    let mut info = RiscvSignalInfo { bytes: [0; 128] };
    info.bytes[0..4].copy_from_slice(&sig.to_ne_bytes());
    info.bytes[4..8].copy_from_slice(&0i32.to_ne_bytes());
    info.bytes[8..12].copy_from_slice(&code.to_ne_bytes());
    info.bytes[16..20].copy_from_slice(&tid.to_ne_bytes());
    info.bytes[20..24].copy_from_slice(&0u32.to_ne_bytes());
    info
}

#[cfg(target_arch = "riscv64")]
pub(super) fn make_riscv_signal_frame(
    sig: i32,
    code: i32,
    tid: i32,
    current_mask: u64,
    stack_flags: i32,
    trampoline: [u32; 3],
    mcontext: RiscvSignalSigcontext,
) -> RiscvSignalFrame {
    RiscvSignalFrame {
        info: make_riscv_siginfo(sig, code, tid),
        ucontext: RiscvSignalUcontext {
            flags: 0,
            link: 0,
            stack: RiscvSignalStack {
                sp: 0,
                stack_flags,
                stack_pad: 0,
                size: 0,
            },
            sigmask: RiscvKernelSigset {
                sig: [current_mask],
                reserved: [0; RISCV_SIGNAL_SIGSET_RESERVED_BYTES],
            },
            mcontext,
        },
        trampoline,
    }
}

#[cfg(target_arch = "riscv64")]
const _: [(); RISCV_SIGNAL_FPSTATE_BYTES] = [(); size_of::<RiscvSignalFpState>()];
#[cfg(target_arch = "riscv64")]
const _: [(); 784] = [(); size_of::<RiscvSignalSigcontext>()];
#[cfg(target_arch = "riscv64")]
const _: [(); 960] = [(); size_of::<RiscvSignalUcontext>()];
#[cfg(target_arch = "riscv64")]
const _: [(); 1104] = [(); size_of::<RiscvSignalFrame>()];
