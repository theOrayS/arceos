pub(super) const USER_ASPACE_BASE: usize = 0x1_0000;
pub(super) const USER_ASPACE_SIZE: usize = 0x20_0000_0000;
pub(super) const USER_STACK_SIZE: usize = 8 * 1024 * 1024;
pub(super) const USER_STACK_GUARD: usize = 0x1_0000;
pub(super) const USER_STACK_TOP: usize = USER_ASPACE_BASE + USER_ASPACE_SIZE - USER_STACK_GUARD;
pub(super) const USER_MMAP_BASE: usize = 0x10_0000_0000;
pub(super) const USER_BRK_GROW_SIZE: usize = 64 * 1024 * 1024;
pub(super) const MAX_IN_MEMORY_FILE_SIZE: u64 = 128 * 1024 * 1024;
pub(super) const USER_PIE_LOAD_BASE: usize = USER_ASPACE_BASE;
pub(super) const MAX_SCRIPT_INTERPRETER_DEPTH: usize = 4;
pub(super) const TESTSUITE_STAGE_ROOT: &str = "/tmp/testsuite";
pub(super) const AUX_CLOCK_TICKS: usize = 100;

pub(super) const SIGCHLD_NUM: isize = 17;
pub(super) const SIGKILL_NUM: i32 = 9;
pub(super) const SIGALRM_NUM: i32 = 14;
pub(super) const SIGCANCEL_NUM: i32 = 33;
#[cfg(target_arch = "riscv64")]
pub(super) const SI_TKILL_CODE: i32 = -6;
#[cfg(target_arch = "riscv64")]
pub(super) const SA_NODEFER_FLAG: u64 = 0x4000_0000;
pub(super) const KERNEL_SIGSET_BYTES: usize = core::mem::size_of::<u64>();
pub(super) const SIG_BLOCK_HOW: usize = 0;
pub(super) const SIG_UNBLOCK_HOW: usize = 1;
pub(super) const SIG_SETMASK_HOW: usize = 2;
pub(super) const RLIMIT_STACK_RESOURCE: u32 = 3;
pub(super) const RLIMIT_NOFILE_RESOURCE: u32 = 7;
pub(super) const DEFAULT_NOFILE_LIMIT: u64 = 1024;

pub(super) const FD_SETSIZE: usize = 1024;
pub(super) const BITS_PER_USIZE: usize = usize::BITS as usize;
pub(super) const FD_SET_WORDS: usize = FD_SETSIZE.div_ceil(BITS_PER_USIZE);

#[cfg(target_arch = "riscv64")]
pub(super) const RISCV_SIGNAL_SIGSET_RESERVED_BYTES: usize = 120;
#[cfg(target_arch = "riscv64")]
pub(super) const RISCV_SIGNAL_FPSTATE_BYTES: usize = 528;
#[cfg(target_arch = "riscv64")]
pub(super) const SS_DISABLE: i32 = 2;
#[cfg(target_arch = "riscv64")]
pub(super) const RISCV_SIGTRAMP_CODE: [u32; 3] = [0x08b0_0893, 0x0000_0073, 0x0010_0073];

pub(super) const ST_MODE_DIR: u32 = 0o040000;
pub(super) const ST_MODE_FILE: u32 = 0o100000;
pub(super) const ST_MODE_CHR: u32 = 0o020000;
pub(super) const ST_MODE_SOCKET: u32 = 0o140000;
pub(super) const ST_MODE_TYPE_MASK: u32 = 0o170000;
pub(super) const FILE_MODE_PERMISSION_MASK: u32 = 0o7777;
pub(super) const FILE_MODE_SET_UID: u32 = 0o4000;
pub(super) const FILE_MODE_SET_GID: u32 = 0o2000;
pub(super) const FILE_MODE_GROUP_EXECUTE: u32 = 0o0010;
pub(super) const CHOWN_ID_UNCHANGED: u32 = u32::MAX;

pub(super) const STATFS_BLOCK_SIZE: i64 = 4096;
pub(super) const STATFS_NAME_MAX: i64 = 255;
pub(super) const TMPFS_MAGIC: i64 = 0x0102_1994;
pub(super) const PROC_SUPER_MAGIC: i64 = 0x9fa0;
pub(super) const SYSFS_MAGIC: i64 = 0x6265_6572;
pub(super) const DEVFS_MAGIC: i64 = 0x1373;
pub(super) const PIPEFS_MAGIC: i64 = 0x5049_5045;

