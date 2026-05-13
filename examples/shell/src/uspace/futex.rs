use core::sync::atomic::{AtomicU32, Ordering};

use axsync::Mutex;
use axtask::{AxTaskRef, WaitQueue};
use lazyinit::LazyInit;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(super) struct FutexState {
    pub(super) seq: AtomicU32,
    pub(super) queue: WaitQueue,
}

fn table() -> &'static Mutex<BTreeMap<usize, Arc<FutexState>>> {
    static FUTEXES: LazyInit<Mutex<BTreeMap<usize, Arc<FutexState>>>> = LazyInit::new();
    if !FUTEXES.is_inited() {
        FUTEXES.init_once(Mutex::new(BTreeMap::new()));
    }
    &FUTEXES
}

pub(super) fn state(uaddr: usize) -> Arc<FutexState> {
    let mut table = table().lock();
    table
        .entry(uaddr)
        .or_insert_with(|| {
            Arc::new(FutexState {
                seq: AtomicU32::new(0),
                queue: WaitQueue::new(),
            })
        })
        .clone()
}

pub(super) fn wake_addr(uaddr: usize, count: usize) -> usize {
    let Some(state) = table().lock().get(&uaddr).cloned() else {
        return 0;
    };
    state.seq.fetch_add(1, Ordering::Release);
    let mut woken = 0usize;
    for _ in 0..count {
        if !state.queue.notify_one(true) {
            break;
        }
        woken += 1;
    }
    woken
}

pub(super) fn wake_task(uaddr: usize, task: &AxTaskRef) {
    if let Some(state) = table().lock().get(&uaddr).cloned() {
        state.seq.fetch_add(1, Ordering::Release);
        let _ = state.queue.notify_task(true, task);
    }
}
