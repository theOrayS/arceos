//! Trap handling.

#[cfg(feature = "uspace")]
use core::sync::atomic::{AtomicUsize, Ordering};

use memory_addr::VirtAddr;

pub use crate::TrapFrame;
pub use linkme::distributed_slice as def_trap_handler;
pub use linkme::distributed_slice as register_trap_handler;
pub use page_table_entry::MappingFlags as PageFaultFlags;

/// A slice of IRQ handler functions.
#[def_trap_handler]
pub static IRQ: [fn(usize) -> bool];

/// A slice of page fault handler functions.
#[def_trap_handler]
pub static PAGE_FAULT: [fn(VirtAddr, PageFaultFlags, bool) -> bool];

/// A slice of syscall handler functions.
#[cfg(feature = "uspace")]
#[cfg_attr(docsrs, doc(cfg(feature = "uspace")))]
#[def_trap_handler]
pub static SYSCALL: [fn(&TrapFrame, usize) -> isize];

#[cfg(feature = "uspace")]
static USER_RETURN_HANDLER: AtomicUsize = AtomicUsize::new(0);

#[allow(unused_macros)]
macro_rules! handle_trap {
    ($trap:ident, $($args:tt)*) => {{
        let mut iter = $crate::trap::$trap.iter();
        if let Some(func) = iter.next() {
            if iter.next().is_some() {
                warn!("Multiple handlers for trap {} are not currently supported", stringify!($trap));
            }
            func($($args)*)
        } else {
            warn!("No registered handler for trap {}", stringify!($trap));
            false
        }
    }}
}

/// Call the external syscall handler.
#[cfg(feature = "uspace")]
pub(crate) fn handle_syscall(tf: &TrapFrame, syscall_num: usize) -> isize {
    SYSCALL[0](tf, syscall_num)
}

/// Call the external user-return hook.
#[cfg(feature = "uspace")]
pub(crate) fn handle_user_return(tf: &mut TrapFrame) {
    let handler = USER_RETURN_HANDLER.load(Ordering::Acquire);
    if handler != 0 {
        let func = unsafe { core::mem::transmute::<usize, fn(&mut TrapFrame)>(handler) };
        func(tf);
    }
}

/// Register a hook invoked right before returning to user mode.
#[cfg(feature = "uspace")]
pub fn register_user_return_handler(handler: fn(&mut TrapFrame)) {
    let prev = USER_RETURN_HANDLER.compare_exchange(
        0,
        handler as usize,
        Ordering::AcqRel,
        Ordering::Acquire,
    );
    if prev.is_err() {
        warn!("USER_RETURN handler is already registered");
    }
}