pub(super) const SYSV_IPC_PRIVATE: i32 = 0;
pub(super) const SYSV_IPC_CREAT: i32 = 0o1000;
pub(super) const SYSV_IPC_EXCL: i32 = 0o2000;
pub(super) const SYSV_IPC_RMID: i32 = 0;
pub(super) const SYSV_IPC_SET: i32 = 1;
pub(super) const SYSV_IPC_STAT: i32 = 2;
pub(super) const SYSV_SHM_RDONLY: i32 = 0o10000;
pub(super) const SYSV_SHM_MAX_SIZE: usize = 16 * 1024 * 1024;

pub(super) const O_PATH_FLAG: u32 = 0o10000000;
pub(super) const PROC_SELF_MAPS_PATH: &str = "/proc/self/maps";
pub(super) const ETC_PASSWD_PATH: &str = "/etc/passwd";
pub(super) const ETC_GROUP_PATH: &str = "/etc/group";

pub(super) const AF_UNIX_DOMAIN: i32 = 1;
pub(super) const LOCAL_SOCKET_INO_BASE: u64 = 0x5f00_0000;
pub(super) const LINUX_EACCES: u32 = 13;
pub(super) const ACCESS_X_OK: usize = 1;
pub(super) const ACCESS_W_OK: usize = 2;
pub(super) const ACCESS_R_OK: usize = 4;
pub(super) const ACCESS_MODE_MASK: usize = ACCESS_X_OK | ACCESS_W_OK | ACCESS_R_OK;
pub(super) const DEFAULT_PASSWD_CONTENT: &[u8] =
    b"root:x:0:0:root:/root:/bin/sh\nnobody:x:65534:65534:nobody:/nonexistent:/sbin/nologin\n";
pub(super) const DEFAULT_GROUP_CONTENT: &[u8] = b"root:x:0:\nnogroup:x:65534:\n";
pub(super) const RTC_RD_TIME: u32 = 0x8024_7009;

pub(super) const SOL_SOCKET_LEVEL: i32 = 1;
pub(super) const SO_REUSEADDR_OPT: i32 = 2;
pub(super) const SO_TYPE_OPT: i32 = 3;
pub(super) const SO_ERROR_OPT: i32 = 4;
pub(super) const SO_DONTROUTE_OPT: i32 = 5;
pub(super) const SO_BROADCAST_OPT: i32 = 6;
pub(super) const SO_SNDBUF_OPT: i32 = 7;
pub(super) const SO_RCVBUF_OPT: i32 = 8;
pub(super) const SO_KEEPALIVE_OPT: i32 = 9;
pub(super) const SO_REUSEPORT_OPT: i32 = 15;
pub(super) const SO_RCVTIMEO_OPT: i32 = 20;
pub(super) const SO_SNDTIMEO_OPT: i32 = 21;
pub(super) const IPPROTO_IP_LEVEL: i32 = 0;
pub(super) const IP_RECVERR_OPT: i32 = 11;
pub(super) const MCAST_JOIN_GROUP_OPT: i32 = 42;
pub(super) const MCAST_LEAVE_GROUP_OPT: i32 = 45;
pub(super) const TCP_NODELAY_OPT: i32 = 1;
pub(super) const TCP_MAXSEG_OPT: i32 = 2;
pub(super) const TCP_INFO_OPT: i32 = 11;
pub(super) const DEFAULT_TCP_MAXSEG: i32 = 1460;
pub(super) const TCP_INFO_COMPAT_SIZE: usize = 256;
pub(super) const DEFAULT_SOCKET_BUFFER_SIZE: i32 = 64 * 1024;
pub(super) const INTERRUPTIBLE_SOCKET_RECV_QUANTUM: core::time::Duration =
    core::time::Duration::from_millis(20);
// Linux UAPI socket errno values used by both RV64 and LA64 targets here.
pub(super) const LINUX_EPROTONOSUPPORT: u32 = 93;
pub(super) const LINUX_ESOCKTNOSUPPORT: u32 = 94;
pub(super) const LINUX_EAFNOSUPPORT: u32 = 97;

pub(super) const LINUX_PERSONALITY_QUERY: usize = 0xffff_ffff;
pub(super) const LINUX_PERSONALITY_MASK: usize = 0xffff_ffff;

#[cfg(target_arch = "riscv64")]
pub(super) const AUX_PLATFORM: &str = "riscv64";
#[cfg(target_arch = "loongarch64")]
pub(super) const AUX_PLATFORM: &str = "loongarch64";
