use core::cmp;
use core::mem::size_of;

use linux_raw_sys::general;

use super::linux_abi::{
    DEFAULT_NOFILE_LIMIT, RLIMIT_NOFILE_RESOURCE, RLIMIT_STACK_RESOURCE, USER_STACK_SIZE,
};
use super::{UserProcess, task_context::current_tid};

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct UserRlimit {
    rlim_cur: u64,
    rlim_max: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct UserSchedParam {
    sched_priority: i32,
}

pub(super) fn default_rlimit(resource: u32) -> UserRlimit {
    match resource {
        RLIMIT_STACK_RESOURCE => UserRlimit {
            rlim_cur: USER_STACK_SIZE as u64,
            rlim_max: USER_STACK_SIZE as u64,
        },
        RLIMIT_NOFILE_RESOURCE => UserRlimit {
            rlim_cur: DEFAULT_NOFILE_LIMIT,
            rlim_max: DEFAULT_NOFILE_LIMIT,
        },
        _ => UserRlimit {
            rlim_cur: u64::MAX,
            rlim_max: u64::MAX,
        },
    }
}

pub(super) fn rlimit_is_valid(limit: UserRlimit) -> bool {
    limit.rlim_cur <= limit.rlim_max
}

pub(super) fn prlimit_target_valid(pid: i32) -> bool {
    pid == 0 || pid == current_tid()
}

pub(super) fn default_sched_param() -> UserSchedParam {
    UserSchedParam { sched_priority: 0 }
}

pub(super) fn sched_param_accepts_setparam(param: UserSchedParam) -> bool {
    param.sched_priority == 0
}

pub(super) fn sched_param_accepts_policy(policy: i32, param: UserSchedParam) -> bool {
    match policy as u32 {
        0 if param.sched_priority == 0 => true,
        general::SCHED_FIFO | general::SCHED_RR if (1..=99).contains(&param.sched_priority) => true,
        general::SCHED_BATCH | general::SCHED_IDLE if param.sched_priority == 0 => true,
        _ => false,
    }
}

pub(super) fn is_same_sched_target(process: &UserProcess, pid: i32) -> bool {
    pid == 0 || pid == current_tid() || pid == process.pid()
}

pub(super) fn sched_affinity_accepts_current_cpu(first_mask_byte: u8) -> bool {
    first_mask_byte & 1 != 0
}

pub(super) fn sched_affinity_result_len(cpusetsize: usize) -> usize {
    cmp::min(cpusetsize, size_of::<usize>())
}
