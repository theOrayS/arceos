//! Page table manipulation.

use alloc::vec::Vec;
use core::alloc::Layout;
use core::ptr::NonNull;

use axalloc::global_allocator;
use kspin::SpinNoIrq;
use memory_addr::{PhysAddr, VirtAddr};
use page_table_multiarch::PagingHandler;

use crate::mem::{phys_to_virt, virt_to_phys};

const PAGE_SIZE: usize = 0x1000;

static FALLBACK_PAGING_FRAMES: SpinNoIrq<Vec<(usize, usize, usize)>> = SpinNoIrq::new(Vec::new());

#[doc(no_inline)]
pub use page_table_multiarch::{MappingFlags, PageSize, PagingError, PagingResult};

/// Implementation of [`PagingHandler`], to provide physical memory manipulation to
/// the [page_table_multiarch] crate.
pub struct PagingHandlerImpl;

impl PagingHandler for PagingHandlerImpl {
    fn alloc_frames(num: usize, align: usize) -> Option<PhysAddr> {
        if let Ok(vaddr) = global_allocator().alloc_pages(num, align) {
            return Some(virt_to_phys(vaddr.into()));
        }

        let size = num.checked_mul(PAGE_SIZE)?;
        let align = align.max(PAGE_SIZE);
        let layout = Layout::from_size_align(size, align).ok()?;
        let ptr = global_allocator().alloc(layout).ok()?;
        let vaddr = ptr.as_ptr() as usize;
        let mut fallback = FALLBACK_PAGING_FRAMES.lock();
        if fallback.try_reserve(1).is_err() {
            global_allocator().dealloc(ptr, layout);
            return None;
        }
        fallback.push((vaddr, num, align));
        Some(virt_to_phys(vaddr.into()))
    }

    fn dealloc_frames(paddr: PhysAddr, num: usize) {
        let vaddr = phys_to_virt(paddr);
        let vaddr_usize = vaddr.as_usize();
        let mut fallback = FALLBACK_PAGING_FRAMES.lock();
        if let Some(index) = fallback
            .iter()
            .position(|&(addr, pages, _)| addr == vaddr_usize && pages == num)
        {
            let (_, pages, align) = fallback.swap_remove(index);
            drop(fallback);
            let size = pages
                .checked_mul(PAGE_SIZE)
                .expect("fallback page table allocation size overflow");
            let layout =
                Layout::from_size_align(size, align).expect("valid fallback page table layout");
            let ptr = NonNull::new(vaddr.as_mut_ptr()).expect("frame pointer must be non-null");
            global_allocator().dealloc(ptr, layout);
        } else {
            global_allocator().dealloc_pages(vaddr_usize, num)
        }
    }

    #[inline]
    fn phys_to_virt(paddr: PhysAddr) -> VirtAddr {
        phys_to_virt(paddr)
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        /// The architecture-specific page table.
        pub type PageTable = page_table_multiarch::x86_64::X64PageTable<PagingHandlerImpl>;
    } else if #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))] {
        /// The architecture-specific page table.
        pub type PageTable = page_table_multiarch::riscv::Sv39PageTable<PagingHandlerImpl>;
    } else if #[cfg(target_arch = "aarch64")]{
        /// The architecture-specific page table.
        pub type PageTable = page_table_multiarch::aarch64::A64PageTable<PagingHandlerImpl>;
    } else if #[cfg(target_arch = "loongarch64")] {
        /// The architecture-specific page table.
        pub type PageTable = page_table_multiarch::loongarch64::LA64PageTable<PagingHandlerImpl>;
    }
}
