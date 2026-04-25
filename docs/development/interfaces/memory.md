# Memory Management Interfaces

Read this document when changing `brk`, `mmap`, `munmap`, `mprotect`,
`mremap`, page fault handling, file-backed mappings, page pinning, NUMA policy,
or shared memory.

Also read:

- `../policies/errno.md`
- `../policies/compatibility.md` if adding temporary behavior.
- `filesystem.md` when file-backed mappings are involved.
- `syscall-inventory.md` when changing syscall handlers.

## Workloads

- `basic`: `brk`, `mmap`, `munmap`.
- `libcbench`: allocator, string, and runtime memory pressure.
- `lmbench`: `lat_pagefault`, `lat_mmap`, `bw_mmap_rd`.
- LTP: prioritize `mm`, `hugetlb`, and `numa`; syscall families include
  `brk*`, `mmap*`, `munmap*`, `mremap*`, `madvise*`, `mincore*`, `mlock*`,
  `munlock*`, `move_pages*`, `migrate_pages*`, `shmat*`, `shmctl*`,
  `shmget*`.

## Current ArceOS Surfaces

- `axmm::AddrSpace`: `map_alloc`, `map_linear`, `unmap`, `protect`,
  `find_free_area`, `read`, `write`, `can_access_range`, `handle_page_fault`,
  and user mapping clone support.
- `MemorySet<Backend>` and paging flags behind `AddrSpace`.
- Current shell process logic stores `brk` state and calls `AddrSpace`
  directly for simple mmap/munmap/mprotect.

Known gaps:

- file-backed mmap and `MAP_SHARED` need a VMA-backed file mapping object.
- current file-backed `mmap` behavior is eager file read, not page-fault driven
  mapping semantics.
- `clone_user_mappings_from` copies pages eagerly; it is not COW.
- `mremap`, `mincore`, `madvise`, `mlock`, NUMA calls, and SysV shared memory
  need explicit contracts before LTP.

## VMA Authority Model

`AddrSpace` executes page-table changes. A VMA layer owns Linux memory
semantics.

Example model sketch:

```rust
pub struct MemoryMap {
    vmas: BTreeMap<VirtAddr, Vma>,
}

pub struct Vma {
    pub start: VirtAddr,
    pub end: VirtAddr,
    pub perms: VmaPerms,
    pub flags: VmaFlags,
    pub kind: VmaKind,
}

pub enum VmaKind {
    Heap,
    Anonymous,
    File { mapping: Arc<dyn FileMapping>, offset: u64, shared: bool },
    Stack,
    SharedMemory,
    Guard,
}
```

Required semantics:

- VMA types include `Heap`, `Anonymous`, `File`, `Stack`, `SharedMemory`, and
  `Guard`.
- `mmap`, `munmap`, `mprotect`, and `mremap` update VMAs first, then ask
  `AddrSpace` to update mappings.
- unmap/protect/remap can split and merge VMAs.
- page-table flags are derived from VMA permissions. Page tables are not the
  semantic source of truth.
- page fault looks up the VMA, checks access, fills anonymous/file/shared pages,
  then calls `AddrSpace` to install mappings.
- file-backed mappings store page-aligned file offset and length, plus any
  Linux-required intra-page delta.
- `MAP_PRIVATE` and `fork` should move toward COW. Eager copy is a functional
  bring-up path only.

## File-Backed Mapping Contract

Example API sketch:

```rust
pub trait FileMapping: Send + Sync {
    fn len(&self) -> AxResult<u64>;
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn read_page(&self, file_offset: u64, dst: &mut [u8]) -> AxResult<usize>;
    fn write_page(&self, file_offset: u64, src: &[u8]) -> AxResult<usize>;
    fn flush_range(&self, file_offset: u64, len: usize) -> AxResult<()>;
}
```

Required semantics:

- the mapping object holds file identity and lifetime until the VMA is removed.
- `MAP_SHARED` dirty pages have a defined writeback path.
- `MAP_PRIVATE` dirty pages do not write back to the source file.
- `pread/pwrite` and `mmap` do not depend on the current open-file offset.
- `msync` is missing until dirty tracking and flush semantics exist.

## Unsupported Calls

- `mlock*` must not return fake success. Return `ENOSYS` or `EOPNOTSUPP` until
  real pinning state exists.
- NUMA calls return explicit unsupported errors until NUMA policy is real.
- SysV shared memory needs a shared-memory object model before broad LTP mm.

## Promotion Gates

- Basic memory cases pass for anonymous `brk`, `mmap`, and `munmap`.
- libcbench allocator/string cases run without address-space corruption.
- lmbench memory cases emit valid results.
- file-backed mmap gates require VMA-backed semantics, not eager one-shot file
  reads.
- LTP mm gates require unsupported features to fail with documented errno.
