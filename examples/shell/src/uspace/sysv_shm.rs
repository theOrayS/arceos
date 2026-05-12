use core::ptr;
use core::sync::atomic::{AtomicI32, Ordering};

use axalloc::global_allocator;
use axerrno::LinuxError;
use axsync::Mutex;
use lazyinit::LazyInit;
use memory_addr::PAGE_SIZE_4K;
use std::collections::BTreeMap;

use super::linux_abi::{SYSV_IPC_CREAT, SYSV_IPC_EXCL, SYSV_IPC_PRIVATE, SYSV_SHM_MAX_SIZE};
use super::memory_map::align_up;

#[derive(Clone)]
struct SysvShmSegment {
    key: i32,
    size: usize,
    backing_vaddr: usize,
}

static NEXT_SYSV_SHM_ID: AtomicI32 = AtomicI32::new(1);

fn table() -> &'static Mutex<BTreeMap<i32, SysvShmSegment>> {
    static SYSV_SHM: LazyInit<Mutex<BTreeMap<i32, SysvShmSegment>>> = LazyInit::new();
    if !SYSV_SHM.is_inited() {
        SYSV_SHM.init_once(Mutex::new(BTreeMap::new()));
    }
    &SYSV_SHM
}

pub(super) fn get_or_create(key: usize, size: usize, shmflg: usize) -> Result<i32, LinuxError> {
    let key = key as i32;
    let flags = shmflg as i32;
    let mut table = table().lock();
    if key != SYSV_IPC_PRIVATE {
        if let Some((shmid, segment)) = table.iter().find(|(_, segment)| segment.key == key) {
            if flags & SYSV_IPC_CREAT != 0 && flags & SYSV_IPC_EXCL != 0 {
                return Err(LinuxError::EINVAL);
            }
            if size > segment.size {
                return Err(LinuxError::EINVAL);
            }
            return Ok(*shmid);
        }
        if flags & SYSV_IPC_CREAT == 0 {
            return Err(LinuxError::ENOENT);
        }
    }

    let size = align_up(size.max(1), PAGE_SIZE_4K);
    if size > SYSV_SHM_MAX_SIZE {
        return Err(LinuxError::ENOMEM);
    }
    let pages = size / PAGE_SIZE_4K;
    let backing_vaddr = global_allocator()
        .alloc_pages(pages, PAGE_SIZE_4K)
        .map_err(|_| LinuxError::ENOMEM)?;
    unsafe {
        ptr::write_bytes(backing_vaddr as *mut u8, 0, size);
    }
    let shmid = NEXT_SYSV_SHM_ID.fetch_add(1, Ordering::Relaxed);
    table.insert(
        shmid,
        SysvShmSegment {
            key,
            size,
            backing_vaddr,
        },
    );
    Ok(shmid)
}

pub(super) fn lookup(shmid: i32) -> Option<(usize, usize)> {
    table()
        .lock()
        .get(&shmid)
        .map(|segment| (segment.size, segment.backing_vaddr))
}

pub(super) fn contains(shmid: i32) -> bool {
    table().lock().contains_key(&shmid)
}

pub(super) fn remove(shmid: i32) {
    table().lock().remove(&shmid);
}
