use alloc::{format, sync::Arc};
use axfs_vfs::{VfsNodeType, VfsOps, VfsResult};

use crate::fs;

#[cfg(feature = "devfs")]
pub(crate) fn devfs() -> Arc<fs::devfs::DeviceFileSystem> {
    let null = fs::devfs::NullDev;
    let zero = fs::devfs::ZeroDev;
    let urandom = fs::devfs::UrandomDev::default();
    let bar = fs::devfs::ZeroDev;
    let devfs = fs::devfs::DeviceFileSystem::new();
    let foo_dir = devfs.mkdir("foo");
    devfs.add("null", Arc::new(null));
    devfs.add("zero", Arc::new(zero));
    devfs.add("urandom", Arc::new(urandom));
    foo_dir.add("bar", Arc::new(bar));
    Arc::new(devfs)
}

#[cfg(feature = "ramfs")]
pub(crate) fn ramfs() -> Arc<fs::ramfs::RamFileSystem> {
    Arc::new(fs::ramfs::RamFileSystem::new())
}

#[cfg(feature = "procfs")]
pub(crate) fn procfs() -> VfsResult<Arc<fs::ramfs::RamFileSystem>> {
    let procfs = fs::ramfs::RamFileSystem::new();
    let proc_root = procfs.root_dir();

    // Create Linux-compatible proc files used by common user-space tools.
    proc_root.create("mounts", VfsNodeType::File)?;
    let file_mounts = proc_root.clone().lookup("./mounts")?;
    file_mounts.write_at(0, proc_mounts().as_bytes())?;

    proc_root.create("meminfo", VfsNodeType::File)?;
    let file_meminfo = proc_root.clone().lookup("./meminfo")?;
    let meminfo = proc_meminfo();
    file_meminfo.write_at(0, meminfo.as_bytes())?;

    proc_root.create("cpuinfo", VfsNodeType::File)?;
    let file_cpuinfo = proc_root.clone().lookup("./cpuinfo")?;
    file_cpuinfo.write_at(0, proc_cpuinfo().as_bytes())?;

    proc_root.create("sys", VfsNodeType::Dir)?;
    proc_root.create("sys/kernel", VfsNodeType::Dir)?;
    proc_root.create("sys/kernel/tainted", VfsNodeType::File)?;
    let file_tainted = proc_root.clone().lookup("./sys/kernel/tainted")?;
    file_tainted.write_at(0, b"0\n")?;
    proc_root.create("sys/kernel/pid_max", VfsNodeType::File)?;
    let file_pid_max = proc_root.clone().lookup("./sys/kernel/pid_max")?;
    file_pid_max.write_at(0, b"4194304\n")?;

    proc_root.create("sys/net", VfsNodeType::Dir)?;
    proc_root.create("sys/net/core", VfsNodeType::Dir)?;
    proc_root.create("sys/net/core/somaxconn", VfsNodeType::File)?;
    let file_somaxconn = proc_root.clone().lookup("./sys/net/core/somaxconn")?;
    file_somaxconn.write_at(0, b"4096\n")?;
    proc_root.create("sys/net/ipv4", VfsNodeType::Dir)?;
    proc_root.create("sys/net/ipv4/conf", VfsNodeType::Dir)?;
    proc_root.create("sys/net/ipv4/conf/lo", VfsNodeType::Dir)?;
    proc_root.create("sys/net/ipv4/conf/lo/tag", VfsNodeType::File)?;
    let file_lo_tag = proc_root.clone().lookup("./sys/net/ipv4/conf/lo/tag")?;
    file_lo_tag.write_at(0, b"0\n")?;

    // Create /proc/sys/vm/overcommit_memory
    proc_root.create("sys/vm", VfsNodeType::Dir)?;
    proc_root.create("sys/vm/overcommit_memory", VfsNodeType::File)?;
    let file_over = proc_root.clone().lookup("./sys/vm/overcommit_memory")?;
    file_over.write_at(0, b"0\n")?;

    proc_root.create("self", VfsNodeType::Dir)?;
    proc_root.create("self/mounts", VfsNodeType::File)?;
    let file_self_mounts = proc_root.clone().lookup("./self/mounts")?;
    file_self_mounts.write_at(0, proc_mounts().as_bytes())?;
    proc_root.create("self/stat", VfsNodeType::File)?;
    let file_self_stat = proc_root.clone().lookup("./self/stat")?;
    file_self_stat.write_at(0, proc_self_stat().as_bytes())?;

    Ok(Arc::new(procfs))
}

