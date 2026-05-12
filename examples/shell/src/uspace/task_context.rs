use core::sync::atomic::{AtomicI32, AtomicU64, AtomicUsize, Ordering};

use axerrno::LinuxError;
use axhal::context::TrapFrame;
use axsync::Mutex;
use axtask::AxTaskRef;
use std::sync::Arc;

use super::UserProcess;

pub(super) struct UserTaskExt {
    pub(super) process: Arc<UserProcess>,
    pub(super) clear_child_tid: AtomicUsize,
    pub(super) pending_signal: AtomicI32,
    pub(super) signal_mask: AtomicU64,
    pub(super) futex_wait: AtomicUsize,
    pub(super) robust_list_head: AtomicUsize,
    pub(super) robust_list_len: AtomicUsize,
    pub(super) deferred_unmap_start: AtomicUsize,
    pub(super) deferred_unmap_len: AtomicUsize,
    pub(super) signal_frame: AtomicUsize,
    pub(super) pending_sigreturn: Mutex<Option<TrapFrame>>,
}

impl UserTaskExt {
    pub(super) fn new(process: Arc<UserProcess>, clear_child_tid: usize, signal_mask: u64) -> Self {
        Self {
            process,
            clear_child_tid: AtomicUsize::new(clear_child_tid),
            pending_signal: AtomicI32::new(0),
            signal_mask: AtomicU64::new(signal_mask),
            futex_wait: AtomicUsize::new(0),
            robust_list_head: AtomicUsize::new(0),
            robust_list_len: AtomicUsize::new(0),
            deferred_unmap_start: AtomicUsize::new(0),
            deferred_unmap_len: AtomicUsize::new(0),
            signal_frame: AtomicUsize::new(0),
            pending_sigreturn: Mutex::new(None),
        }
    }
}

axtask::def_task_ext!(UserTaskExt);

pub(super) fn current_process() -> Option<Arc<UserProcess>> {
    let ext = current_task_ext()?;
    Some(ext.process.clone())
}

pub(super) fn current_task_ext() -> Option<&'static UserTaskExt> {
    let curr = axtask::current_may_uninit()?;
    let ptr = unsafe { curr.task_ext_ptr() };
    if ptr.is_null() {
        return None;
    }
    let ext = unsafe { &*(ptr as *const UserTaskExt) };
    Some(ext)
}

pub(super) fn task_ext(task: &AxTaskRef) -> Option<&UserTaskExt> {
    let ptr = unsafe { task.task_ext_ptr() };
    if ptr.is_null() {
        return None;
    }
    Some(unsafe { &*(ptr as *const UserTaskExt) })
}

pub(super) fn set_current_robust_list(head: usize, len: usize) -> Result<(), LinuxError> {
    let Some(ext) = current_task_ext() else {
        return Err(LinuxError::EINVAL);
    };
    ext.robust_list_head.store(head, Ordering::Release);
    ext.robust_list_len.store(len, Ordering::Release);
    Ok(())
}

pub(super) fn robust_list_for_task(task: &AxTaskRef) -> Option<(usize, usize)> {
    let ext = task_ext(task)?;
    Some((
        ext.robust_list_head.load(Ordering::Acquire),
        ext.robust_list_len.load(Ordering::Acquire),
    ))
}

pub(super) fn current_tid() -> i32 {
    axtask::current().id().as_u64() as i32
}