#[cfg(feature = "procfs")]
fn proc_mounts() -> &'static str {
    "rootfs / rootfs rw 0 0\n\
     devfs /dev devfs rw 0 0\n\
     tmpfs /tmp tmpfs rw 0 0\n\
     tmpfs /var tmpfs rw 0 0\n\
     proc /proc proc rw 0 0\n\
     sysfs /sys sysfs rw 0 0\n"
}

#[cfg(feature = "procfs")]
fn proc_cpuinfo() -> &'static str {
    if cfg!(target_arch = "riscv64") {
        "processor\t: 0\n\
         hart\t\t: 0\n\
         isa\t\t: rv64imac\n\
         mmu\t\t: sv39\n\
         uarch\t\t: arceos\n"
    } else if cfg!(target_arch = "loongarch64") {
        "processor\t: 0\n\
         model name\t: ArceOS LoongArch64 virtual CPU\n\
         CPU Family\t: Loongson-64bit\n"
    } else {
        "processor\t: 0\n\
         model name\t: ArceOS virtual CPU\n"
    }
}

#[cfg(feature = "procfs")]
fn proc_self_stat() -> &'static str {
    "1 (arceos) R 0 1 1 0 -1 4194560 0 0 0 0 1 0 0 0 20 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n"
}

#[cfg(feature = "procfs")]
fn proc_meminfo() -> alloc::string::String {
    const PAGE_SIZE_KIB: usize = 4;

    let allocator = axalloc::global_allocator();
    let used_pages_kib = allocator.used_pages().saturating_mul(PAGE_SIZE_KIB);
    let free_pages_kib = allocator.available_pages().saturating_mul(PAGE_SIZE_KIB);
    let heap_free_kib = allocator.available_bytes() / 1024;
    let heap_used_kib = allocator.used_bytes() / 1024;
    let total_kib = used_pages_kib.saturating_add(free_pages_kib);
    let free_kib = free_pages_kib.saturating_add(heap_free_kib);
    let used_kib = total_kib
        .saturating_sub(free_kib)
        .saturating_add(heap_used_kib);

    format!(
        "MemTotal:       {total_kib:8} kB\n\
         MemFree:        {free_kib:8} kB\n\
         MemAvailable:   {free_kib:8} kB\n\
         Buffers:               0 kB\n\
         Cached:                0 kB\n\
         SwapCached:            0 kB\n\
         Active:                0 kB\n\
         Inactive:              0 kB\n\
         SwapTotal:             0 kB\n\
         SwapFree:              0 kB\n\
         Dirty:                 0 kB\n\
         Writeback:             0 kB\n\
         Slab:           {used_kib:8} kB\n"
    )
}

#[cfg(feature = "sysfs")]
pub(crate) fn sysfs() -> VfsResult<Arc<fs::ramfs::RamFileSystem>> {
    let sysfs = fs::ramfs::RamFileSystem::new();
    let sys_root = sysfs.root_dir();

    // Create /sys/kernel/mm/transparent_hugepage/enabled
    sys_root.create("kernel", VfsNodeType::Dir)?;
    sys_root.create("kernel/mm", VfsNodeType::Dir)?;
    sys_root.create("kernel/mm/transparent_hugepage", VfsNodeType::Dir)?;
    sys_root.create("kernel/mm/transparent_hugepage/enabled", VfsNodeType::File)?;
    let file_hp = sys_root
        .clone()
        .lookup("./kernel/mm/transparent_hugepage/enabled")?;
    file_hp.write_at(0, b"always [madvise] never\n")?;

    // Create /sys/devices/system/clocksource/clocksource0/current_clocksource
    sys_root.create("devices", VfsNodeType::Dir)?;
    sys_root.create("devices/system", VfsNodeType::Dir)?;
    sys_root.create("devices/system/clocksource", VfsNodeType::Dir)?;
    sys_root.create("devices/system/clocksource/clocksource0", VfsNodeType::Dir)?;
    sys_root.create(
        "devices/system/clocksource/clocksource0/current_clocksource",
        VfsNodeType::File,
    )?;
    let file_cc = sys_root
        .clone()
        .lookup("devices/system/clocksource/clocksource0/current_clocksource")?;
    file_cc.write_at(0, b"tsc\n")?;

    Ok(Arc::new(sysfs))
}
