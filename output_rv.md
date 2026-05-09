# RV evaluation output

Source: full RV evaluation log for the iozone SysV shared-memory + pwrite64 compatibility candidate.
Recorded metrics: top-level 116 pass-like / 4 fail-like / 55 skip.
Comparison with previous tracked output: top-level delta +0 pass-like / +0 fail-like / +0 skip; iozone completion delta +7; LTP diagnostic delta RUN +1, FAIL +1.
Target note: iozone-musl now reports 8 `iozone test complete` markers, with shared-memory setup and pwrite blockers absent (`shmget/Error 38` markers 0, `pwrite` ENOSYS markers 0).
Note: terminal ANSI/control bytes were removed so GitHub can render this file as readable Markdown. The evaluation content and line order are preserved.

```text
make test_build ARCH=riscv64 BUS=mmio \
	APP_FEATURES="auto-run-tests,uspace" \
	AXCONFIG_WRITES="-w plat.phys-memory-size=0x4000_0000" \
	OUT_DIR=/root/arceos/build/kernels/riscv64 \
	OUT_CONFIG=/root/arceos/build/kernels/riscv64.axconfig.toml \
	TARGET_DIR=/root/arceos/build/kernels/target/riscv64
make[1]: Entering directory '/root/arceos'
make A=examples/shell MODE=release LOG=info SMP=1 FEATURES=alloc,paging,irq,multitask,fs,net \
	ARCH=riscv64 BUS=mmio \
	APP_FEATURES="auto-run-tests,uspace" \
	AXCONFIG_WRITES="-w plat.phys-memory-size=0x4000_0000" \
	OUT_DIR=/root/arceos/build/kernels/riscv64 \
	OUT_CONFIG=/root/arceos/build/kernels/riscv64.axconfig.toml \
	TARGET_DIR=/root/arceos/build/kernels/target/riscv64 \
	build
make[2]: Entering directory '/root/arceos'
axconfig-gen configs/defconfig.toml /root/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/axplat-riscv64-qemu-virt-0.4.1/axconfig.toml  -w arch=riscv64 -w platform=riscv64-qemu-virt -o "/root/arceos/build/kernels/riscv64.axconfig.toml" -w plat.phys-memory-size=0x4000_0000 -w plat.max-cpu-num=1 -c "/root/arceos/build/kernels/riscv64.axconfig.toml"
    Building App: shell, Arch: riscv64, Platform: riscv64-qemu-virt, App type: rust
cargo -C examples/shell build -Z unstable-options --target riscv64gc-unknown-none-elf --target-dir /root/arceos/build/kernels/target/riscv64 --release  --features "axstd/defplat axstd/log-level-info axstd/bus-mmio axstd/alloc axstd/paging axstd/irq axstd/multitask axstd/fs axstd/net auto-run-tests uspace"
warning: methods `recv_loopback` and `recv_loopback_from` are never used
   --> kernel/net/axnet/src/smoltcp_impl/udp.rs:315:8
    |
272 | impl UdpSocket {
    | -------------- methods in this implementation
...
315 |     fn recv_loopback(&self, buf: &mut [u8], remote: Option<IpEndpoint>) -> AxResult<usize> {
    |        ^^^^^^^^^^^^^
...
319 |     fn recv_loopback_from(
    |        ^^^^^^^^^^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` on by default

warning: `axnet` (lib) generated 1 warning
warning: methods `insert` and `insert_min` are never used
    --> examples/shell/src/uspace.rs:7292:8
     |
6723 | impl FdTable {
     | ------------ methods in this implementation
...
7292 |     fn insert(&mut self, entry: FdEntry) -> Result<i32, LinuxError> {
     |        ^^^^^^
...
7300 |     fn insert_min(&mut self, entry: FdEntry, min_fd: usize) -> Result<i32, LinuxError> {
     |        ^^^^^^^^^^
     |
     = note: `#[warn(dead_code)]` on by default

warning: `arceos-shell` (bin "arceos-shell") generated 1 warning
    Finished `release` profile [optimized] target(s) in 0.39s
rust-objcopy --binary-architecture=riscv64 /root/arceos/build/kernels/riscv64/shell_riscv64-qemu-virt.elf --strip-all -O binary /root/arceos/build/kernels/riscv64/shell_riscv64-qemu-virt.bin
make[2]: Leaving directory '/root/arceos'
rust-objcopy -I binary -O elf64-littleriscv --rename-section .data=.text,alloc,load,readonly,code /root/arceos/build/kernels/riscv64/shell_riscv64-qemu-virt.bin /root/arceos/build/kernels/kernel-rv.wrap.o
rust-lld -flavor gnu -m elf64lriscv -T scripts/make/riscv64-kernel-wrap.lds /root/arceos/build/kernels/kernel-rv.wrap.o -o /root/arceos/kernel-rv
make[1]: Leaving directory '/root/arceos'
rm -f /tmp/arceos-sdcard-rv.run.qcow2
qemu-img create -f qcow2 -F raw -b sdcard-rv.img /tmp/arceos-sdcard-rv.run.qcow2
Formatting '/tmp/arceos-sdcard-rv.run.qcow2', fmt=qcow2 cluster_size=65536 extended_l2=off compression_type=zlib size=4294967296 backing_file=sdcard-rv.img backing_fmt=raw lazy_refcounts=off refcount_bits=16
qemu-system-riscv64 -machine virt -kernel /root/arceos/kernel-rv -m 1G -nographic -smp 1 -bios default -drive file=/tmp/arceos-sdcard-rv.run.qcow2,if=none,format=qcow2,id=x0 \
	-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 -no-reboot -device virtio-net-device,netdev=net -netdev user,id=net \
	-rtc base=utc 

OpenSBI v1.3
   ____                    _____ ____ _____
  / __ \                  / ____|  _ \_   _|
 | |  | |_ __   ___ _ __ | (___ | |_) || |
 | |  | | '_ \ / _ \ '_ \ \___ \|  _ < | |
 | |__| | |_) |  __/ | | |____) | |_) || |_
  \____/| .__/ \___|_| |_|_____/|___/_____|
        | |
        |_|

Platform Name             : riscv-virtio,qemu
Platform Features         : medeleg
Platform HART Count       : 1
Platform IPI Device       : aclint-mswi
Platform Timer Device     : aclint-mtimer @ 10000000Hz
Platform Console Device   : uart8250
Platform HSM Device       : ---
Platform PMU Device       : ---
Platform Reboot Device    : sifive_test
Platform Shutdown Device  : sifive_test
Platform Suspend Device   : ---
Platform CPPC Device      : ---
Firmware Base             : 0x80000000
Firmware Size             : 322 KB
Firmware RW Offset        : 0x40000
Firmware RW Size          : 66 KB
Firmware Heap Offset      : 0x48000
Firmware Heap Size        : 34 KB (total), 2 KB (reserved), 9 KB (used), 22 KB (free)
Firmware Scratch Size     : 4096 B (total), 760 B (used), 3336 B (free)
Runtime SBI Version       : 1.0

Domain0 Name              : root
Domain0 Boot HART         : 0
Domain0 HARTs             : 0*
Domain0 Region00          : 0x0000000002000000-0x000000000200ffff M: (I,R,W) S/U: ()
Domain0 Region01          : 0x0000000080040000-0x000000008005ffff M: (R,W) S/U: ()
Domain0 Region02          : 0x0000000080000000-0x000000008003ffff M: (R,X) S/U: ()
Domain0 Region03          : 0x0000000000000000-0xffffffffffffffff M: (R,W,X) S/U: (R,W,X)
Domain0 Next Address      : 0x0000000080200000
Domain0 Next Arg1         : 0x00000000bfe00000
Domain0 Next Mode         : S-mode
Domain0 SysReset          : yes
Domain0 SysSuspend        : yes

Boot HART ID              : 0
Boot HART Domain          : root
Boot HART Priv Version    : v1.12
Boot HART Base ISA        : rv64imafdch
Boot HART ISA Extensions  : time,sstc
Boot HART PMP Count       : 16
Boot HART PMP Granularity : 4
Boot HART PMP Address Bits: 54
Boot HART MHPM Count      : 16
Boot HART MIDELEG         : 0x0000000000001666
Boot HART MEDELEG         : 0x0000000000f0b509

       d8888                            .d88888b.   .d8888b.
      d88888                           d88P" "Y88b d88P  Y88b
     d88P888                           888     888 Y88b.
    d88P 888 888d888  .d8888b  .d88b.  888     888  "Y888b.
   d88P  888 888P"   d88P"    d8P  Y8b 888     888     "Y88b.
  d88P   888 888     888      88888888 888     888       "888
 d8888888888 888     Y88b.    Y8b.     Y88b. .d88P Y88b  d88P
d88P     888 888      "Y8888P  "Y8888   "Y88888P"   "Y8888P"

arch = riscv64
platform = riscv64-qemu-virt
target = riscv64gc-unknown-none-elf
build_mode = release
log_level = info

[  0.140757 0 axruntime:135] Logging is enabled.
[  0.143883 0 axruntime:136] Primary CPU 0 started, arg = 0xbfe00000.
[  0.147090 0 axruntime:139] Found physcial memory regions:
[  0.148061 0 axruntime:141]   [PA:0x101000, PA:0x102000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.149411 0 axruntime:141]   [PA:0xc000000, PA:0xc210000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.151255 0 axruntime:141]   [PA:0x10000000, PA:0x10001000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.152402 0 axruntime:141]   [PA:0x10001000, PA:0x10009000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.154178 0 axruntime:141]   [PA:0x30000000, PA:0x40000000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.154841 0 axruntime:141]   [PA:0x40000000, PA:0x80000000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.155475 0 axruntime:141]   [PA:0x80200000, PA:0x802b3000) .text (READ | EXECUTE | RESERVED)
[  0.157644 0 axruntime:141]   [PA:0x802b3000, PA:0x802d8000) .rodata (READ | RESERVED)
[  0.158962 0 axruntime:141]   [PA:0x802d8000, PA:0x802dc000) .data .tdata .tbss .percpu (READ | WRITE | RESERVED)
[  0.160705 0 axruntime:141]   [PA:0x802dc000, PA:0x8031c000) boot stack (READ | WRITE | RESERVED)
[  0.161965 0 axruntime:141]   [PA:0x8031c000, PA:0x80345000) .bss (READ | WRITE | RESERVED)
[  0.162561 0 axruntime:141]   [PA:0x80345000, PA:0xc0000000) free memory (READ | WRITE | FREE)
[  0.164789 0 axruntime:216] Initialize global memory allocator...
[  0.165675 0 axruntime:217]   use TLSF allocator.
[  0.170769 0 axmm:103] Initialize virtual memory management...
[  0.329341 0 axruntime:156] Initialize platform devices...
smp = 1
[  0.331558 0 axtask::api:73] Initialize scheduling...
[  0.335739 0 axtask::api:83]   use FIFO scheduler.
[  0.337246 0 axdriver:152] Initialize device drivers...
[  0.338498 0 axdriver:153]   device model: static
[  0.341415 0 virtio_drivers::device::blk:63] found a block device of size 4194304KB
[  0.345941 0 axdriver::bus::mmio:11] registered a new Block device at [PA:0x10001000, PA:0x10002000): "virtio-blk"
[  0.350036 0 virtio_drivers::device::net::dev_raw:33] negotiated_features Features(MAC | STATUS | RING_INDIRECT_DESC | RING_EVENT_IDX)
[  0.355732 0 axdriver::bus::mmio:11] registered a new Net device at [PA:0x10008000, PA:0x10009000): "virtio-net"
[  0.358459 0 axfs:44] Initialize filesystems...
[  0.360176 0 axfs:47]   use block device 0: "virtio-blk"
[  0.363095 0 axfs::root:336]   detected root filesystem: Ext4
[  0.406557 0 axnet:42] Initialize network subsystem...
[  0.407861 0 axnet:45]   use NIC 0: "virtio-net"
[  0.415731 0 axnet::smoltcp_impl:335] created net interface "eth0":
[  0.417749 0 axnet::smoltcp_impl:336]   ether:    52-54-00-12-34-56
[  0.419416 0 axnet::smoltcp_impl:337]   ip:       10.0.2.15/24
[  0.421475 0 axnet::smoltcp_impl:338]   gateway:  10.0.2.2
[  0.422641 0 axruntime:182] Initialize interrupt handlers...
[  0.425121 0 axruntime:194] Primary CPU 0 init OK.
#### OS COMP TEST GROUP START basic-musl ####
Testing brk :
========== START test_brk ==========
Before alloc,heap pos: 77824
After alloc,heap pos: 77888
Alloc again,heap pos: 77952
========== END test_brk ==========
Testing chdir :
========== START test_chdir ==========
chdir ret: 0
  current working dir : 
========== END test_chdir ==========
Testing clone :
========== START test_clone ==========
  Child says successfully!
clone process successfully.
pid:11
========== END test_clone ==========
Testing close :
========== START test_close ==========
  close 3 success.
========== END test_close ==========
Testing dup2 :
========== START test_dup2 ==========
  from fd 100
========== END test_dup2 ==========
Testing dup :
========== START test_dup ==========
  new fd is 3.
========== END test_dup ==========
Testing execve :
========== START test_execve ==========
  I am test_echo.
execve success.
========== END main ==========
Testing exit :
========== START test_exit ==========
exit OK.
========== END test_exit ==========
Testing fork :
========== START test_fork ==========
  child process.
  parent process. wstatus:0
========== END test_fork ==========
Testing fstat :
========== START test_fstat ==========
fstat ret: 0
fstat: dev: 1, inode: 1012599416, mode: 33206, nlink: 1, size: 52, atime: 0, mtime: 0, ctime: 0
========== END test_fstat ==========
Testing getcwd :
========== START test_getcwd ==========
getcwd: /tmp/testsuite/musl/basic/basic successfully!
========== END test_getcwd ==========
Testing getdents :
========== START test_getdents ==========
open fd:3
getdents fd:-20
getdents success.


========== END test_getdents ==========
Testing getpid :
========== START test_getpid ==========
getpid success.
pid = 23
========== END test_getpid ==========
Testing getppid :
========== START test_getppid ==========
  getppid success. ppid : 7
========== END test_getppid ==========
Testing gettimeofday :
========== START test_gettimeofday ==========
gettimeofday success.
start:7857, end:7966
interval: 109
========== END test_gettimeofday ==========
Testing mkdir_ :
========== START test_mkdir ==========
mkdir ret: 0
  mkdir success.
========== END test_mkdir ==========
Testing mmap :
========== START test_mmap ==========
file len: 27
mmap content:   Hello, mmap successfully!
========== END test_mmap ==========
Testing mount :
========== START test_mount ==========
Mounting dev:/dev/vda2 to ./mnt
mount return: -38

 --- Assert Fatal ! ---
Testing munmap :
========== START test_munmap ==========
file len: 27
munmap return: 0
munmap successfully!
========== END test_munmap ==========
Testing openat :
========== START test_openat ==========
open dir fd: 3
openat fd: 4
openat success.
========== END test_openat ==========
Testing open :
========== START test_open ==========
Hi, this is a text file.
syscalls testing success!

========== END test_open ==========
Testing pipe :
========== START test_pipe ==========
cpid: 33
cpid: 0
  Write to pipe successfully.

========== END test_pipe ==========
Testing read :
========== START test_read ==========
Hi, this is a text file.
syscalls testing success!

========== END test_read ==========
Testing /musl/busybox :
/tmp/testsuite/musl/basic/basic/run-all.sh: line 40: .//musl/busybox: Exec format error
Testing sleep :
========== START test_sleep ==========
sleep success.
========== END test_sleep ==========
Testing times :
========== START test_times ==========
mytimes success
{tms_utime:0, tms_stime:0, tms_cutime:0, tms_cstime:0}
========== END test_times ==========
Testing umount :
========== START test_umount ==========
Mounting dev:/dev/vda2 to ./mnt
mount return: -38
========== END test_umount ==========
Testing uname :
========== START test_uname ==========
Uname: Linux arceos 6.0.0 ArceOS riscv64 localdomain
========== END test_uname ==========
Testing unlink :
========== START test_unlink ==========
  unlink success!
========== END test_unlink ==========
Testing wait :
========== START test_wait ==========
This is child process
wait child success.
wstatus: 0
========== END test_wait ==========
Testing waitpid :
========== START test_waitpid ==========
This is child process
waitpid successfully.
wstatus: 3
========== END test_waitpid ==========
Testing write :
========== START test_write ==========
Hello operating system contest.
========== END test_write ==========
Testing yield :
========== START test_yield ==========
  I am child process: 47. iteration -2144489472.
  I am child process: 48. iteration -2144489472.
  I am child process: 49. iteration -2144489472.
  I am child process: 47. iteration -2144489472.
  I am child process: 48. iteration -2144489472.
  I am child process: 49. iteration -2144489472.
  I am child process: 47. iteration -2144489472.
  I am child process: 48. iteration -2144489472.
  I am child process: 49. iteration -2144489472.
  I am child process: 47. iteration -2144489472.
  I am child process: 48. iteration -2144489472.
  I am child process: 49. iteration -2144489472.
  I am child process: 47. iteration -2144489472.
  I am child process: 48. iteration -2144489472.
  I am child process: 49. iteration -2144489472.
========== END test_yield ==========
#### OS COMP TEST GROUP END basic-musl ####
#### OS COMP TEST GROUP START busybox-musl ####
#### independent command test
testcase busybox echo "#### independent command test" success
testcase busybox ash -c exit success
testcase busybox sh -c exit success
bbb
testcase busybox basename /aaa/bbb success
    January 1970
Su Mo Tu We Th Fr Sa
             1  2  3
 4  5  6  7  8  9 10
11 12 13 14 15 16 17
18 19 20 21 22 23 24
25 26 27 28 29 30 31
                     
testcase busybox cal success
testcase busybox clear success
Thu Jan  1 00:00:20 UTC 1970
testcase busybox date success
Filesystem           1K-blocks      Used Available Use% Mounted on
devfs                  1045228     36648   1008580   4% /dev
tmpfs                  1045228     36648   1008580   4% /tmp
tmpfs                  1045228     36648   1008580   4% /var
proc                   1045228     36648   1008580   4% /proc
sysfs                  1045228     36648   1008580   4% /sys
testcase busybox df success
/aaa
testcase busybox dirname /aaa/bbb success
testcase busybox dmesg success
0	.
testcase busybox du success
2
testcase busybox expr 1 + 1 success
testcase busybox false success
testcase busybox true success
testcase busybox which ls fail
return: 1, cmd: which ls
Linux
testcase busybox uname success
 00:00:29 up 0 min,  0 users,  load average: 0.00, 0.00, 0.00
testcase busybox uptime success
abc
testcase busybox printf "abc\n" success
PID   USER     TIME  COMMAND
testcase busybox ps success
/tmp/testsuite/musl/busybox
testcase busybox pwd success
              total        used        free      shared  buff/cache   available
Mem:              0           0           0           0           0     1039813
-/+ buffers/cache:            0           0
Swap:             0           0           0
testcase busybox free success
Thu Jan  1 00:00:33 1970  0.000000 seconds
testcase busybox hwclock success
sh: sleep: not found
testcase busybox sh -c 'sleep 5' & /musl/busybox kill $! success
busybox_cmd.txt      busybox_testcode.sh  line
testcase busybox ls success
testcase busybox sleep 1 success
#### file opration test
testcase busybox echo "#### file opration test" success
testcase busybox touch test.txt success
testcase busybox echo "hello world" > test.txt success
hello world
testcase busybox cat test.txt success
l
testcase busybox cut -c 3 test.txt success
0000000 062550 066154 020157 067567 066162 005144
0000014
testcase busybox od test.txt success
hello world
testcase busybox head test.txt success
hello world
testcase busybox tail test.txt success
00000000  68 65 6c 6c 6f 20 77 6f  72 6c 64 0a              |hello world.|
0000000c
testcase busybox hexdump -C test.txt success
6f5902ac237024bdd0c176cb93063dc4  test.txt
testcase busybox md5sum test.txt success
testcase busybox echo "ccccccc" >> test.txt success
testcase busybox echo "bbbbbbb" >> test.txt success
testcase busybox echo "aaaaaaa" >> test.txt success
testcase busybox echo "2222222" >> test.txt success
testcase busybox echo "1111111" >> test.txt success
testcase busybox echo "bbbbbbb" >> test.txt success
1111111
2222222
aaaaaaa
bbbbbbb
ccccccc
hello world
testcase busybox sort test.txt | /musl/busybox uniq success
  File: test.txt
  Size: 60        	Blocks: 0          IO Block: 512    regular file
Device: 1h/1d	Inode: 14331471978328146352  Links: 1
Access: (0666/-rw-rw-rw-)  Uid: (    0/    root)   Gid: (    0/    root)
Access: 1970-01-01 00:00:00.000000000 +0000
Modify: 1970-01-01 00:00:00.000000000 +0000
Change: 1970-01-01 00:00:00.000000000 +0000
testcase busybox stat test.txt success
hello world
ccccccc
bbbbbbb
aaaaaaa
2222222
1111111
bbbbbbb
testcase busybox strings test.txt success
        7         8        60 test.txt
testcase busybox wc test.txt success
testcase busybox [ -f test.txt ] success
hello world
ccccccc
bbbbbbb
aaaaaaa
2222222
1111111
bbbbbbb
testcase busybox more test.txt success
testcase busybox rm test.txt -f success
testcase busybox mkdir test_dir success
testcase busybox mv test_dir test success
testcase busybox rmdir test success
echo "hello world" > test.txt
grep hello busybox_cmd.txt
testcase busybox grep hello busybox_cmd.txt success
testcase busybox cp busybox_cmd.txt busybox_cmd.bak success
testcase busybox rm busybox_cmd.bak -f success
./busybox_cmd.txt
testcase busybox find -name "busybox_cmd.txt" success
#### OS COMP TEST GROUP END busybox-musl ####
#### OS COMP TEST GROUP START cyclictest-musl ####
====== cyclictest NO_STRESS_P1 begin ======
WARN: stat /dev/cpu_dma_latency failed: No such file or directory
T: 0 (  113) P:99 I:1000 C:    975 Min:      4 Act:    9 Avg:   54 Max:    4090
====== cyclictest NO_STRESS_P1 end: success ======
====== cyclictest NO_STRESS_P8 begin ======
WARN: stat /dev/cpu_dma_latency failed: No such file or directory
T: 0 (  115) P:99 I:1000 C:    989 Min:      4 Act:   13 Avg:   46 Max:    3368
T: 1 (  116) P:99 I:1500 C:    662 Min:      4 Act:   81 Avg:   43 Max:    2618
T: 2 (  117) P:99 I:2000 C:    497 Min:      4 Act:  592 Avg:   47 Max:    2082
T: 3 (  118) P:99 I:2500 C:    399 Min:      4 Act:  583 Avg:   43 Max:    2544
T: 4 (  119) P:99 I:3000 C:    334 Min:      4 Act:   24 Avg:   34 Max:    2360
T: 5 (  120) P:99 I:3500 C:    286 Min:      4 Act:   28 Avg:   52 Max:    2471
T: 6 (  121) P:99 I:4000 C:    250 Min:      5 Act:  647 Avg:   27 Max:     716
T: 7 (  122) P:99 I:4500 C:    223 Min:      4 Act:   31 Avg:   51 Max:    2234
====== cyclictest NO_STRESS_P8 end: success ======
====== start hackbench ======
Running in process mode with 10 groups using 40 file descriptors each (== 400 tasks)
Each sender will pass 100000000 messages of 100 bytes
Creating fdpair (error: Function not implemented)
====== cyclictest STRESS_P1 begin ======
WARN: stat /dev/cpu_dma_latency failed: No such file or directory
T: 0 (  126) P:99 I:1000 C:    997 Min:      5 Act:   13 Avg:   30 Max:    1975
====== cyclictest STRESS_P1 end: success ======
====== cyclictest STRESS_P8 begin ======
WARN: stat /dev/cpu_dma_latency failed: No such file or directory
T: 0 (  128) P:99 I:1000 C:    999 Min:      4 Act:   22 Avg:   27 Max:    1210
T: 1 (  129) P:99 I:1500 C:    667 Min:      5 Act:   78 Avg:   23 Max:     783
T: 2 (  130) P:99 I:2000 C:    500 Min:      4 Act:   98 Avg:   22 Max:    1188
T: 3 (  131) P:99 I:2500 C:    400 Min:      5 Act:  131 Avg:   20 Max:     330
T: 4 (  132) P:99 I:3000 C:    334 Min:      4 Act:   22 Avg:   21 Max:     622
T: 5 (  133) P:99 I:3500 C:    286 Min:      5 Act:   97 Avg:   20 Max:     500
T: 6 (  134) P:99 I:4000 C:    250 Min:      3 Act:   58 Avg:   24 Max:     147
T: 7 (  135) P:99 I:4500 C:    223 Min:      5 Act:   15 Avg:   21 Max:     185
====== cyclictest STRESS_P8 end: success ======
kill: can't kill pid 123: No such process
====== kill hackbench: fail, ignore STRESS result ======
#### OS COMP TEST GROUP END cyclictest-musl ####
#### OS COMP TEST GROUP START iozone-musl ####
iozone automatic measurements
	Iozone: Performance Test of File I/O
	        Version $Revision: 3.506 $
		Compiled for 64 bit mode.
		Build: linux 

	Contributors:William Norcott, Don Capps, Isom Crawford, Kirby Collins
	             Al Slater, Scott Rhine, Mike Wisner, Ken Goss
	             Steve Landherr, Brad Smith, Mark Kelly, Dr. Alain CYR,
	             Randy Dunlap, Mark Montague, Dan Million, Gavin Brebner,
	             Jean-Marc Zucconi, Jeff Blomberg, Benny Halevy, Dave Boone,
	             Erik Habbinga, Kris Strecker, Walter Wong, Joshua Root,
	             Fabrice Bacchella, Zhenghua Xue, Qin Li, Darren Sawyer,
	             Vangel Bojaxhi, Ben England, Vikentsi Lapa,
	             Alexey Skidanov, Sudhir Kumar.

	Run began: Thu Jan  1 00:01:21 1970

	Auto Mode
	Record Size 1 kB
	File size set to 4096 kB
	Command line used: ./iozone -a -r 1k -s 4m
	Output is in kBytes/sec
	Time Resolution = 0.000007 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
                                                                    random    random      bkwd     record     stride                                        
              kB  reclen    write    rewrite      read    reread      read     write      read    rewrite       read    fwrite  frewrite     fread   freread
            4096       1     42154     99119     51080     77531     33467     37211[ 83.194629 0:143 axfs::fops:269] [AxError::InvalidInput]
[ 83.276201 0:143 axfs::fops:269] [AxError::InvalidInput]
     55090      51222      56382     78027     78473     51076     46822

iozone test complete.
iozone throughput write/read measurements
	Iozone: Performance Test of File I/O
	        Version $Revision: 3.506 $
		Compiled for 64 bit mode.
		Build: linux 

	Contributors:William Norcott, Don Capps, Isom Crawford, Kirby Collins
	             Al Slater, Scott Rhine, Mike Wisner, Ken Goss
	             Steve Landherr, Brad Smith, Mark Kelly, Dr. Alain CYR,
	             Randy Dunlap, Mark Montague, Dan Million, Gavin Brebner,
	             Jean-Marc Zucconi, Jeff Blomberg, Benny Halevy, Dave Boone,
	             Erik Habbinga, Kris Strecker, Walter Wong, Joshua Root,
	             Fabrice Bacchella, Zhenghua Xue, Qin Li, Darren Sawyer,
	             Vangel Bojaxhi, Ben England, Vikentsi Lapa,
	             Alexey Skidanov, Sudhir Kumar.

	Run began: Thu Jan  1 00:01:24 1970

	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 0 -i 1 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000007 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes
	Each process writes a 1024 kByte file in 1 kByte records

	Children see throughput for  4 initial writers 	=   76744.36 kB/sec
	Parent sees throughput for  4 initial writers 	=    2401.48 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   76744.36 kB/sec
	Avg throughput per process 			=   19186.09 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for  4 rewriters 	=   96640.24 kB/sec
	Parent sees throughput for  4 rewriters 	=    2924.81 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   96640.24 kB/sec
	Avg throughput per process 			=   24160.06 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for  4 readers 		=   84726.12 kB/sec
	Parent sees throughput for  4 readers 		=    2827.62 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   84726.12 kB/sec
	Avg throughput per process 			=   21181.53 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for 4 re-readers 	=   79079.47 kB/sec
	Parent sees throughput for 4 re-readers 	=    2384.98 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   79079.47 kB/sec
	Avg throughput per process 			=   19769.87 kB/sec
	Min xfer 					=       0.00 kB



iozone test complete.
iozone throughput random-read measurements
	Iozone: Performance Test of File I/O
	        Version $Revision: 3.506 $
		Compiled for 64 bit mode.
		Build: linux 

	Contributors:William Norcott, Don Capps, Isom Crawford, Kirby Collins
	             Al Slater, Scott Rhine, Mike Wisner, Ken Goss
	             Steve Landherr, Brad Smith, Mark Kelly, Dr. Alain CYR,
	             Randy Dunlap, Mark Montague, Dan Million, Gavin Brebner,
	             Jean-Marc Zucconi, Jeff Blomberg, Benny Halevy, Dave Boone,
	             Erik Habbinga, Kris Strecker, Walter Wong, Joshua Root,
	             Fabrice Bacchella, Zhenghua Xue, Qin Li, Darren Sawyer,
	             Vangel Bojaxhi, Ben England, Vikentsi Lapa,
	             Alexey Skidanov, Sudhir Kumar.

	Run began: Thu Jan  1 00:01:43 1970

	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 0 -i 2 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000007 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes
	Each process writes a 1024 kByte file in 1 kByte records

	Children see throughput for  4 initial writers 	=   77929.98 kB/sec
	Parent sees throughput for  4 initial writers 	=    2572.26 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   77929.98 kB/sec
	Avg throughput per process 			=   19482.50 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for  4 rewriters 	=   83333.33 kB/sec
	Parent sees throughput for  4 rewriters 	=    2575.71 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   83333.33 kB/sec
	Avg throughput per process 			=   20833.33 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for 4 random readers 	=   40095.54 kB/sec
	Parent sees throughput for 4 random readers 	=    2810.11 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   40095.54 kB/sec
	Avg throughput per process 			=   10023.88 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for 4 random writers 	=   61383.52 kB/sec
	Parent sees throughput for 4 random writers 	=    2666.49 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   61383.52 kB/sec
	Avg throughput per process 			=   15345.88 kB/sec
	Min xfer 					=       0.00 kB



iozone test complete.
iozone throughput read-backwards measurements
	Iozone: Performance Test of File I/O
	        Version $Revision: 3.506 $
		Compiled for 64 bit mode.
		Build: linux 

	Contributors:William Norcott, Don Capps, Isom Crawford, Kirby Collins
	             Al Slater, Scott Rhine, Mike Wisner, Ken Goss
	             Steve Landherr, Brad Smith, Mark Kelly, Dr. Alain CYR,
	             Randy Dunlap, Mark Montague, Dan Million, Gavin Brebner,
	             Jean-Marc Zucconi, Jeff Blomberg, Benny Halevy, Dave Boone,
	             Erik Habbinga, Kris Strecker, Walter Wong, Joshua Root,
	             Fabrice Bacchella, Zhenghua Xue, Qin Li, Darren Sawyer,
	             Vangel Bojaxhi, Ben England, Vikentsi Lapa,
	             Alexey Skidanov, Sudhir Kumar.

	Run began: Thu Jan  1 00:02:07 1970

	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 0 -i 3 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000007 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes
	Each process writes a 1024 kByte file in 1 kByte records

	Children see throughput for  4 initial writers 	=   77120.05 kB/sec
	Parent sees throughput for  4 initial writers 	=    2334.33 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   77120.05 kB/sec
	Avg throughput per process 			=   19280.01 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for  4 rewriters 	=   94814.81 kB/sec
	Parent sees throughput for  4 rewriters 	=    2722.69 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   94814.81 kB/sec
	Avg throughput per process 			=   23703.70 kB/sec
	Min xfer 					=       0.00 kB
[136.508704 0:198 axfs::fops:269] [AxError::InvalidInput]

	Children see throughput for 4 reverse readers 	=   41704.00 kB/sec
	Parent sees throughput for 4 reverse readers 	=    2519.72 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   41704.00 kB/sec
	Avg throughput per process 			=   10426.00 kB/sec
	Min xfer 					=       0.00 kB



iozone test complete.
iozone throughput stride-read measurements
	Iozone: Performance Test of File I/O
	        Version $Revision: 3.506 $
		Compiled for 64 bit mode.
		Build: linux 

	Contributors:William Norcott, Don Capps, Isom Crawford, Kirby Collins
	             Al Slater, Scott Rhine, Mike Wisner, Ken Goss
	             Steve Landherr, Brad Smith, Mark Kelly, Dr. Alain CYR,
	             Randy Dunlap, Mark Montague, Dan Million, Gavin Brebner,
	             Jean-Marc Zucconi, Jeff Blomberg, Benny Halevy, Dave Boone,
	             Erik Habbinga, Kris Strecker, Walter Wong, Joshua Root,
	             Fabrice Bacchella, Zhenghua Xue, Qin Li, Darren Sawyer,
	             Vangel Bojaxhi, Ben England, Vikentsi Lapa,
	             Alexey Skidanov, Sudhir Kumar.

	Run began: Thu Jan  1 00:02:25 1970

	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 0 -i 5 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000006 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes
	Each process writes a 1024 kByte file in 1 kByte records

	Children see throughput for  4 initial writers 	=   79657.72 kB/sec
	Parent sees throughput for  4 initial writers 	=    2370.73 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   79657.72 kB/sec
	Avg throughput per process 			=   19914.43 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for  4 rewriters 	=   93082.45 kB/sec
	Parent sees throughput for  4 rewriters 	=    2662.31 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   93082.45 kB/sec
	Avg throughput per process 			=   23270.61 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for 4 stride readers 	=   57241.88 kB/sec
	Parent sees throughput for 4 stride readers 	=    2546.64 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   57241.88 kB/sec
	Avg throughput per process 			=   14310.47 kB/sec
	Min xfer 					=       0.00 kB



iozone test complete.
iozone throughput fwrite/fread measurements
	Iozone: Performance Test of File I/O
	        Version $Revision: 3.506 $
		Compiled for 64 bit mode.
		Build: linux 

	Contributors:William Norcott, Don Capps, Isom Crawford, Kirby Collins
	             Al Slater, Scott Rhine, Mike Wisner, Ken Goss
	             Steve Landherr, Brad Smith, Mark Kelly, Dr. Alain CYR,
	             Randy Dunlap, Mark Montague, Dan Million, Gavin Brebner,
	             Jean-Marc Zucconi, Jeff Blomberg, Benny Halevy, Dave Boone,
	             Erik Habbinga, Kris Strecker, Walter Wong, Joshua Root,
	             Fabrice Bacchella, Zhenghua Xue, Qin Li, Darren Sawyer,
	             Vangel Bojaxhi, Ben England, Vikentsi Lapa,
	             Alexey Skidanov, Sudhir Kumar.

	Run began: Thu Jan  1 00:02:43 1970

	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 6 -i 7 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000007 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes
	Each process writes a 1024 kByte file in 1 kByte records

	Children see throughput for  4 fwriters 	=  232435.26 kB/sec
	Parent sees throughput for  4 fwriters 		=    9612.07 kB/sec
	Min throughput per process 			=   35320.09 kB/sec 
	Max throughput per process 			=   71698.64 kB/sec
	Avg throughput per process 			=   58108.82 kB/sec
	Min xfer 					=    1024.00 kB

	Children see throughput for  4 freaders 	=  205471.73 kB/sec
	Parent sees throughput for  4 freaders 		=    9585.39 kB/sec
	Min throughput per process 			=   50309.52 kB/sec 
	Max throughput per process 			=   51811.37 kB/sec
	Avg throughput per process 			=   51367.93 kB/sec
	Min xfer 					=    1024.00 kB



iozone test complete.
iozone throughput pwrite/pread measurements
	Iozone: Performance Test of File I/O
	        Version $Revision: 3.506 $
		Compiled for 64 bit mode.
		Build: linux 

	Contributors:William Norcott, Don Capps, Isom Crawford, Kirby Collins
	             Al Slater, Scott Rhine, Mike Wisner, Ken Goss
	             Steve Landherr, Brad Smith, Mark Kelly, Dr. Alain CYR,
	             Randy Dunlap, Mark Montague, Dan Million, Gavin Brebner,
	             Jean-Marc Zucconi, Jeff Blomberg, Benny Halevy, Dave Boone,
	             Erik Habbinga, Kris Strecker, Walter Wong, Joshua Root,
	             Fabrice Bacchella, Zhenghua Xue, Qin Li, Darren Sawyer,
	             Vangel Bojaxhi, Ben England, Vikentsi Lapa,
	             Alexey Skidanov, Sudhir Kumar.

	Run began: Thu Jan  1 00:02:56 1970

	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 9 -i 10 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000006 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes
	Each process writes a 1024 kByte file in 1 kByte records

	Children see throughput for 4 pwrite writers 	=   78317.40 kB/sec
	Parent sees throughput for 4 pwrite writers 	=    2500.93 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   78317.40 kB/sec
	Avg throughput per process 			=   19579.35 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for 4 pread readers 	=  100569.63 kB/sec
	Parent sees throughput for 4 pread readers 	=    2689.19 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=  100569.63 kB/sec
	Avg throughput per process 			=   25142.41 kB/sec
	Min xfer 					=       0.00 kB



iozone test complete.
iozone throughtput pwritev/preadv measurements
	Iozone: Performance Test of File I/O
	        Version $Revision: 3.506 $
		Compiled for 64 bit mode.
		Build: linux 

	Contributors:William Norcott, Don Capps, Isom Crawford, Kirby Collins
	             Al Slater, Scott Rhine, Mike Wisner, Ken Goss
	             Steve Landherr, Brad Smith, Mark Kelly, Dr. Alain CYR,
	             Randy Dunlap, Mark Montague, Dan Million, Gavin Brebner,
	             Jean-Marc Zucconi, Jeff Blomberg, Benny Halevy, Dave Boone,
	             Erik Habbinga, Kris Strecker, Walter Wong, Joshua Root,
	             Fabrice Bacchella, Zhenghua Xue, Qin Li, Darren Sawyer,
	             Vangel Bojaxhi, Ben England, Vikentsi Lapa,
	             Alexey Skidanov, Sudhir Kumar.

	Run began: Thu Jan  1 00:03:13 1970

	Selected test not available on the version.
	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 11 -i 12 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000006 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes
	Each process writes a 1024 kByte file in 1 kByte records

	Children see throughput for  4 initial writers 	=   88550.68 kB/sec
	Parent sees throughput for  4 initial writers 	=    2483.61 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   88550.68 kB/sec
	Avg throughput per process 			=   22137.67 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for  4 rewriters 	=   86691.50 kB/sec
	Parent sees throughput for  4 rewriters 	=    2856.35 kB/sec
	Min throughput per process 			=       0.00 kB/sec 
	Max throughput per process 			=   86691.50 kB/sec
	Avg throughput per process 			=   21672.88 kB/sec
	Min xfer 					=       0.00 kB



iozone test complete.
#### OS COMP TEST GROUP END iozone-musl ####
#### OS COMP TEST GROUP START iperf-musl ####
====== iperf BASIC_UDP begin ======
warning: Ignoring nonsense TCP MSS 0
Connecting to host 127.0.0.1, port 5001
[  5] local 0.0.0.0 port 49152 connected to 127.0.0.1 port 5001
[ ID] Interval           Transfer     Bitrate         Total Datagrams
[  5]   0.00-2.00   sec  6.24 MBytes  26.2 Mbits/sec  4483  
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-2.00   sec  6.24 MBytes  26.2 Mbits/sec  0.000 ms  0/4483 (0%)  sender
[  5]   0.00-2.00   sec  6.24 MBytes  26.1 Mbits/sec  0.177 ms  0/4483 (0%)  receiver

iperf Done.
====== iperf BASIC_UDP end: success ======

====== iperf BASIC_TCP begin ======
warning: Ignoring nonsense TCP MSS 0
Connecting to host 127.0.0.1, port 5001
[  5] local 127.0.0.1 port 49154 connected to 127.0.0.1 port 5001
iperf3: getsockopt - Invalid argument
[ ID] Interval           Transfer     Bitrate         Retr  Cwnd
[  5]   0.00-2.00   sec  55.8 MBytes   234 Mbits/sec    0   42.7 MBytes       
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Retr
[  5]   0.00-2.00   sec  55.8 MBytes   234 Mbits/sec    0             sender
[  5]   0.00-2.01   sec  54.9 MBytes   229 Mbits/sec                  receiver

iperf Done.
====== iperf BASIC_TCP end: success ======

====== iperf PARALLEL_UDP begin ======
warning: Ignoring nonsense TCP MSS 0
Connecting to host 127.0.0.1, port 5001
[  5] local 0.0.0.0 port 49153 connected to 127.0.0.1 port 5001
[  7] local 0.0.0.0 port 49154 connected to 127.0.0.1 port 5001
[  9] local 0.0.0.0 port 49155 connected to 127.0.0.1 port 5001
[ 11] local 0.0.0.0 port 49156 connected to 127.0.0.1 port 5001
[ 13] local 0.0.0.0 port 49157 connected to 127.0.0.1 port 5001
[ ID] Interval           Transfer     Bitrate         Total Datagrams
[  5]   0.00-2.00   sec  1.09 MBytes  4.56 Mbits/sec  781  
[  7]   0.00-2.00   sec  1.09 MBytes  4.56 Mbits/sec  781  
[  9]   0.00-2.00   sec  1.09 MBytes  4.56 Mbits/sec  781  
[ 11]   0.00-2.00   sec  1.09 MBytes  4.56 Mbits/sec  781  
[ 13]   0.00-2.00   sec  1.09 MBytes  4.56 Mbits/sec  781  
[SUM]   0.00-2.00   sec  5.44 MBytes  22.8 Mbits/sec  3905  
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-2.00   sec  1.09 MBytes  4.56 Mbits/sec  0.000 ms  0/781 (0%)  sender
[  5]   0.00-2.00   sec  5.44 MBytes  22.8 Mbits/sec  0.273 ms  959981556/959985462 (1e+02%)  receiver
[  7]   0.00-2.00   sec  1.09 MBytes  4.56 Mbits/sec  0.000 ms  0/781 (0%)  sender
[  7]   0.00-2.00   sec  5.44 MBytes  22.8 Mbits/sec  0.275 ms  959981556/959985462 (1e+02%)  receiver
[  9]   0.00-2.00   sec  1.09 MBytes  4.56 Mbits/sec  0.000 ms  0/781 (0%)  sender
[  9]   0.00-2.00   sec  5.44 MBytes  22.8 Mbits/sec  0.274 ms  1551438522/1551442428 (1e+02%)  receiver
[ 11]   0.00-2.00   sec  1.09 MBytes  4.56 Mbits/sec  0.000 ms  0/781 (0%)  sender
[ 11]   0.00-2.00   sec  5.44 MBytes  22.8 Mbits/sec  0.274 ms  1323731717/1323735623 (1e+02%)  receiver
[ 13]   0.00-2.00   sec  1.09 MBytes  4.56 Mbits/sec  0.000 ms  0/781 (0%)  sender
[ 13]   0.00-2.00   sec  5.44 MBytes  22.8 Mbits/sec  0.273 ms  0/781 (0%)  receiver
[SUM]   0.00-2.00   sec  5.44 MBytes  22.8 Mbits/sec  0.000 ms  0/3905 (0%)  sender
[SUM]   0.00-2.00   sec  27.2 MBytes   114 Mbits/sec  0.274 ms  500166055/500182460 (1.3e+07%)  receiver

iperf Done.
====== iperf PARALLEL_UDP end: success ======

====== iperf PARALLEL_TCP begin ======
warning: Ignoring nonsense TCP MSS 0
Connecting to host 127.0.0.1, port 5001
[  5] local 127.0.0.1 port 49157 connected to 127.0.0.1 port 5001
[  7] local 127.0.0.1 port 49158 connected to 127.0.0.1 port 5001
[  9] local 127.0.0.1 port 49159 connected to 127.0.0.1 port 5001
[ 11] local 127.0.0.1 port 49160 connected to 127.0.0.1 port 5001
[ 13] local 127.0.0.1 port 49161 connected to 127.0.0.1 port 5001
iperf3: getsockopt - Invalid argument
iperf3: getsockopt - Invalid argument
iperf3: getsockopt - Invalid argument
iperf3: getsockopt - Invalid argument
iperf3: getsockopt - Invalid argument
[ ID] Interval           Transfer     Bitrate         Retr  Cwnd
[  5]   0.00-2.03   sec  13.0 MBytes  53.8 Mbits/sec    0   43.7 MBytes       
[  7]   0.00-2.03   sec  13.0 MBytes  53.7 Mbits/sec    0   43.7 MBytes       
[  9]   0.00-2.03   sec  13.0 MBytes  53.7 Mbits/sec    0   43.7 MBytes       
[ 11]   0.00-2.03   sec  13.0 MBytes  53.7 Mbits/sec    0   43.7 MBytes       
[ 13]   0.00-2.03   sec  13.0 MBytes  53.7 Mbits/sec    0   43.7 MBytes       
[SUM]   0.00-2.03   sec  65.0 MBytes   269 Mbits/sec    0             
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Retr
[  5]   0.00-2.03   sec  13.0 MBytes  53.8 Mbits/sec    0             sender
[  5]   0.00-2.05   sec  12.1 MBytes  49.7 Mbits/sec                  receiver
[  7]   0.00-2.03   sec  13.0 MBytes  53.8 Mbits/sec    0             sender
[  7]   0.00-2.05   sec  12.1 MBytes  49.7 Mbits/sec                  receiver
[  9]   0.00-2.03   sec  13.0 MBytes  53.8 Mbits/sec    0             sender
[  9]   0.00-2.05   sec  12.1 MBytes  49.7 Mbits/sec                  receiver
[ 11]   0.00-2.03   sec  13.0 MBytes  53.8 Mbits/sec    0             sender
[ 11]   0.00-2.05   sec  12.1 MBytes  49.7 Mbits/sec                  receiver
[ 13]   0.00-2.03   sec  13.0 MBytes  53.8 Mbits/sec    0             sender
[ 13]   0.00-2.05   sec  12.1 MBytes  49.7 Mbits/sec                  receiver
[SUM]   0.00-2.03   sec  65.0 MBytes   269 Mbits/sec    0             sender
[SUM]   0.00-2.05   sec  60.6 MBytes   248 Mbits/sec                  receiver

iperf Done.
====== iperf PARALLEL_TCP end: success ======

====== iperf REVERSE_UDP begin ======
warning: Ignoring nonsense TCP MSS 0
Connecting to host 127.0.0.1, port 5001
Reverse mode, remote host 127.0.0.1 is sending
[  5] local 0.0.0.0 port 49158 connected to 127.0.0.1 port 5001
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-2.00   sec  5.64 MBytes  23.7 Mbits/sec  0.070 ms  0/4053 (0%)  
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-2.01   sec  5.64 MBytes  23.6 Mbits/sec  0.000 ms  0/4054 (0%)  sender
[  5]   0.00-2.00   sec  5.64 MBytes  23.7 Mbits/sec  0.070 ms  0/4053 (0%)  receiver

iperf Done.
====== iperf REVERSE_UDP end: success ======

====== iperf REVERSE_TCP begin ======
warning: Ignoring nonsense TCP MSS 0
Connecting to host 127.0.0.1, port 5001
Reverse mode, remote host 127.0.0.1 is sending
[  5] local 127.0.0.1 port 49164 connected to 127.0.0.1 port 5001
iperf3: getsockopt - Invalid argument
[ ID] Interval           Transfer     Bitrate         Retr  Cwnd
[  5]   0.00-2.00   sec  50.1 MBytes   210 Mbits/sec                  
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Retr
[  5]   0.00-2.05   sec  51.0 MBytes   209 Mbits/sec    0             sender
[  5]   0.00-2.00   sec  50.1 MBytes   210 Mbits/sec                  receiver

iperf Done.
====== iperf REVERSE_TCP end: success ======

#### OS COMP TEST GROUP END iperf-musl ####
#### OS COMP TEST GROUP START libcbench-musl ####
SKIP: libcbench currently triggers an unrecovered allocator exhaustion path
#### OS COMP TEST GROUP END libcbench-musl ####
#### OS COMP TEST GROUP START libctest-musl ####
SKIP: libctest still trips unresolved pthread cancellation paths
#### OS COMP TEST GROUP END libctest-musl ####
#### OS COMP TEST GROUP START lmbench-musl ####
SKIP: lmbench still triggers an unresolved user-space page-fault path
#### OS COMP TEST GROUP END lmbench-musl ####
#### OS COMP TEST GROUP START ltp-musl ####
RUN LTP CASE abort01
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
abort01.c:51: TFAIL: Child exited with 139, expected SIGIOT

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[231.648909 0:282 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE abort01 : 0
RUN LTP CASE abs01
abs01       1  TPASS  :  Test passed
abs01       2  TPASS  :  Test passed
abs01       3  TPASS  :  Test passed
FAIL LTP CASE abs01 : 0
RUN LTP CASE accept01
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
accept01.c:92: TPASS: bad file descriptor : EBADF (9)
[242.654521 0:293 axnet::smoltcp_impl::tcp:284] [AxError::InvalidInput] socket accept() failed: not listen
[242.655318 0:293 arceos_posix_api::imp::net:486] sys_accept => Err(EINVAL)
accept01.c:92: TPASS: invalid socket buffer : EINVAL (22)
[242.660910 0:293 axnet::smoltcp_impl::tcp:284] [AxError::InvalidInput] socket accept() failed: not listen
[242.662207 0:293 arceos_posix_api::imp::net:486] sys_accept => Err(EINVAL)
accept01.c:92: TPASS: invalid salen : EINVAL (22)
[242.666829 0:293 axnet::smoltcp_impl::tcp:284] [AxError::InvalidInput] socket accept() failed: not listen
[242.667359 0:293 arceos_posix_api::imp::net:486] sys_accept => Err(EINVAL)
accept01.c:92: TPASS: no queued connections : EINVAL (22)
[242.668251 0:293 arceos_posix_api::imp::net:486] sys_accept => Err(EOPNOTSUPP)
accept01.c:92: TPASS: UDP accept : EOPNOTSUPP (95)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[242.787156 0:290 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE accept01 : 0
RUN LTP CASE accept02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_buffers.c:57: TINFO: Test is using guarded buffers
accept02.c:131: TINFO: Starting listener on port: 49166
accept02.c:75: TPASS: Multicast group was not copied: EADDRNOTAVAIL (99)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[249.236020 0:295 axfs::fops:297] [AxError::NotADirectory]
[249.238986 0:295 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE accept02 : 0
RUN LTP CASE accept03
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
accept03.c:48: TPASS: accept() on file : ENOTSOCK (88)
accept03.c:48: TPASS: accept() on O_PATH file : EBADF (9)
accept03.c:48: TPASS: accept() on directory : ENOTSOCK (88)
accept03.c:48: TPASS: accept() on /dev/zero : ENOTSOCK (88)
accept03.c:48: TPASS: accept() on /proc/self/maps : ENOTSOCK (88)
accept03.c:48: TPASS: accept() on pipe read end : ENOTSOCK (88)
accept03.c:48: TPASS: accept() on pipe write end : ENOTSOCK (88)
tst_fd.c:106: TCONF: epoll_create(): ENOSYS (38)
tst_fd.c:114: TCONF: Skipping eventfd: ENOSYS (38)
tst_fd.c:125: TCONF: Skipping signalfd: ENOSYS (38)
tst_fd.c:135: TCONF: Skipping timerfd: ENOSYS (38)
tst_fd.c:144: TCONF: pidfd_open(): ENOSYS (38)
tst_fd.c:151: TCONF: Skipping fanotify: ENOSYS (38)
tst_fd.c:160: TCONF: Skipping inotify: ENOSYS (38)
tst_fd.c:170: TCONF: Skipping userfaultfd: ENOSYS (38)
tst_fd.c:188: TCONF: Skipping perf event: ENOSYS (38)
tst_fd.c:199: TCONF: Skipping io uring: ENOSYS (38)
tst_fd.c:215: TCONF: Skipping bpf map: ENOSYS (38)
tst_fd.c:224: TCONF: Skipping fsopen: ENOSYS (38)
tst_fd.c:233: TCONF: Skipping fspick: ENOSYS (38)
tst_fd.c:242: TCONF: Skipping open_tree: ENOSYS (38)
tst_fd.c:251: TCONF: Skipping memfd: ENOSYS (38)
tst_fd.c:260: TCONF: Skipping memfd secret: ENOSYS (38)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[256.503637 0:302 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE accept03 : 0
RUN LTP CASE accept4_01
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
accept4_01.c:71: TINFO: Testing variant: libc accept4()
accept4_01.c:78: TINFO: server listening on: 49168
accept4_01.c:151: TPASS: Close-on-exec 0, nonblock 0
accept4_01.c:151: TPASS: Close-on-exec 1, nonblock 0
accept4_01.c:151: TPASS: Close-on-exec 0, nonblock 1
accept4_01.c:151: TPASS: Close-on-exec 1, nonblock 1
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
accept4_01.c:71: TINFO: Testing variant: __NR_accept4 syscall
accept4_01.c:78: TINFO: server listening on: 49173
accept4_01.c:151: TPASS: Close-on-exec 0, nonblock 0
accept4_01.c:151: TPASS: Close-on-exec 1, nonblock 0
accept4_01.c:151: TPASS: Close-on-exec 0, nonblock 1
accept4_01.c:151: TPASS: Close-on-exec 1, nonblock 1
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
accept4_01.c:71: TINFO: Testing variant: __NR_socketcall SYS_ACCEPT4 syscall
accept4_01.c:78: TINFO: server listening on: 49178
accept4_01.c:43: TCONF: syscall(-1) __NR_socketcall not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[263.518887 0:307 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE accept4_01 : 32
RUN LTP CASE access01
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
access01.c:245: TPASS: access(accessfile_rwx, F_OK) as root passed
access01.c:245: TPASS: access(accessfile_rwx, F_OK) as nobody passed
access01.c:245: TPASS: access(accessfile_rwx, X_OK) as root passed
access01.c:245: TPASS: access(accessfile_rwx, X_OK) as nobody passed
access01.c:245: TPASS: access(accessfile_rwx, W_OK) as root passed
access01.c:245: TPASS: access(accessfile_rwx, W_OK) as nobody passed
access01.c:245: TPASS: access(accessfile_rwx, R_OK) as root passed
access01.c:245: TPASS: access(accessfile_rwx, R_OK) as nobody passed
access01.c:245: TPASS: access(accessfile_rwx, R_OK|W_OK) as root passed
access01.c:245: TPASS: access(accessfile_rwx, R_OK|W_OK) as nobody passed
access01.c:245: TPASS: access(accessfile_rwx, R_OK|X_OK) as root passed
access01.c:245: TPASS: access(accessfile_rwx, R_OK|X_OK) as nobody passed
access01.c:245: TPASS: access(accessfile_rwx, W_OK|X_OK) as root passed
access01.c:245: TPASS: access(accessfile_rwx, W_OK|X_OK) as nobody passed
access01.c:245: TPASS: access(accessfile_rwx, R_OK|W_OK|X_OK) as root passed
access01.c:245: TPASS: access(accessfile_rwx, R_OK|W_OK|X_OK) as nobody passed
access01.c:245: TPASS: access(accessfile_x, X_OK) as root passed
access01.c:245: TPASS: access(accessfile_x, X_OK) as nobody passed
access01.c:245: TPASS: access(accessfile_w, W_OK) as root passed
access01.c:245: TPASS: access(accessfile_w, W_OK) as nobody passed
access01.c:245: TPASS: access(accessfile_r, R_OK) as root passed
access01.c:245: TPASS: access(accessfile_r, R_OK) as nobody passed
access01.c:242: TPASS: access(accessfile_r, X_OK) as root : EACCES (13)
access01.c:242: TPASS: access(accessfile_r, X_OK) as nobody : EACCES (13)
access01.c:242: TPASS: access(accessfile_r, W_OK) as nobody : EACCES (13)
tst_test.c:1464: TBROK: Test 12 haven't reported results!

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[271.661179 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.661970 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.663279 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.665666 0:318 axfs::root:433] [AxError::IsADirectory]
[271.669155 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.669803 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.670493 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.672328 0:318 axfs::root:433] [AxError::IsADirectory]
[271.673258 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.675770 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.678215 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.681236 0:318 axfs::root:433] [AxError::IsADirectory]
[271.682157 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.684021 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.688042 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.693422 0:318 axfs::root:433] [AxError::IsADirectory]
[271.694513 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.698992 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.702843 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.704579 0:318 axfs::root:433] [AxError::IsADirectory]
[271.705660 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.706345 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.707105 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.707945 0:318 axfs::root:433] [AxError::IsADirectory]
[271.708764 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.709412 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.710146 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.710761 0:318 axfs::fops:297] [AxError::NotADirectory]
[271.711671 0:318 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE access01 : 2
RUN LTP CASE access02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
access02.c:175: TBROK: symlink(file_f,symlink_f) failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[278.274859 0:336 axfs::fops:297] [AxError::NotADirectory]
[278.276666 0:336 axfs::fops:297] [AxError::NotADirectory]
[278.277374 0:336 axfs::fops:297] [AxError::NotADirectory]
[278.278723 0:336 axfs::fops:297] [AxError::NotADirectory]
[278.280800 0:336 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE access02 : 2
RUN LTP CASE access03
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
access03.c:37: TPASS: invalid address as root : EFAULT (14)
access03.c:46: TPASS: invalid address as nobody : EFAULT (14)
access03.c:37: TPASS: invalid address as root : EFAULT (14)
access03.c:46: TPASS: invalid address as nobody : EFAULT (14)
access03.c:37: TPASS: invalid address as root : EFAULT (14)
access03.c:46: TPASS: invalid address as nobody : EFAULT (14)
access03.c:37: TPASS: invalid address as root : EFAULT (14)
access03.c:46: TPASS: invalid address as nobody : EFAULT (14)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[285.670069 0:341 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE access03 : 0
RUN LTP CASE access04
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_test.c:1003: TINFO: Can't mount (null) at mntpoint (tmpfs): ENOSYS (38)
tst_test.c:1303: TINFO: Can't mount tmpfs read-only, falling back to block device...
tst_device.c:293: TWARN: Failed to create test_dev.img: ENOSPC (28)
tst_device.c:354: TBROK: Failed to acquire device

Summary:
passed   0
failed   0
broken   1
skipped  0
warnings 1
[296.282667 0:350 axfs::root:433] [AxError::IsADirectory]
[296.284800 0:350 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE access04 : 6
RUN LTP CASE acct01
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_test.c:1003: TINFO: Can't mount (null) at ro_mntpoint (tmpfs): ENOSYS (38)
tst_test.c:1303: TINFO: Can't mount tmpfs read-only, falling back to block device...
tst_device.c:293: TWARN: Failed to create test_dev.img: ENOSPC (28)
tst_device.c:354: TBROK: Failed to acquire device

Summary:
passed   0
failed   0
broken   1
skipped  0
warnings 1
[307.609363 0:352 axfs::root:433] [AxError::IsADirectory]
[307.611109 0:352 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE acct01 : 6
RUN LTP CASE acct02
tst_kconfig.c:71: TINFO: Couldn't locate kernel config!
tst_kconfig.c:207: TBROK: Cannot parse kernel .config
FAIL LTP CASE acct02 : 2
RUN LTP CASE acct02_helper
FAIL LTP CASE acct02_helper : 128
RUN LTP CASE acl1
The acl library was missing upon compilation.
FAIL LTP CASE acl1 : 32
RUN LTP CASE add_ipv6addr
SKIP LTP CASE add_ipv6addr : requires LTP network environment
FAIL LTP CASE add_ipv6addr : 32
RUN LTP CASE add_key01
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
../../../../include/lapi/keyctl.h:29: TCONF: syscall(217) __NR_add_key not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[324.504254 0:362 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE add_key01 : 32
RUN LTP CASE add_key02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
../../../../include/lapi/keyctl.h:29: TCONF: syscall(217) __NR_add_key not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[329.943749 0:367 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE add_key02 : 32
RUN LTP CASE add_key03
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
../../../../include/lapi/keyctl.h:29: TCONF: syscall(217) __NR_add_key not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[335.479619 0:372 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE add_key03 : 32
RUN LTP CASE add_key04
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_buffers.c:57: TINFO: Test is using guarded buffers
../../../../include/lapi/keyctl.h:54: TCONF: syscall(219) __NR_keyctl not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[341.410735 0:377 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE add_key04 : 32
RUN LTP CASE add_key05
tst_cmd.c:257: TCONF: Couldn't find 'useradd' in $PATH
FAIL LTP CASE add_key05 : 32
RUN LTP CASE adjtimex01
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
adjtimex01.c:48: TBROK: adjtimex(): failed to save current params: ENOSYS (38)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[352.572805 0:384 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE adjtimex01 : 2
RUN LTP CASE adjtimex02
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
adjtimex02.c:111: TINFO: Testing variant: libc adjtimex()
adjtimex02.c:129: TBROK: adjtimex(): failed to save current params: ENOSYS (38)
adjtimex02.c:141: TWARN: Failed to restore saved parameters

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[358.001085 0:389 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE adjtimex02 : 2
RUN LTP CASE adjtimex03
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
adjtimex03.c:57: TBROK: adjtimex(): Unexpeceted error, expecting EINVAL with mode 0x8000: ENOSYS (38)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[363.621428 0:394 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE adjtimex03 : 2
RUN LTP CASE af_alg01
SKIP LTP CASE af_alg01 : AF_ALG unsupported by kernel
FAIL LTP CASE af_alg01 : 32
RUN LTP CASE af_alg02
SKIP LTP CASE af_alg02 : AF_ALG unsupported by kernel
FAIL LTP CASE af_alg02 : 32
RUN LTP CASE af_alg03
SKIP LTP CASE af_alg03 : AF_ALG unsupported by kernel
FAIL LTP CASE af_alg03 : 32
RUN LTP CASE af_alg04
SKIP LTP CASE af_alg04 : AF_ALG unsupported by kernel
FAIL LTP CASE af_alg04 : 32
RUN LTP CASE af_alg05
SKIP LTP CASE af_alg05 : AF_ALG unsupported by kernel
FAIL LTP CASE af_alg05 : 32
RUN LTP CASE af_alg06
SKIP LTP CASE af_alg06 : AF_ALG unsupported by kernel
FAIL LTP CASE af_alg06 : 32
RUN LTP CASE af_alg07
SKIP LTP CASE af_alg07 : AF_ALG unsupported by kernel
FAIL LTP CASE af_alg07 : 32
RUN LTP CASE aio-stress
tst_test.c:1175: TCONF: test requires libaio and its development packages
FAIL LTP CASE aio-stress : 32
RUN LTP CASE aio01
aio01       1  TCONF  :  aio01.c:421: test requires libaio and it's development packages
aio01       2  TCONF  :  aio01.c:421: Remaining cases not appropriate for configuration
FAIL LTP CASE aio01 : 32
RUN LTP CASE aio02
tst_test.c:1175: TCONF: test requires libaio and its development packages
FAIL LTP CASE aio02 : 32
RUN LTP CASE aiocp
tst_test.c:1175: TCONF: test requires libaio and its development packages
FAIL LTP CASE aiocp : 32
RUN LTP CASE aiodio_append
tst_test.c:1175: TCONF: test requires libaio and its development packages
FAIL LTP CASE aiodio_append : 32
RUN LTP CASE aiodio_sparse
tst_test.c:1175: TCONF: test requires libaio and its development packages
FAIL LTP CASE aiodio_sparse : 32
RUN LTP CASE alarm02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
alarm02.c:36: TPASS: alarm(2147483647) passed
alarm02.c:38: TPASS: alarm(0) passed
alarm02.c:36: TPASS: alarm(2147483647) passed
alarm02.c:38: TPASS: alarm(0) passed
alarm02.c:36: TPASS: alarm(1073741823) passed
alarm02.c:38: TPASS: alarm(0) passed

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[404.111206 0:418 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE alarm02 : 0
RUN LTP CASE alarm03
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
alarm03.c:30: TPASS: alarm(0) in parent process passed
alarm03.c:26: TPASS: alarm(0) in child process passed

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[410.184549 0:426 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE alarm03 : 0
RUN LTP CASE alarm05
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
alarm05.c:28: TPASS: alarm(10) passed
alarm05.c:30: TPASS: alarm(1) passed
alarm05.c:32: TFAIL: alarms_fired (0) != 1 (1)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[419.184750 0:433 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE alarm05 : 0
RUN LTP CASE alarm06
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
alarm06.c:35: TPASS: alarm(0) passed
alarm06.c:40: TPASS: alarms_received == 0 (0)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[427.686576 0:440 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE alarm06 : 0
RUN LTP CASE alarm07
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
alarm07.c:36: TFAIL: alarm_cnt (0) != 1 (1)
alarm07.c:32: TPASS: alarm_cnt == 0 (0)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[436.635516 0:446 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE alarm07 : 0
RUN LTP CASE ar01.sh
/musl/ltp/testcases/bin/ar01.sh: .: line 349: tst_test.sh: not found
FAIL LTP CASE ar01.sh : 2
RUN LTP CASE arch_prctl01
tst_test.c:1201: TCONF: This arch 'unknown' is not supported for test!
FAIL LTP CASE arch_prctl01 : 32
RUN LTP CASE arping01.sh
/musl/ltp/testcases/bin/arping01.sh: .: line 21: tst_net.sh: not found
FAIL LTP CASE arping01.sh : 2
RUN LTP CASE asapi_01
asapi_01    1  TPASS  :  IN6_ARE_ADDR_EQUAL
asapi_01    2  TFAIL  :  asapi_01.c:119: "hopopt" protocols entry
asapi_01    3  TPASS  :  "ipv6" protocols entry
asapi_01    4  TPASS  :  "ipv6-route" protocols entry
asapi_01    5  TPASS  :  "ipv6-frag" protocols entry
asapi_01    6  TPASS  :  "esp" protocols entry
asapi_01    7  TPASS  :  "ah" protocols entry
asapi_01    8  TPASS  :  "ipv6-icmp" protocols entry
asapi_01    9  TPASS  :  "ipv6-nonxt" protocols entry
asapi_01   10  TPASS  :  "ipv6-opts" protocols entry
[450.163140 0:459 arceos_posix_api::imp::net:333] sys_bind => Err(EINVAL)
asapi_01   11  TBROK  :  asapi_01.c:355: bind(3, sock_ntop: unknown AF_xxx: 0, len: 16, 16) failed: errno=EINVAL(22): Invalid argument
asapi_01   12  TBROK  :  asapi_01.c:355: Remaining cases broken
FAIL LTP CASE asapi_01 : 3
RUN LTP CASE asapi_02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
asapi_02.c:219: TCONF: socket(10, 3, 58) failed: EAFNOSUPPORT (97)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[456.226800 0:461 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE asapi_02 : 32
RUN LTP CASE asapi_03
asapi_03    1  TCONF  :  asapi_03.c:255: socket(10, 3, 159) failed: errno=EAFNOSUPPORT(97): Address family not supported by protocol
asapi_03    2  TCONF  :  asapi_03.c:255: Remaining cases not appropriate for configuration: errno=EAFNOSUPPORT(97): Address family not supported by protocol
FAIL LTP CASE asapi_03 : 32
RUN LTP CASE ask_password.sh
/tmp/testsuite/musl-ltp-script/ltp_testcode.sh: line 15: ltp/testcases/bin/ask_password.sh: Exec format error
FAIL LTP CASE ask_password.sh : 126
RUN LTP CASE aslr01
tst_kconfig.c:71: TINFO: Couldn't locate kernel config!
tst_kconfig.c:207: TBROK: Cannot parse kernel .config
FAIL LTP CASE aslr01 : 2
RUN LTP CASE assign_password.sh
/tmp/testsuite/musl-ltp-script/ltp_testcode.sh: line 15: ltp/testcases/bin/assign_password.sh: Exec format error
FAIL LTP CASE assign_password.sh : 126
RUN LTP CASE atof01
atof01      1  TPASS  :  Test passed
atof01      2  TPASS  :  Test passed
atof01      3  TPASS  :  Test passed
atof01      4  TPASS  :  Test passed
FAIL LTP CASE atof01 : 0
RUN LTP CASE autogroup01
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
autogroup01.c:65: TCONF: autogroup not supported

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[480.767111 0:476 axfs::fops:297] [AxError::NotADirectory]
[480.769927 0:476 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE autogroup01 : 32
RUN LTP CASE bbr01.sh
/musl/ltp/testcases/bin/bbr01.sh: .: line 31: tcp_cc_lib.sh: not found
FAIL LTP CASE bbr01.sh : 2
RUN LTP CASE bbr02.sh
/musl/ltp/testcases/bin/bbr02.sh: .: line 37: tcp_cc_lib.sh: not found
FAIL LTP CASE bbr02.sh : 2
RUN LTP CASE bind01
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
[488.985253 0:488 arceos_posix_api::imp::net:333] sys_bind => Err(EINVAL)
bind01.c:60: TPASS: invalid salen : EINVAL (22)
bind01.c:60: TPASS: invalid socket : ENOTSOCK (88)
bind01.c:63: TPASS: INADDR_ANYPORT passed
[488.990573 0:488 arceos_posix_api::imp::net:333] sys_bind => Err(EINVAL)
bind01.c:60: TFAIL: UNIX-domain of current directory expected EAFNOSUPPORT: EINVAL (22)
bind01.c:60: TFAIL: non-local address succeeded
bind01.c:60: TPASS: sockfd is not a valid file descriptor : EBADF (9)
bind01.c:60: TFAIL: a component of addr prefix is not a directory expected ENOTDIR: ENOTSOCK (88)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[489.081344 0:485 axfs::fops:297] [AxError::NotADirectory]
[489.084427 0:485 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE bind01 : 0
RUN LTP CASE bind02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
bind02.c:52: TINFO: Switching credentials to user: nobody, group: nogroup
bind02.c:39: TFAIL: bind() succeeded

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[494.722899 0:490 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE bind02 : 0
RUN LTP CASE bind03
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
bind03.c:72: TBROK: bind(3, socket.1, 110) failed: ENOTSOCK (88)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[500.522996 0:495 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE bind03 : 2
RUN LTP CASE bind04
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
bind04.c:117: TINFO: Testing AF_UNIX pathname stream
bind04.c:121: TFAIL: bind() failed: ENOTSOCK (88)
bind04.c:117: TINFO: Testing AF_UNIX pathname seqpacket
bind04.c:118: TCONF: socket(1, 5, 0) failed: ESOCKTNOSUPPORT (94)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[506.511648 0:500 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE bind04 : 32
RUN LTP CASE bind05
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
bind05.c:131: TINFO: Testing AF_UNIX pathname datagram
bind05.c:134: TFAIL: bind() failed: ENOTSOCK (88)
bind05.c:131: TINFO: Testing AF_UNIX abstract datagram
bind05.c:134: TFAIL: bind() failed: ENOTSOCK (88)
bind05.c:131: TINFO: Testing IPv4 loop UDP variant 1
bind05.c:167: TPASS: Communication successful
bind05.c:131: TINFO: Testing IPv4 loop UDP variant 2
bind05.c:167: TPASS: Communication successful
bind05.c:131: TINFO: Testing IPv4 loop UDP-Lite
bind05.c:132: TCONF: socket(2, 2, 136) failed: EPROTONOSUPPORT (93)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[510.148204 0:505 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE bind05 : 32
RUN LTP CASE bind06
tst_kconfig.c:71: TINFO: Couldn't locate kernel config!
tst_kconfig.c:207: TBROK: Cannot parse kernel .config
FAIL LTP CASE bind06 : 2
RUN LTP CASE bind_noport01.sh
/musl/ltp/testcases/bin/bind_noport01.sh: .: line 33: tst_net.sh: not found
FAIL LTP CASE bind_noport01.sh : 2
RUN LTP CASE binfmt_misc01.sh
/musl/ltp/testcases/bin/binfmt_misc01.sh: .: line 63: binfmt_misc_lib.sh: not found
FAIL LTP CASE binfmt_misc01.sh : 2
RUN LTP CASE binfmt_misc02.sh
/musl/ltp/testcases/bin/binfmt_misc02.sh: .: line 108: binfmt_misc_lib.sh: not found
FAIL LTP CASE binfmt_misc02.sh : 2
RUN LTP CASE binfmt_misc_lib.sh
SKIP LTP CASE binfmt_misc_lib.sh : LTP shell helper library is not a standalone test
FAIL LTP CASE binfmt_misc_lib.sh : 32
RUN LTP CASE block_dev
block_dev    1  TCONF  :  tst_module.c:69: Failed to find module 'ltp_block_dev.ko'
block_dev    2  TCONF  :  tst_module.c:69: Remaining cases not appropriate for configuration
FAIL LTP CASE block_dev : 32
RUN LTP CASE bpf_map01
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
bpf_common.c:16: TINFO: Raising RLIMIT_MEMLOCK to 2097151
../../../../include/lapi/bpf.h:623: TCONF: syscall(280) __NR_bpf not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[524.079998 0:523 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE bpf_map01 : 32
RUN LTP CASE bpf_prog01
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
bpf_common.c:16: TINFO: Raising RLIMIT_MEMLOCK to 2097151
../../../../include/lapi/bpf.h:623: TCONF: syscall(280) __NR_bpf not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[527.801298 0:528 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE bpf_prog01 : 32
RUN LTP CASE bpf_prog02
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
bpf_common.c:16: TINFO: Raising RLIMIT_MEMLOCK to 2097151
tst_capability.c:17: TCONF: syscall(90) __NR_capget not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[531.466546 0:533 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE bpf_prog02 : 32
RUN LTP CASE bpf_prog03
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
bpf_common.c:16: TINFO: Raising RLIMIT_MEMLOCK to 2097151
tst_capability.c:17: TCONF: syscall(90) __NR_capget not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[535.124632 0:538 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE bpf_prog03 : 32
RUN LTP CASE bpf_prog04
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
bpf_common.c:16: TINFO: Raising RLIMIT_MEMLOCK to 2097151
tst_capability.c:17: TCONF: syscall(90) __NR_capget not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[538.761473 0:543 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE bpf_prog04 : 32
RUN LTP CASE bpf_prog05
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
bpf_common.c:16: TINFO: Raising RLIMIT_MEMLOCK to 2097151
tst_capability.c:17: TCONF: syscall(90) __NR_capget not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[542.493739 0:548 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE bpf_prog05 : 32
RUN LTP CASE bpf_prog06
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
bpf_common.c:16: TINFO: Raising RLIMIT_MEMLOCK to 2097151
tst_capability.c:17: TCONF: syscall(90) __NR_capget not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[546.231347 0:553 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE bpf_prog06 : 32
RUN LTP CASE bpf_prog07
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
bpf_common.c:16: TINFO: Raising RLIMIT_MEMLOCK to 2097151
tst_capability.c:17: TCONF: syscall(90) __NR_capget not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[549.891784 0:558 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE bpf_prog07 : 32
RUN LTP CASE brk01
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
brk01.c:24: TINFO: Testing libc variant
brk01.c:35: TCONF: brk() not implemented
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
brk01.c:21: TINFO: Testing syscall variant
brk01.c:70: TPASS: brk() works fine

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[553.428008 0:563 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE brk01 : 32
RUN LTP CASE brk02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
brk02.c:42: TINFO: Testing libc variant
brk02.c:53: TCONF: brk() not implemented
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
brk02.c:39: TINFO: Testing syscall variant
brk02.c:86: TPASS: munmap at least two VMAs of brk() passed

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[557.061114 0:571 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE brk02 : 32
RUN LTP CASE broken_ip-checksum.sh
/musl/ltp/testcases/bin/broken_ip-checksum.sh: .: line 21: tst_net.sh: not found
FAIL LTP CASE broken_ip-checksum.sh : 2
RUN LTP CASE broken_ip-dstaddr.sh
/musl/ltp/testcases/bin/broken_ip-dstaddr.sh: .: line 17: tst_net.sh: not found
FAIL LTP CASE broken_ip-dstaddr.sh : 2
RUN LTP CASE broken_ip-fragment.sh
/musl/ltp/testcases/bin/broken_ip-fragment.sh: .: line 21: tst_net.sh: not found
FAIL LTP CASE broken_ip-fragment.sh : 2
RUN LTP CASE broken_ip-ihl.sh
/musl/ltp/testcases/bin/broken_ip-ihl.sh: .: line 21: tst_net.sh: not found
FAIL LTP CASE broken_ip-ihl.sh : 2
RUN LTP CASE broken_ip-nexthdr.sh
/musl/ltp/testcases/bin/broken_ip-nexthdr.sh: .: line 17: tst_net.sh: not found
FAIL LTP CASE broken_ip-nexthdr.sh : 2
RUN LTP CASE broken_ip-plen.sh
/musl/ltp/testcases/bin/broken_ip-plen.sh: .: line 17: tst_net.sh: not found
FAIL LTP CASE broken_ip-plen.sh : 2
RUN LTP CASE broken_ip-protcol.sh
/musl/ltp/testcases/bin/broken_ip-protcol.sh: .: line 21: tst_net.sh: not found
FAIL LTP CASE broken_ip-protcol.sh : 2
RUN LTP CASE broken_ip-version.sh
/musl/ltp/testcases/bin/broken_ip-version.sh: .: line 17: tst_net.sh: not found
FAIL LTP CASE broken_ip-version.sh : 2
RUN LTP CASE busy_poll01.sh
/musl/ltp/testcases/bin/busy_poll01.sh: .: line 48: busy_poll_lib.sh: not found
FAIL LTP CASE busy_poll01.sh : 2
RUN LTP CASE busy_poll02.sh
/musl/ltp/testcases/bin/busy_poll02.sh: .: line 40: busy_poll_lib.sh: not found
FAIL LTP CASE busy_poll02.sh : 2
RUN LTP CASE busy_poll03.sh
/musl/ltp/testcases/bin/busy_poll03.sh: .: line 43: busy_poll_lib.sh: not found
FAIL LTP CASE busy_poll03.sh : 2
RUN LTP CASE busy_poll_lib.sh
SKIP LTP CASE busy_poll_lib.sh : LTP shell helper library is not a standalone test
FAIL LTP CASE busy_poll_lib.sh : 32
RUN LTP CASE cacheflush01
tst_test.c:1175: TCONF: system doesn't support cacheflush()
FAIL LTP CASE cacheflush01 : 32
RUN LTP CASE can_bcm01
tst_kernel.c:90: TINFO: uname.machine=riscv64 kernel is 64bit
tst_kernel.c:126: TWARN: expected file /lib/modules/6.0.0/modules.dep does not exist or not a file
tst_kernel.c:126: TWARN: expected file /lib/modules/6.0.0/modules.builtin does not exist or not a file
tst_test.c:1229: TCONF: vcan driver not available
FAIL LTP CASE can_bcm01 : 32
RUN LTP CASE can_filter
tst_kernel.c:126: TWARN: expected file /lib/modules/6.0.0/modules.dep does not exist or not a file
tst_kernel.c:126: TWARN: expected file /lib/modules/6.0.0/modules.builtin does not exist or not a file
tst_test.c:1229: TCONF: vcan driver not available
FAIL LTP CASE can_filter : 32
RUN LTP CASE can_rcv_own_msgs
tst_kernel.c:126: TWARN: expected file /lib/modules/6.0.0/modules.dep does not exist or not a file
tst_kernel.c:126: TWARN: expected file /lib/modules/6.0.0/modules.builtin does not exist or not a file
tst_test.c:1229: TCONF: vcan driver not available
FAIL LTP CASE can_rcv_own_msgs : 32
RUN LTP CASE cap_bounds_r
cap_bounds_r    1  TCONF  :  cap_bounds_r.c:103: System doesn't have POSIX capabilities.
FAIL LTP CASE cap_bounds_r : 32
RUN LTP CASE cap_bounds_rw
cap_bounds_rw    1  TCONF  :  cap_bounds_rw.c:161: System doesn't have POSIX capabilities.
FAIL LTP CASE cap_bounds_rw : 32
RUN LTP CASE cap_bset_inh_bounds
cap_bounds_r    1  TCONF  :  cap_bset_inh_bounds.c:132: System doesn't have sys/capability.h.
FAIL LTP CASE cap_bset_inh_bounds : 32
RUN LTP CASE capget01
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_capability.c:17: TCONF: syscall(90) __NR_capget not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[595.292699 0:616 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE capget01 : 32
RUN LTP CASE capget02
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
capget02.c:57: TCONF: syscall(90) __NR_capget not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[598.766096 0:621 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE capget02 : 32
RUN LTP CASE capset01
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
capset01.c:43: TCONF: syscall(90) __NR_capget not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[602.256966 0:626 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE capset01 : 32
RUN LTP CASE capset02
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
capset02.c:91: TCONF: syscall(91) __NR_capset not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[606.298471 0:631 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE capset02 : 32
RUN LTP CASE capset03
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
capset03.c:43: TCONF: syscall(91) __NR_capset not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[610.606463 0:636 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE capset03 : 32
RUN LTP CASE capset04
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
capset04.c:46: TCONF: syscall(90) __NR_capget not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[614.598199 0:641 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE capset04 : 32
RUN LTP CASE cfs_bandwidth01
tst_kconfig.c:71: TINFO: Couldn't locate kernel config!
tst_kconfig.c:207: TBROK: Cannot parse kernel .config
FAIL LTP CASE cfs_bandwidth01 : 2
RUN LTP CASE cgroup_core01
SKIP LTP CASE cgroup_core01 : cgroup unsupported by kernel
FAIL LTP CASE cgroup_core01 : 32
RUN LTP CASE cgroup_core02
SKIP LTP CASE cgroup_core02 : cgroup unsupported by kernel
FAIL LTP CASE cgroup_core02 : 32
RUN LTP CASE cgroup_core03
SKIP LTP CASE cgroup_core03 : cgroup unsupported by kernel
FAIL LTP CASE cgroup_core03 : 32
RUN LTP CASE cgroup_fj_common.sh
SKIP LTP CASE cgroup_fj_common.sh : cgroup unsupported by kernel
FAIL LTP CASE cgroup_fj_common.sh : 32
RUN LTP CASE cgroup_fj_function.sh
SKIP LTP CASE cgroup_fj_function.sh : cgroup unsupported by kernel
FAIL LTP CASE cgroup_fj_function.sh : 32
RUN LTP CASE cgroup_fj_proc
SKIP LTP CASE cgroup_fj_proc : cgroup unsupported by kernel
FAIL LTP CASE cgroup_fj_proc : 32
RUN LTP CASE cgroup_fj_stress.sh
SKIP LTP CASE cgroup_fj_stress.sh : cgroup unsupported by kernel
FAIL LTP CASE cgroup_fj_stress.sh : 32
RUN LTP CASE cgroup_lib.sh
SKIP LTP CASE cgroup_lib.sh : cgroup unsupported by kernel
FAIL LTP CASE cgroup_lib.sh : 32
RUN LTP CASE cgroup_regression_3_1.sh
SKIP LTP CASE cgroup_regression_3_1.sh : cgroup unsupported by kernel
FAIL LTP CASE cgroup_regression_3_1.sh : 32
RUN LTP CASE cgroup_regression_3_2.sh
SKIP LTP CASE cgroup_regression_3_2.sh : cgroup unsupported by kernel
FAIL LTP CASE cgroup_regression_3_2.sh : 32
RUN LTP CASE cgroup_regression_5_1.sh
SKIP LTP CASE cgroup_regression_5_1.sh : cgroup unsupported by kernel
FAIL LTP CASE cgroup_regression_5_1.sh : 32
RUN LTP CASE cgroup_regression_5_2.sh
SKIP LTP CASE cgroup_regression_5_2.sh : cgroup unsupported by kernel
FAIL LTP CASE cgroup_regression_5_2.sh : 32
RUN LTP CASE cgroup_regression_6_1.sh
SKIP LTP CASE cgroup_regression_6_1.sh : cgroup unsupported by kernel
FAIL LTP CASE cgroup_regression_6_1.sh : 32
RUN LTP CASE cgroup_regression_6_2.sh
SKIP LTP CASE cgroup_regression_6_2.sh : cgroup unsupported by kernel
FAIL LTP CASE cgroup_regression_6_2.sh : 32
RUN LTP CASE cgroup_regression_fork_processes
SKIP LTP CASE cgroup_regression_fork_processes : cgroup unsupported by kernel
FAIL LTP CASE cgroup_regression_fork_processes : 32
RUN LTP CASE cgroup_regression_getdelays
SKIP LTP CASE cgroup_regression_getdelays : cgroup unsupported by kernel
FAIL LTP CASE cgroup_regression_getdelays : 32
RUN LTP CASE cgroup_regression_test.sh
SKIP LTP CASE cgroup_regression_test.sh : cgroup unsupported by kernel
FAIL LTP CASE cgroup_regression_test.sh : 32
RUN LTP CASE cgroup_xattr
SKIP LTP CASE cgroup_xattr : cgroup unsupported by kernel
FAIL LTP CASE cgroup_xattr : 32
RUN LTP CASE change_password.sh
/tmp/testsuite/musl-ltp-script/ltp_testcode.sh: line 15: ltp/testcases/bin/change_password.sh: Exec format error
FAIL LTP CASE change_password.sh : 126
RUN LTP CASE chdir01
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_device.c:293: TWARN: Failed to create test_dev.img: ENOSPC (28)
tst_device.c:354: TBROK: Failed to acquire device

Summary:
passed   0
failed   0
broken   1
skipped  0
warnings 1
[629.500129 0:668 axfs::root:433] [AxError::IsADirectory]
[629.501223 0:668 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chdir01 : 6
RUN LTP CASE chdir04
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
chdir04.c:29: TFAIL: chdir() expected ENAMETOOLONG: ENOENT (2)
chdir04.c:29: TPASS: chdir() : ENOENT (2)
chdir04.c:29: TPASS: chdir() : EFAULT (14)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[632.962871 0:670 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chdir04 : 0
RUN LTP CASE check_envval
SKIP LTP CASE check_envval : requires LTP network environment
FAIL LTP CASE check_envval : 32
RUN LTP CASE check_icmpv4_connectivity
Usage: /musl/ltp/testcases/bin/check_icmpv4_connectivity source_interface_name destionation_ipv4_address
FAIL LTP CASE check_icmpv4_connectivity : 1
RUN LTP CASE check_icmpv6_connectivity
Usage: /musl/ltp/testcases/bin/check_icmpv6_connectivity source_interface_name destionation_ipv6_address
FAIL LTP CASE check_icmpv6_connectivity : 1
RUN LTP CASE check_keepcaps
keepcaps    1  TCONF  :  check_keepcaps.c:152: linux/securebits.h or libcap does not exist.
keepcaps    2  TCONF  :  check_keepcaps.c:152: Remaining cases not appropriate for configuration
FAIL LTP CASE check_keepcaps : 32
RUN LTP CASE check_netem
SKIP LTP CASE check_netem : requires LTP network environment
FAIL LTP CASE check_netem : 32
RUN LTP CASE check_pe
check_pe    1  TCONF  :  check_pe.c:83: System doesn't have sys/capability.h
FAIL LTP CASE check_pe : 32
RUN LTP CASE check_setkey
SKIP LTP CASE check_setkey : requires LTP network environment
FAIL LTP CASE check_setkey : 32
RUN LTP CASE check_simple_capset
System doesn't support full POSIX capabilities.
FAIL LTP CASE check_simple_capset : 1
RUN LTP CASE chmod01
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
chmod01.c:60: TINFO: Testing variant: verify permissions of file
chmod01.c:40: TPASS: chmod(testfile, 0000) passed
chmod01.c:50: TPASS: stat(testfile) mode=0000
chmod01.c:40: TPASS: chmod(testfile, 0007) passed
chmod01.c:50: TPASS: stat(testfile) mode=0007
chmod01.c:40: TPASS: chmod(testfile, 0070) passed
chmod01.c:50: TPASS: stat(testfile) mode=0070
chmod01.c:40: TPASS: chmod(testfile, 0700) passed
chmod01.c:50: TPASS: stat(testfile) mode=0700
chmod01.c:40: TPASS: chmod(testfile, 0777) passed
chmod01.c:50: TPASS: stat(testfile) mode=0777
chmod01.c:40: TPASS: chmod(testfile, 2777) passed
chmod01.c:50: TPASS: stat(testfile) mode=2777
chmod01.c:40: TPASS: chmod(testfile, 4777) passed
chmod01.c:50: TPASS: stat(testfile) mode=4777
chmod01.c:40: TPASS: chmod(testfile, 6777) passed
chmod01.c:50: TPASS: stat(testfile) mode=6777
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
chmod01.c:60: TINFO: Testing variant: verify permissions of directory
chmod01.c:40: TPASS: chmod(testdir_1, 0000) passed
chmod01.c:50: TPASS: stat(testdir_1) mode=0000
chmod01.c:40: TPASS: chmod(testdir_1, 0007) passed
chmod01.c:50: TPASS: stat(testdir_1) mode=0007
chmod01.c:40: TPASS: chmod(testdir_1, 0070) passed
chmod01.c:50: TPASS: stat(testdir_1) mode=0070
chmod01.c:40: TPASS: chmod(testdir_1, 0700) passed
chmod01.c:50: TPASS: stat(testdir_1) mode=0700
chmod01.c:40: TPASS: chmod(testdir_1, 0777) passed
chmod01.c:50: TPASS: stat(testdir_1) mode=0777
chmod01.c:40: TPASS: chmod(testdir_1, 2777) passed
chmod01.c:50: TPASS: stat(testdir_1) mode=2777
chmod01.c:40: TPASS: chmod(testdir_1, 4777) passed
chmod01.c:50: TPASS: stat(testdir_1) mode=4777
chmod01.c:40: TPASS: chmod(testdir_1, 6777) passed
chmod01.c:50: TPASS: stat(testdir_1) mode=6777

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[647.760451 0:691 axfs::root:433] [AxError::IsADirectory]
[647.761913 0:691 axfs::fops:297] [AxError::NotADirectory]
[647.762772 0:691 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chmod01 : 0
RUN LTP CASE chmod03
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
chmod03.c:42: TPASS: chmod(testfile, 1777) passed
chmod03.c:54: TPASS: stat(testfile) mode=101777
chmod03.c:42: TPASS: chmod(testdir_3, 1777) passed
chmod03.c:54: TPASS: stat(testdir_3) mode=41777

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[651.303178 0:699 axfs::root:433] [AxError::IsADirectory]
[651.304692 0:699 axfs::fops:297] [AxError::NotADirectory]
[651.305603 0:699 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chmod03 : 0
RUN LTP CASE chmod05
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
chmod05.c:76: TBROK: Group ID lookup failed: ENOTSOCK (88)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[654.803435 0:704 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chmod05 : 2
RUN LTP CASE chmod06
tst_test.c:1003: TINFO: Can't mount (null) at mntpoint (tmpfs): ENOSYS (38)
tst_test.c:1303: TINFO: Can't mount tmpfs read-only, falling back to block device...
tst_device.c:293: TWARN: Failed to create test_dev.img: ENOSPC (28)
tst_device.c:354: TBROK: Failed to acquire device

Summary:
passed   0
failed   0
broken   1
skipped  0
warnings 1
[659.297497 0:709 axfs::root:433] [AxError::IsADirectory]
[659.298667 0:709 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chmod06 : 6
RUN LTP CASE chmod07
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
chmod07.c:77: TINFO: getgrnam(users) failed - try fallback daemon
chmod07.c:77: TBROK: getgrnam(daemon) failed: ENOTSOCK (88)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[662.722429 0:711 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chmod07 : 2
RUN LTP CASE chown01
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
chown01.c:24: TPASS: chown(chown01_testfile,0,0) passed

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[666.224190 0:716 axfs::fops:297] [AxError::NotADirectory]
[666.225981 0:716 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chown01 : 0
RUN LTP CASE chown01_16
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
/code/ltp-full-20240524/testcases/kernel/syscalls/chown/../utils/compat_tst_16.h:153: TCONF: 16-bit version of chown() is not supported on your platform

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[669.656988 0:721 axfs::fops:297] [AxError::NotADirectory]
[669.658664 0:721 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chown01_16 : 32
RUN LTP CASE chown02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
chown02.c:46: TPASS: chown(testfile1, 0, 0) passed
chown02.c:46: TPASS: chown(testfile2, 0, 0) passed

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[673.077151 0:726 axfs::fops:297] [AxError::NotADirectory]
[673.078532 0:726 axfs::fops:297] [AxError::NotADirectory]
[673.080822 0:726 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chown02 : 0
RUN LTP CASE chown02_16
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
/code/ltp-full-20240524/testcases/kernel/syscalls/chown/../utils/compat_tst_16.h:153: TCONF: 16-bit version of chown() is not supported on your platform

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[676.560494 0:731 axfs::fops:297] [AxError::NotADirectory]
[676.561402 0:731 axfs::fops:297] [AxError::NotADirectory]
[676.562899 0:731 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chown02_16 : 32
RUN LTP CASE chown03
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
chown03.c:63: TPASS: chown(chown03_testfile, -1, 65534) passed

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[679.972938 0:736 axfs::fops:297] [AxError::NotADirectory]
[679.975390 0:736 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chown03 : 0
RUN LTP CASE chown03_16
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
/code/ltp-full-20240524/testcases/kernel/syscalls/chown/../utils/compat_tst_16.h:153: TCONF: 16-bit version of chown() is not supported on your platform

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[683.401917 0:741 axfs::fops:297] [AxError::NotADirectory]
[683.403638 0:741 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chown03_16 : 32
RUN LTP CASE chown04
tst_test.c:1003: TINFO: Can't mount (null) at mntpoint (tmpfs): ENOSYS (38)
tst_test.c:1303: TINFO: Can't mount tmpfs read-only, falling back to block device...
tst_device.c:293: TWARN: Failed to create test_dev.img: ENOSPC (28)
tst_device.c:354: TBROK: Failed to acquire device

Summary:
passed   0
failed   0
broken   1
skipped  0
warnings 1
[688.110961 0:746 axfs::root:433] [AxError::IsADirectory]
[688.112099 0:746 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chown04 : 6
RUN LTP CASE chown04_16
tst_test.c:1003: TINFO: Can't mount (null) at mntpoint (tmpfs): ENOSYS (38)
tst_test.c:1303: TINFO: Can't mount tmpfs read-only, falling back to block device...
tst_device.c:293: TWARN: Failed to create test_dev.img: ENOSPC (28)
tst_device.c:354: TBROK: Failed to acquire device

Summary:
passed   0
failed   0
broken   1
skipped  0
warnings 1
[692.699416 0:748 axfs::root:433] [AxError::IsADirectory]
[692.700646 0:748 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chown04_16 : 6
RUN LTP CASE chown05
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
chown05.c:42: TPASS: chown(testfile, 700, 701), change owner/group ids passed
chown05.c:42: TPASS: chown(testfile, 702, -1), change owner id only passed
chown05.c:42: TPASS: chown(testfile, 703, 701), change owner id only passed
chown05.c:42: TPASS: chown(testfile, -1, 704), change group id only passed
chown05.c:42: TPASS: chown(testfile, 703, 705), change group id only passed
chown05.c:42: TPASS: chown(testfile, -1, -1), no change passed

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[696.182348 0:750 axfs::fops:297] [AxError::NotADirectory]
[696.184468 0:750 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chown05 : 0
RUN LTP CASE chown05_16
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
/code/ltp-full-20240524/testcases/kernel/syscalls/chown/../utils/compat_tst_16.h:153: TCONF: 16-bit version of chown() is not supported on your platform

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[699.589854 0:755 axfs::fops:297] [AxError::NotADirectory]
[699.592068 0:755 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chown05_16 : 32
RUN LTP CASE chroot01
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
chroot01.c:23: TFAIL: unprivileged chroot() expected EPERM: ENOSYS (38)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[703.002774 0:760 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chroot01 : 0
RUN LTP CASE chroot02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
chroot02.c:30: TFAIL: chroot(/tmp/LTP_chrhCaCeM) failed: ENOSYS (38)
tst_test.c:1449: TBROK: Test haven't reported results!

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[706.823480 0:765 axfs::fops:297] [AxError::NotADirectory]
[706.825667 0:765 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chroot02 : 2
RUN LTP CASE chroot03
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
chroot03.c:65: TBROK: symlink(sym_dir1/,sym_dir2) failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[710.211020 0:771 axfs::fops:297] [AxError::NotADirectory]
[710.212694 0:771 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chroot03 : 2
RUN LTP CASE chroot04
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
chroot04.c:27: TFAIL: no search permission chroot() expected EACCES: ENOSYS (38)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[713.600102 0:776 axfs::root:433] [AxError::IsADirectory]
[713.601401 0:776 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chroot04 : 0
RUN LTP CASE cleanup_lvm.sh
/musl/ltp/testcases/bin/cleanup_lvm.sh: .: line 34: tst_test.sh: not found
FAIL LTP CASE cleanup_lvm.sh : 2
RUN LTP CASE clock_adjtime01
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_adjtime01.c:186: TINFO: Testing variant: syscall with old kernel spec
clock_adjtime.h:123: TCONF: syscall(266) __NR_clock_adjtime not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[717.927190 0:783 axfs::root:433] [AxError::IsADirectory]
tst_clocks.c:107: TCONF: clock_settime() not available
tst_wallclock.c:64: TBROK: tst_clock_settime() realtime failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   1
skipped  1
warnings 0
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_cloaMOkCk) failed: unlink(/tmp/LTP_cloaMOkCk) failed; errno=2: ENOENT
FAIL LTP CASE clock_adjtime01 : 34
RUN LTP CASE clock_adjtime02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_adjtime02.c:197: TINFO: Testing variant: syscall with old kernel spec
clock_adjtime.h:123: TCONF: syscall(266) __NR_clock_adjtime not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[721.398686 0:788 axfs::root:433] [AxError::IsADirectory]
tst_clocks.c:107: TCONF: clock_settime() not available
tst_wallclock.c:64: TBROK: tst_clock_settime() realtime failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   1
skipped  1
warnings 0
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_cloBMNfHK) failed: unlink(/tmp/LTP_cloBMNfHK) failed; errno=2: ENOENT
FAIL LTP CASE clock_adjtime02 : 34
RUN LTP CASE clock_getres01
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_getres01.c:62: TINFO: Testing variant: vDSO or syscall with libc spec
clock_getres01.c:88: TPASS: clock_getres(REALTIME, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(MONOTONIC, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(PROCESS_CPUTIME_ID, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(THREAD_CPUTIME_ID, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(CLOCK_MONOTONIC_RAW, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(CLOCK_REALTIME_COARSE, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(CLOCK_MONOTONIC_COARSE, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(CLOCK_BOOTTIME, ...) succeeded
clock_getres01.c:73: TCONF: clock_getres(CLOCK_REALTIME_ALARM, ...) NO SUPPORTED
clock_getres01.c:73: TCONF: clock_getres(CLOCK_BOOTTIME_ALARM, ...) NO SUPPORTED
clock_getres01.c:88: TPASS: clock_getres(-1, ...) succeeded
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_getres01.c:62: TINFO: Testing variant: vDSO or syscall with libc spec with NULL res
clock_getres01.c:88: TPASS: clock_getres(REALTIME, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(MONOTONIC, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(PROCESS_CPUTIME_ID, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(THREAD_CPUTIME_ID, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(CLOCK_MONOTONIC_RAW, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(CLOCK_REALTIME_COARSE, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(CLOCK_MONOTONIC_COARSE, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(CLOCK_BOOTTIME, ...) succeeded
clock_getres01.c:73: TCONF: clock_getres(CLOCK_REALTIME_ALARM, ...) NO SUPPORTED
clock_getres01.c:73: TCONF: clock_getres(CLOCK_BOOTTIME_ALARM, ...) NO SUPPORTED
clock_getres01.c:88: TPASS: clock_getres(-1, ...) succeeded
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_getres01.c:62: TINFO: Testing variant: syscall with old kernel spec
clock_getres01.c:88: TPASS: clock_getres(REALTIME, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(MONOTONIC, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(PROCESS_CPUTIME_ID, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(THREAD_CPUTIME_ID, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(CLOCK_MONOTONIC_RAW, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(CLOCK_REALTIME_COARSE, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(CLOCK_MONOTONIC_COARSE, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(CLOCK_BOOTTIME, ...) succeeded
clock_getres01.c:73: TCONF: clock_getres(CLOCK_REALTIME_ALARM, ...) NO SUPPORTED
clock_getres01.c:73: TCONF: clock_getres(CLOCK_BOOTTIME_ALARM, ...) NO SUPPORTED
clock_getres01.c:88: TPASS: clock_getres(-1, ...) succeeded
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_getres01.c:62: TINFO: Testing variant: syscall with old kernel spec with NULL res
clock_getres01.c:88: TPASS: clock_getres(REALTIME, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(MONOTONIC, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(PROCESS_CPUTIME_ID, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(THREAD_CPUTIME_ID, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(CLOCK_MONOTONIC_RAW, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(CLOCK_REALTIME_COARSE, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(CLOCK_MONOTONIC_COARSE, ...) succeeded
clock_getres01.c:88: TPASS: clock_getres(CLOCK_BOOTTIME, ...) succeeded
clock_getres01.c:73: TCONF: clock_getres(CLOCK_REALTIME_ALARM, ...) NO SUPPORTED
clock_getres01.c:73: TCONF: clock_getres(CLOCK_BOOTTIME_ALARM, ...) NO SUPPORTED
clock_getres01.c:88: TPASS: clock_getres(-1, ...) succeeded

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[725.282410 0:793 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clock_getres01 : 0
RUN LTP CASE clock_gettime01
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_gettime01.c:78: TINFO: Testing variant: vDSO or syscall with libc spec
clock_gettime01.c:111: TPASS: clock_gettime(2): clock CLOCK_REALTIME passed
clock_gettime01.c:111: TPASS: clock_gettime(2): clock CLOCK_MONOTONIC passed
clock_gettime01.c:111: TPASS: clock_gettime(2): clock CLOCK_PROCESS_CPUTIME_ID passed
clock_gettime01.c:111: TPASS: clock_gettime(2): clock CLOCK_THREAD_CPUTIME_ID passed
clock_gettime01.c:111: TPASS: clock_gettime(2): clock CLOCK_REALTIME_COARSE passed
clock_gettime01.c:111: TPASS: clock_gettime(2): clock CLOCK_MONOTONIC_COARSE passed
clock_gettime01.c:111: TPASS: clock_gettime(2): clock CLOCK_MONOTONIC_RAW passed
clock_gettime01.c:111: TPASS: clock_gettime(2): clock CLOCK_BOOTTIME passed
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_gettime01.c:78: TINFO: Testing variant: syscall with old kernel spec
clock_gettime01.c:111: TPASS: clock_gettime(2): clock CLOCK_REALTIME passed
clock_gettime01.c:111: TPASS: clock_gettime(2): clock CLOCK_MONOTONIC passed
clock_gettime01.c:111: TPASS: clock_gettime(2): clock CLOCK_PROCESS_CPUTIME_ID passed
clock_gettime01.c:111: TPASS: clock_gettime(2): clock CLOCK_THREAD_CPUTIME_ID passed
clock_gettime01.c:111: TPASS: clock_gettime(2): clock CLOCK_REALTIME_COARSE passed
clock_gettime01.c:111: TPASS: clock_gettime(2): clock CLOCK_MONOTONIC_COARSE passed
clock_gettime01.c:111: TPASS: clock_gettime(2): clock CLOCK_MONOTONIC_RAW passed
clock_gettime01.c:111: TPASS: clock_gettime(2): clock CLOCK_BOOTTIME passed

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[728.912470 0:807 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clock_gettime01 : 0
RUN LTP CASE clock_gettime02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_gettime02.c:102: TINFO: Testing variant: 0: syscall with old kernel spec
clock_gettime02.c:130: TPASS: clock_gettime(2): clock INVALID/UNKNOWN CLOCK failed as expected: EINVAL (22)
clock_gettime02.c:130: TPASS: clock_gettime(2): clock INVALID/UNKNOWN CLOCK failed as expected: EINVAL (22)
clock_gettime02.c:130: TPASS: clock_gettime(2): clock CLOCK_REALTIME failed as expected: EFAULT (14)
clock_gettime02.c:130: TPASS: clock_gettime(2): clock CLOCK_MONOTONIC failed as expected: EFAULT (14)
clock_gettime02.c:130: TPASS: clock_gettime(2): clock CLOCK_PROCESS_CPUTIME_ID failed as expected: EFAULT (14)
clock_gettime02.c:130: TPASS: clock_gettime(2): clock CLOCK_THREAD_CPUTIME_ID failed as expected: EFAULT (14)
clock_gettime02.c:130: TPASS: clock_gettime(2): clock CLOCK_REALTIME_COARSE failed as expected: EFAULT (14)
clock_gettime02.c:130: TPASS: clock_gettime(2): clock CLOCK_MONOTONIC_COARSE failed as expected: EFAULT (14)
clock_gettime02.c:130: TPASS: clock_gettime(2): clock CLOCK_MONOTONIC_RAW failed as expected: EFAULT (14)
clock_gettime02.c:130: TPASS: clock_gettime(2): clock CLOCK_BOOTTIME failed as expected: EFAULT (14)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[732.474878 0:815 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clock_gettime02 : 0
RUN LTP CASE clock_gettime03
tst_kconfig.c:71: TINFO: Couldn't locate kernel config!
tst_kconfig.c:207: TBROK: Cannot parse kernel .config
FAIL LTP CASE clock_gettime03 : 2
RUN LTP CASE clock_gettime04
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
vdso_helpers.c:27: TINFO: Couldn't find AT_SYSINFO_EHDR
vdso_helpers.c:72: TINFO: Couldn't find vdso_gettime()
vdso_helpers.c:76: TINFO: Couldn't find vdso_gettime64()
clock_gettime04.c:183: TPASS: CLOCK_REALTIME: Difference between successive readings is reasonable for following variants:
clock_gettime04.c:188: TINFO: 	- vDSO or syscall with libc spec
clock_gettime04.c:188: TINFO: 	- syscall with old kernel spec
clock_gettime04.c:188: TINFO: 	- vDSO with old kernel spec
clock_gettime04.c:188: TINFO: 	- gettimeofday
clock_gettime04.c:176: TFAIL: CLOCK_REALTIME_COARSE(vDSO or syscall with libc spec): Difference between successive readings greater than 5 ms (0): 5
clock_gettime04.c:183: TPASS: CLOCK_MONOTONIC: Difference between successive readings is reasonable for following variants:
clock_gettime04.c:188: TINFO: 	- vDSO or syscall with libc spec
clock_gettime04.c:188: TINFO: 	- syscall with old kernel spec
clock_gettime04.c:188: TINFO: 	- vDSO with old kernel spec
clock_gettime04.c:183: TPASS: CLOCK_MONOTONIC_COARSE: Difference between successive readings is reasonable for following variants:
clock_gettime04.c:188: TINFO: 	- vDSO or syscall with libc spec
clock_gettime04.c:188: TINFO: 	- syscall with old kernel spec
clock_gettime04.c:188: TINFO: 	- vDSO with old kernel spec
clock_gettime04.c:183: TPASS: CLOCK_MONOTONIC_RAW: Difference between successive readings is reasonable for following variants:
clock_gettime04.c:188: TINFO: 	- vDSO or syscall with libc spec
clock_gettime04.c:188: TINFO: 	- syscall with old kernel spec
clock_gettime04.c:188: TINFO: 	- vDSO with old kernel spec
clock_gettime04.c:183: TPASS: CLOCK_BOOTTIME: Difference between successive readings is reasonable for following variants:
clock_gettime04.c:188: TINFO: 	- vDSO or syscall with libc spec
clock_gettime04.c:188: TINFO: 	- syscall with old kernel spec
clock_gettime04.c:188: TINFO: 	- vDSO with old kernel spec

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[741.162423 0:822 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clock_gettime04 : 0
RUN LTP CASE clock_nanosleep01
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_nanosleep01.c:124: TINFO: Testing variant: vDSO or syscall with libc spec
clock_nanosleep01.c:139: TINFO: case NORMAL
clock_nanosleep01.c:218: TPASS: clock_nanosleep() failed with: EINVAL (22)
clock_nanosleep01.c:139: TINFO: case NORMAL
clock_nanosleep01.c:218: TPASS: clock_nanosleep() failed with: EINVAL (22)
clock_nanosleep01.c:139: TINFO: case NORMAL
clock_nanosleep01.c:218: TPASS: clock_nanosleep() failed with: EINVAL (22)
clock_nanosleep01.c:139: TINFO: case SEND_SIGINT
clock_nanosleep01.c:195: TFAIL: The clock_nanosleep() haven't updated timespec or it's not valid: SUCCESS (0)
clock_nanosleep01.c:139: TINFO: case BAD_TS_ADDR_REQ
clock_nanosleep01.c:143: TCONF: The libc wrapper may dereference req or rem
clock_nanosleep01.c:139: TINFO: case BAD_TS_ADDR_REM
clock_nanosleep01.c:143: TCONF: The libc wrapper may dereference req or rem
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_nanosleep01.c:124: TINFO: Testing variant: syscall with old kernel spec
clock_nanosleep01.c:139: TINFO: case NORMAL
clock_nanosleep01.c:218: TPASS: clock_nanosleep() failed with: EINVAL (22)
clock_nanosleep01.c:139: TINFO: case NORMAL
clock_nanosleep01.c:218: TPASS: clock_nanosleep() failed with: EINVAL (22)
clock_nanosleep01.c:139: TINFO: case NORMAL
clock_nanosleep01.c:212: TFAIL: returned 0, expected -1, expected errno: EOPNOTSUPP (95): SUCCESS (0)
clock_nanosleep01.c:139: TINFO: case SEND_SIGINT
clock_nanosleep01.c:195: TFAIL: The clock_nanosleep() haven't updated timespec or it's not valid: SUCCESS (0)
clock_nanosleep01.c:139: TINFO: case BAD_TS_ADDR_REQ
clock_nanosleep01.c:218: TPASS: clock_nanosleep() failed with: EFAULT (14)
clock_nanosleep01.c:139: TINFO: case BAD_TS_ADDR_REM
clock_nanosleep01.c:218: TPASS: clock_nanosleep() failed with: EFAULT (14)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[805.496611 0:828 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clock_nanosleep01 : 0
RUN LTP CASE clock_nanosleep02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_timer_test.c:357: TINFO: CLOCK_MONOTONIC resolution 1ns
tst_timer_test.c:369: TINFO: prctl(PR_GET_TIMERSLACK) = 0us
tst_test.c:1625: TINFO: Updating max runtime to 0h 00m 09s
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 39s
tst_timer_test.c:379: TINFO: Failed to set zero latency constraint: No such file or directory
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 1000us 500 iterations, threshold 401.01us
tst_timer_test.c:305: TINFO: min 1023us, max 1524us, median 1042us, trunc mean 1044.37us (discarded 25)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 2000us 500 iterations, threshold 402.01us
tst_timer_test.c:305: TINFO: min 2024us, max 2174us, median 2044us, trunc mean 2048.93us (discarded 25)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 5000us 300 iterations, threshold 405.04us
tst_timer_test.c:305: TINFO: min 5030us, max 8126us, median 5076us, trunc mean 5073.43us (discarded 15)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 10000us 100 iterations, threshold 410.33us
tst_timer_test.c:305: TINFO: min 10076us, max 10165us, median 10112us, trunc mean 10111.17us (discarded 5)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 25000us 50 iterations, threshold 426.29us
tst_timer_test.c:305: TINFO: min 25113us, max 25257us, median 25147us, trunc mean 25146.38us (discarded 2)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 100000us 10 iterations, threshold 537.00us
tst_timer_test.c:305: TINFO: min 100119us, max 100187us, median 100158us, trunc mean 100151.00us (discarded 1)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 1000000us 2 iterations, threshold 4400.00us
tst_timer_test.c:305: TINFO: min 1000152us, max 1000248us, median 1000152us, trunc mean 1000152.00us (discarded 1)
tst_timer_test.c:326: TPASS: Measured times are within thresholds

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[817.477692 0:839 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clock_nanosleep02 : 0
RUN LTP CASE clock_nanosleep03
tst_kconfig.c:71: TINFO: Couldn't locate kernel config!
tst_kconfig.c:207: TBROK: Cannot parse kernel .config
FAIL LTP CASE clock_nanosleep03 : 2
RUN LTP CASE clock_nanosleep04
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_nanosleep04.c:33: TINFO: Testing variant: vDSO or syscall with libc spec
clock_nanosleep04.c:57: TPASS: clock_nanosleep(2) passed for clock CLOCK_MONOTONIC
clock_nanosleep04.c:57: TPASS: clock_nanosleep(2) passed for clock CLOCK_REALTIME
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_nanosleep04.c:33: TINFO: Testing variant: syscall with old kernel spec
clock_nanosleep04.c:57: TPASS: clock_nanosleep(2) passed for clock CLOCK_MONOTONIC
clock_nanosleep04.c:57: TPASS: clock_nanosleep(2) passed for clock CLOCK_REALTIME

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[824.781993 0:847 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clock_nanosleep04 : 0
RUN LTP CASE clock_settime01
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_settime01.c:42: TINFO: Testing variant: vDSO or syscall with libc spec
clock_settime01.c:66: TFAIL: clock_settime(2) failed for clock CLOCK_REALTIME: ENOSYS (38)
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_settime01.c:42: TINFO: Testing variant: syscall with old kernel spec
../../../../include/tst_timer.h:241: TCONF: syscall(112) __NR_clock_settime not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[828.467852 0:855 axfs::root:433] [AxError::IsADirectory]
tst_clocks.c:107: TCONF: clock_settime() not available
tst_wallclock.c:64: TBROK: tst_clock_settime() realtime failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   1
skipped  1
warnings 0
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_clopPjLJK) failed: unlink(/tmp/LTP_clopPjLJK) failed; errno=2: ENOENT
FAIL LTP CASE clock_settime01 : 34
RUN LTP CASE clock_settime02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_settime02.c:106: TINFO: Testing variant: syscall with old kernel spec
../../../../include/tst_timer.h:241: TCONF: syscall(112) __NR_clock_settime not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[831.931838 0:863 axfs::root:433] [AxError::IsADirectory]
tst_clocks.c:107: TCONF: clock_settime() not available
tst_wallclock.c:64: TBROK: tst_clock_settime() realtime failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   1
skipped  1
warnings 0
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_clokEOfMk) failed: unlink(/tmp/LTP_clokEOfMk) failed; errno=2: ENOENT
FAIL LTP CASE clock_settime02 : 34
RUN LTP CASE clock_settime03
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clock_settime03.c:35: TINFO: Testing variant: syscall with old kernel spec
clock_settime03.c:62: TCONF: syscall(107) __NR_timer_create not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[835.495774 0:868 axfs::root:433] [AxError::IsADirectory]
tst_clocks.c:107: TCONF: clock_settime() not available
tst_wallclock.c:64: TBROK: tst_clock_settime() realtime failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   1
skipped  1
warnings 0
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_cloLooObN) failed: unlink(/tmp/LTP_cloLooObN) failed; errno=2: ENOENT
FAIL LTP CASE clock_settime03 : 34
RUN LTP CASE clone01
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clone01.c:37: TPASS: clone returned 877
clone01.c:43: TPASS: Child exited with 0

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[839.113333 0:873 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clone01 : 0
RUN LTP CASE clone02
clone02     1  TFAIL  :  clone02.c:139: clone() failed: TEST_ERRNO=ENOSYS(38): Function not implemented
clone02     2  TPASS  :  Test Passed
[842.688204 0:879 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clone02 : 1
RUN LTP CASE clone03
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clone03.c:38: TFAIL: pid(0) retval 886 != 0: SUCCESS (0)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[846.177936 0:882 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clone03 : 0
RUN LTP CASE clone04
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clone04.c:40: TPASS: NULL stack : EINVAL (22)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[849.544830 0:888 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clone04 : 0
RUN LTP CASE clone05
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clone05.c:48: TFAIL: child_exited retval 0 != 1: SUCCESS (0)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[853.175543 0:893 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clone05 : 0
RUN LTP CASE clone06
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clone06.c:38: TPASS: The environment variables of the child and the parent are the same 
tst_test.c:1449: TBROK: Test haven't reported results!

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[856.674866 0:899 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clone06 : 2
RUN LTP CASE clone07
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clone07.c:39: TBROK: waitpid(0,0x1ffffffc9c,0) failed: EINVAL (22)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[860.296996 0:905 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clone07 : 2
RUN LTP CASE clone08
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clone08.c:62: TINFO: running CLONE_PARENT
clone08.c:85: TBROK: CLONE_PARENT clone() failed: ENOSYS (38)
tst_test.c:1464: TBROK: Test 0 haven't reported results!

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[864.147754 0:911 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clone08 : 2
RUN LTP CASE clone09
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clone09.c:59: TINFO: create clone in a new netns with 'CLONE_NEWNET' flag
clone09.c:50: TBROK: clone(CLONE_NEWNET) failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[867.611663 0:917 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clone09 : 2
RUN LTP CASE clone301
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
../../../../include/lapi/sched.h:76: TCONF: syscall(435) __NR_clone3 not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[871.085675 0:922 axfs::fops:297] [AxError::NotADirectory]
[871.090401 0:922 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clone301 : 32
RUN LTP CASE clone302
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clone302.c:63: TPASS: sizeof(struct clone_args_minimal) == 64 (64)
../../../../include/lapi/sched.h:76: TCONF: syscall(435) __NR_clone3 not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[874.590510 0:927 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clone302 : 32
RUN LTP CASE clone303
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_cgroup.c:712: TINFO: Could not mount V2 CGroups on /tmp/cgroup_unified: ENOSYS (38)
tst_cgroup.c:880: TCONF: V2 'base' controller required, but it's mounted on V1

Summary:
passed   0
failed   0
broken   0
skipped  1
warnings 0
[878.040801 0:932 axfs::fops:297] [AxError::NotADirectory]
[878.042434 0:932 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clone303 : 32
RUN LTP CASE close01
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
close01.c:50: TPASS: close a file fd passed
close01.c:50: TPASS: close a pipe fd passed
close01.c:50: TPASS: close a socket fd passed

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[881.646988 0:934 axfs::fops:297] [AxError::NotADirectory]
[881.648632 0:934 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE close01 : 0
RUN LTP CASE close02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
close02.c:20: TPASS: close(-1) : EBADF (9)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[885.120578 0:939 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE close02 : 0
RUN LTP CASE close_range01
tst_device.c:293: TWARN: Failed to create test_dev.img: ENOSPC (28)
tst_device.c:354: TBROK: Failed to acquire device

HINT: You _MAY_ be missing kernel fixes:

https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=fec8a6a69103

Summary:
passed   0
failed   0
broken   1
skipped  0
warnings 1
[889.774537 0:944 axfs::root:433] [AxError::IsADirectory]
[889.775741 0:944 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE close_range01 : 6
RUN LTP CASE close_range02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
../../../../include/lapi/close_range.h:25: TCONF: syscall(436) __NR_close_range not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[893.268303 0:946 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE close_range02 : 32
RUN LTP CASE cmdlib.sh
SKIP LTP CASE cmdlib.sh : LTP shell helper library is not a standalone test
FAIL LTP CASE cmdlib.sh : 32
RUN LTP CASE cn_pec.sh
/musl/ltp/testcases/bin/cn_pec.sh: .: line 147: tst_test.sh: not found
FAIL LTP CASE cn_pec.sh : 2
RUN LTP CASE confstr01
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 14
confstr01.c:75: TPASS: confstr PATH = '/bin:/usr/bin'
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 0
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 0
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 1
confstr01.c:75: TPASS: confstr POSIX_V7_ILP32_OFF32_CFLAGS = ''
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 1
confstr01.c:75: TPASS: confstr POSIX_V7_ILP32_OFF32_LDFLAGS = ''
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 1
confstr01.c:75: TPASS: confstr POSIX_V7_ILP32_OFF32_LIBS = ''
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 1
confstr01.c:75: TPASS: confstr POSIX_V7_ILP32_OFFBIG_CFLAGS = ''
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 1
confstr01.c:75: TPASS: confstr POSIX_V7_ILP32_OFFBIG_LDFLAGS = ''
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 1
confstr01.c:75: TPASS: confstr POSIX_V7_ILP32_OFFBIG_LIBS = ''
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 1
confstr01.c:75: TPASS: confstr POSIX_V7_LP64_OFF64_CFLAGS = ''
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 1
confstr01.c:75: TPASS: confstr POSIX_V7_LP64_OFF64_LDFLAGS = ''
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 1
confstr01.c:75: TPASS: confstr POSIX_V7_LP64_OFF64_LIBS = ''
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 1
confstr01.c:75: TPASS: confstr POSIX_V7_LPBIG_OFFBIG_CFLAGS = ''
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 1
confstr01.c:75: TPASS: confstr POSIX_V7_LPBIG_OFFBIG_LDFLAGS = ''
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 1
confstr01.c:75: TPASS: confstr POSIX_V7_LPBIG_OFFBIG_LIBS = ''
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 1
confstr01.c:75: TPASS: confstr POSIX_V7_WIDTH_RESTRICTED_ENVS = ''
confstr01.c:61: TPASS: confstr(test_cases[i].value, NULL, (size_t)0) returned 1
confstr01.c:75: TPASS: confstr V7_ENV = ''

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[897.848738 0:954 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE confstr01 : 0
RUN LTP CASE connect01
connect01    1  TPASS  :  bad file descriptor successful
connect01    2  TPASS  :  invalid socket buffer successful
[901.520961 0:959 arceos_posix_api::imp::net:352] sys_connect => Err(EINVAL)
connect01    3  TPASS  :  invalid salen successful
connect01    4  TPASS  :  invalid socket successful
[901.526564 0:959 axnet::smoltcp_impl::tcp:197] [AxError::ConnectionRefused] socket connect() failed
[901.528982 0:959 arceos_posix_api::imp::net:352] sys_connect => Err(ECONNREFUSED)
connect01    5  TBROK  :  connect01.c:226: connect(3, 0.0.0.0:49183, 16) failed: errno=ECONNREFUSED(111): Connection refused
connect01    6  TBROK  :  connect01.c:226: Remaining cases broken
FAIL LTP CASE connect01 : 2
RUN LTP CASE connect02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
connect02.c:53: TCONF: socket(10, 1, 6) failed: EAFNOSUPPORT (97)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[905.348083 0:962 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE connect02 : 32
RUN LTP CASE copy_file_range01
tst_device.c:293: TWARN: Failed to create test_dev.img: ENOSPC (28)
tst_device.c:354: TBROK: Failed to acquire device

Summary:
passed   0
failed   0
broken   1
skipped  0
warnings 1
[909.942487 0:967 axfs::root:433] [AxError::IsADirectory]
[909.943514 0:967 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE copy_file_range01 : 6
RUN LTP CASE copy_file_range02
tst_device.c:293: TWARN: Failed to create test_dev.img: ENOSPC (28)
tst_device.c:354: TBROK: Failed to acquire device

Summary:
passed   0
failed   0
broken   1
skipped  0
warnings 1
[914.469546 0:969 axfs::root:433] [AxError::IsADirectory]
[914.470709 0:969 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE copy_file_range02 : 6
RUN LTP CASE copy_file_range03
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
copy_file_range.h:36: TINFO: Testing libc copy_file_range()
copy_file_range03.c:42: TBROK: copy_file_range unexpectedly failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[919.961025 0:971 axfs::fops:297] [AxError::NotADirectory]
[919.961938 0:971 axfs::fops:297] [AxError::NotADirectory]
[919.963268 0:971 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE copy_file_range03 : 2
RUN LTP CASE cp_tests.sh
/musl/ltp/testcases/bin/cp_tests.sh: .: line 93: tst_test.sh: not found
FAIL LTP CASE cp_tests.sh : 2
RUN LTP CASE cpio_tests.sh
/musl/ltp/testcases/bin/cpio_tests.sh: .: line 48: tst_test.sh: not found
FAIL LTP CASE cpio_tests.sh : 2
RUN LTP CASE cpuacct.sh
/musl/ltp/testcases/bin/cpuacct.sh: .: line 179: tst_test.sh: not found
FAIL LTP CASE cpuacct.sh : 2
RUN LTP CASE cpuacct_task
Usage: ltp/testcases/bin/cpuacct_task /cgroup/.../tasks
FAIL LTP CASE cpuacct_task : 1
RUN LTP CASE cpuctl_def_task01
cpu_controller_tests    1  TBROK  :  cpuctl_def_task01.c:120: Invalid input parameters
cpu_controller_tests    2  TBROK  :  cpuctl_def_task01.c:120: Remaining cases broken
FAIL LTP CASE cpuctl_def_task01 : 2
RUN LTP CASE cpuctl_def_task02
cpu_controller_test04    1  TBROK  :  cpuctl_def_task02.c:140: Invalid test number passed
cpu_controller_test04    2  TBROK  :  cpuctl_def_task02.c:140: Remaining cases broken
FAIL LTP CASE cpuctl_def_task02 : 2
RUN LTP CASE cpuctl_def_task03
cpu_controller_test06    1  TBROK  :  cpuctl_def_task03.c:136: Invalid test number passed
cpu_controller_test06    2  TBROK  :  cpuctl_def_task03.c:136: Remaining cases broken
FAIL LTP CASE cpuctl_def_task03 : 2
RUN LTP CASE cpuctl_def_task04
cpu_controller_test06    1  TBROK  :  cpuctl_def_task04.c:139: Invalid test number passed
cpu_controller_test06    2  TBROK  :  cpuctl_def_task04.c:139: Remaining cases broken
FAIL LTP CASE cpuctl_def_task04 : 2
RUN LTP CASE cpuctl_fj_cpu-hog
cpuctl_fj_cpu-hog: sigsuspend(): Function not implemented
FAIL LTP CASE cpuctl_fj_cpu-hog : 1
RUN LTP CASE cpuctl_fj_simple_echo
usage: cpuctl_fj_simple_echo STRING [ostream]
FAIL LTP CASE cpuctl_fj_simple_echo : 1
RUN LTP CASE cpuctl_latency_check_task
Invalid #args received from script. Exiting test..
FAIL LTP CASE cpuctl_latency_check_task : 1
RUN LTP CASE cpuctl_latency_test
cpuctl_latency_test: TBROK	 Invalid #args received from script The test will run without any cpu load
FAIL LTP CASE cpuctl_latency_test : 22
RUN LTP CASE cpuctl_test01
cpuctl_test01    1  TBROK  :  cpuctl_test01.c:120: Invalid input parameters
cpuctl_test01    2  TBROK  :  cpuctl_test01.c:120: Remaining cases broken
FAIL LTP CASE cpuctl_test01 : 2
RUN LTP CASE cpuctl_test02
cpuctl_test02    1  TBROK  :  cpuctl_test02.c:144: Invalid test number passed
cpuctl_test02    2  TBROK  :  cpuctl_test02.c:144: Remaining cases broken
FAIL LTP CASE cpuctl_test02 : 2
RUN LTP CASE cpuctl_test03
cpuctl_test03    1  TBROK  :  cpuctl_test03.c:139: Invalid test number passed
cpuctl_test03    2  TBROK  :  cpuctl_test03.c:139: Remaining cases broken
FAIL LTP CASE cpuctl_test03 : 2
RUN LTP CASE cpuctl_test04
cpuctl_test04    1  TBROK  :  cpuctl_test04.c:140: Invalid test number passed
cpuctl_test04    2  TBROK  :  cpuctl_test04.c:140: Remaining cases broken
FAIL LTP CASE cpuctl_test04 : 2
RUN LTP CASE cpufreq_boost
cpufreq_boost    1  TCONF  :  cpufreq_boost.c:107: overclock not supported
cpufreq_boost    2  TCONF  :  cpufreq_boost.c:107: Remaining cases not appropriate for configuration
FAIL LTP CASE cpufreq_boost : 32
RUN LTP CASE cpuhotplug01.sh
SKIP LTP CASE cpuhotplug01.sh : CPU hotplug unsupported by kernel
FAIL LTP CASE cpuhotplug01.sh : 32
RUN LTP CASE cpuhotplug02.sh
SKIP LTP CASE cpuhotplug02.sh : CPU hotplug unsupported by kernel
FAIL LTP CASE cpuhotplug02.sh : 32
RUN LTP CASE cpuhotplug03.sh
SKIP LTP CASE cpuhotplug03.sh : CPU hotplug unsupported by kernel
FAIL LTP CASE cpuhotplug03.sh : 32
RUN LTP CASE cpuhotplug04.sh
SKIP LTP CASE cpuhotplug04.sh : CPU hotplug unsupported by kernel
FAIL LTP CASE cpuhotplug04.sh : 32
RUN LTP CASE cpuhotplug05.sh
SKIP LTP CASE cpuhotplug05.sh : CPU hotplug unsupported by kernel
FAIL LTP CASE cpuhotplug05.sh : 32
RUN LTP CASE cpuhotplug06.sh
SKIP LTP CASE cpuhotplug06.sh : CPU hotplug unsupported by kernel
FAIL LTP CASE cpuhotplug06.sh : 32
RUN LTP CASE cpuhotplug07.sh
SKIP LTP CASE cpuhotplug07.sh : CPU hotplug unsupported by kernel
FAIL LTP CASE cpuhotplug07.sh : 32
RUN LTP CASE cpuhotplug_do_disk_write_loop
SKIP LTP CASE cpuhotplug_do_disk_write_loop : CPU hotplug unsupported by kernel
FAIL LTP CASE cpuhotplug_do_disk_write_loop : 32
RUN LTP CASE cpuhotplug_do_kcompile_loop
SKIP LTP CASE cpuhotplug_do_kcompile_loop : CPU hotplug unsupported by kernel
FAIL LTP CASE cpuhotplug_do_kcompile_loop : 32
RUN LTP CASE cpuhotplug_do_spin_loop
SKIP LTP CASE cpuhotplug_do_spin_loop : CPU hotplug unsupported by kernel
FAIL LTP CASE cpuhotplug_do_spin_loop : 32
RUN LTP CASE cpuhotplug_hotplug.sh
SKIP LTP CASE cpuhotplug_hotplug.sh : CPU hotplug unsupported by kernel
FAIL LTP CASE cpuhotplug_hotplug.sh : 32
RUN LTP CASE cpuhotplug_report_proc_interrupts
SKIP LTP CASE cpuhotplug_report_proc_interrupts : CPU hotplug unsupported by kernel
FAIL LTP CASE cpuhotplug_report_proc_interrupts : 32
RUN LTP CASE cpuhotplug_testsuite.sh
SKIP LTP CASE cpuhotplug_testsuite.sh : CPU hotplug unsupported by kernel
FAIL LTP CASE cpuhotplug_testsuite.sh : 32
RUN LTP CASE cpuset01
tst_test.c:1175: TCONF: test requires libnuma development packages with LIBNUMA_API_VERSION >= 2
FAIL LTP CASE cpuset01 : 32
RUN LTP CASE crash01
crash01     0  TINFO  :  crashme +2000.80 967 100
crash01     1  TPASS  :  we're still here, OS seems to be robust
exit status ... number of cases
[967.989423 0:1026 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE crash01 : 0
RUN LTP CASE crash02
crash02     0  TINFO  :  crashme02 127 971 100
0000: syscall(51, 0, 0, 0x4512a85d, 0x7622a4ed, 0, 0x619fb929, 0x11dc36df)
0001: syscall(38, 0xc44e0487, 0xbd2, 0, 0, 0xb4dac524, 0x8596, 0xaaa81814)
0002: syscall(9, 0, 0xd9b7, 0xd6237cd8, 0xf7, 0, 0x26ad5b2d, 0x24)
0003: syscall(13, 0, 0x62e472c5, 0x7f47567e, 0x3a22dbb0, 0, 0, 0x6c6b)
0004: syscall(76, 0, 0, 0, 0, 0x35ed1e92, 0, 0x90dca32d)
0005: syscall(102, 0x59bcd874, 0, 0x59c55b55, 0xbb205e02, 0, 0x9b9ae020, 0x629a9f03)
0006: syscall(17, 0x7aab3ab7, 0x153cf983, 0x5735cb0a, 0xfcb1bf45, 0x4a578fd2, 0, 0)
0007: syscall(20, 0, 0xc311ad1d, 0, 0, 0xf7b, 0x7d419b73, 0)
0008: syscall(24, 0, 0, 0x6016, 0x4d4a8a63, 0x126d4454, 0xa3aeffa2, 0x88b9)
0009: syscall(120, 0, 0, 0, 0, 0xac6e7e16, 0, 0x4820)
0010: syscall(9, 0xd10db8a2, 0xb49ed288, 0, 0x5b736721, 0, 0, 0)
0011: syscall(38, 0xfa575947, 0xf1cc81ff, 0xa0, 0, 0, 0x4e23e51, 0x19ad6d4)
0012: syscall(120, 0xc925ad21, 0x22f22c74, 0xc210af50, 0xe42f71c6, 0xa4e08624, 0xc515bc07, 0xf2f2d1c7)
0013: syscall(66, 0, 0, 0x19b4851d, 0xe2, 0x5c7b4c, 0xb8e48a, 0xc5)
0014: syscall(90, 0x75, 0x89621b5e, 0xf41947e4, 0, 0, 0, 0xe8c308d2)
0015: syscall(20, 0x755192c6, 0x72acfb7f, 0xff322724, 0, 0xaba31624, 0xa62ff24e, 0x167)
0016: syscall(52, 0x742da6d1, 0x753980cf, 0x74a2b85a, 0xfd51bfcf, 0, 0x7e2a54f0, 0)
0017: syscall(88, 0xfce0, 0x71a1b5b8, 0xe9a0ee7c, 0xee6fe244, 0x79e577a, 0x24eb2175, 0xc)
0018: syscall(122, 0x5b825e7f, 0, 0x612474c3, 0, 0x3b2d8984, 0xe35472a7, 0x827908a8)
0019: syscall(18, 0x7d52247, 0, 0, 0, 0xf0559093, 0, 0xa1385664)
0020: syscall(22, 0, 0, 0x517d3dce, 0xe6e748a8, 0xcd5bbcd1, 0, 0x19bc8243)
0021: syscall(18, 0xd7fd7196, 0, 0xae46016d, 0x70f25e5c, 0x2ac1c7ef, 0x84, 0xa22d)
0022: syscall(77, 0, 0x3119da20, 0, 0x846a56a6, 0, 0x1348404b, 0)
0023: syscall(5, 0x3cfb3364, 0, 0, 0xcc3cd5f3, 0xc0, 0x4692eb0d, 0x87bc3e9f)
0024: syscall(86, 0xe08782e6, 0, 0, 0xa690033d, 0x61c50a8c, 0xbab11e36, 0)
0025: syscall(82, 0, 0x7d, 0, 0, 0xcec8c96d, 0, 0)
0026: syscall(59, 0xb579066, 0x75, 0x17799826, 0x60a86a88, 0x821dc11, 0xf8, 0)
0027: syscall(17, 0x12c3, 0x9e150ceb, 0, 0, 0, 0, 0x3760)
0028: syscall(34, 0xd4deccd8, 0x6988, 0xc37e6c9c, 0, 0, 0xc2d43109, 0)
0029: syscall(10, 0xeff1e45b, 0xcea5beb7, 0, 0x65acd0d5, 0, 0, 0)
0030: syscall(119, 0x47026065, 0, 0, 0xe8be51d8, 0x7186d786, 0x96234a29, 0x8b60)
0031: syscall(126, 0x18472b8, 0x472abef, 0, 0, 0xf3bfe627, 0x2c26a33c, 0)
0032: syscall(37, 0, 0x41d43c94, 0, 0x932ce4a1, 0, 0x8045875d, 0x3)
0033: syscall(3, 0x11cc3130, 0x44bdd7c1, 0xf9, 0xc03b4c57, 0, 0, 0)
0034: syscall(115, 0xbd9, 0xa3cea0b8, 0x43, 0, 0xe6f026ab, 0xe862d128, 0)
0035: syscall(40, 0, 0x3b9ff9ed, 0x7776bd80, 0, 0xe3561f02, 0x173838f6, 0)
0036: syscall(31, 0xb4f874bb, 0x7d689c26, 0xbc73d397, 0, 0x340c2663, 0x3128e6f6, 0x937ba3a0)
0037: syscall(115, 0xf0d0f4c8, 0, 0x9d7acbb0, 0, 0x6ee4885b, 0, 0x97a1fe21)
0038: syscall(102, 0, 0, 0, 0xd19b220, 0, 0, 0xc8aa4e48)
0039: syscall(82, 0, 0x99d6c5e, 0, 0, 0xa81c6899, 0xb662ff96, 0x4c)
0040: syscall(77, 0, 0x7cf2743b, 0x6cff586c, 0x90b423e1, 0, 0, 0)
0041: syscall(87, 0, 0x6c327d7e, 0x2be3b90a, 0xf3bc, 0x45, 0x59, 0x87ed178c)
0042: syscall(71, 0xd132f34f, 0, 0x4a887c61, 0, 0, 0x4029f5f5, 0x57110f7d)
0043: syscall(35, 0, 0xe98cc3c0, 0xfe9e151d, 0x74a254cb, 0, 0, 0)
0044: syscall(100, 0x45d9ccfa, 0xb4429299, 0x1cf96f6a, 0, 0, 0, 0)
0045: syscall(96, 0x403a192a, 0, 0, 0, 0xb47326f, 0, 0)
0046: syscall(89, 0, 0xbe42a7e0, 0, 0, 0x39cdc1b1, 0, 0xe9f0a343)
0047: syscall(64, 0xecb98a1, 0, 0x8643d019, 0x52, 0x8d767d97, 0xbc2aea3e, 0x964f5cd8)
0048: syscall(47, 0xbd4267b5, 0, 0, 0, 0x7b25d2c7, 0x9388, 0)
0049: syscall(64, 0, 0x1fb44cff, 0xb624d744, 0x33694d28, 0xc7, 0xc7569cea, 0x14842a0f)
0050: syscall(110, 0, 0, 0, 0, 0, 0x6b24, 0x36e3fd2f)
0051: syscall(4, 0, 0xdc9d012d, 0x5662e54e, 0, 0, 0, 0xb30c)
0052: syscall(82, 0, 0x7b, 0xa7f4cb57, 0xbda597f6, 0xc409, 0x6fe29199, 0x29b1ab9d)
0053: syscall(73, 0x2174b40c, 0, 0x49b09379, 0xa4c4f560, 0, 0, 0xd2ddcb8d)
0054: syscall(65, 0, 0, 0x79c81afe, 0, 0xc803be03, 0xf5a3d299, 0x3a6e02f)
0055: syscall(113, 0, 0, 0xa1af9548, 0xca4bc6b2, 0, 0x560f1140, 0)
0056: syscall(103, 0, 0xd0e25186, 0, 0xa864a830, 0, 0, 0)
0057: syscall(90, 0xfd8b9934, 0x5c, 0, 0x8b1e8077, 0x30f46f95, 0, 0x18b3cecd)
0058: syscall(62, 0x4f1d69e0, 0xd8909aa5, 0, 0, 0x2e3d2263, 0, 0xda00cccc)
0059: syscall(77, 0xc710d69c, 0x1f5b322, 0x73, 0, 0x5c, 0, 0)
0060: syscall(39, 0xf0428d96, 0, 0x7974c260, 0x8bde2bd9, 0, 0x3b1c618e, 0)
0061: syscall(84, 0x8899e19, 0xe29f, 0xff21d66b, 0, 0x2a2a030b, 0x37f874ed, 0x8ddc7683)
0062: syscall(23, 0, 0, 0xd7e320b2, 0, 0, 0xa641d501, 0xfa)
0063: syscall(120, 0xe1678bc, 0x3caf04c1, 0xd3ea51af, 0, 0x77f3d11e, 0, 0x3a68)
0064: syscall(14, 0x44a52e95, 0xe7f9051, 0, 0x20f17c13, 0x886b2e8, 0, 0x806b92d2)
0065: syscall(34, 0x4a6644b2, 0x5a199edb, 0xee8c386f, 0, 0, 0, 0)
0066: syscall(111, 0, 0, 0, 0x6aa50f74, 0x7a2a024a, 0x1397d692, 0)
0067: syscall(18, 0xcb3124c2, 0, 0x38, 0xdee3ac44, 0, 0, 0)
0068: syscall(50, 0, 0xb6b90e3a, 0, 0, 0, 0xed1a16d0, 0x333d221a)
0069: syscall(86, 0xa0dbce93, 0xacc3bf55, 0xd6332872, 0x5cb945e5, 0x85, 0, 0x5b9e5d01)
0070: syscall(17, 0xa3800e43, 0, 0, 0x6d24ca51, 0, 0, 0)
0071: syscall(46, 0x2c9d2544, 0, 0, 0x34004b24, 0xd8, 0xc803248f, 0xeff28125)
0072: syscall(31, 0xe9ad9226, 0x4d994a95, 0x1227e533, 0, 0xf2339bf5, 0, 0)
0073: syscall(17, 0x2c258a54, 0, 0, 0, 0x397d722c, 0x1e82ba14, 0xc9d5d45c)
0074: syscall(20, 0, 0, 0xae, 0, 0x35954527, 0, 0)
0075: syscall(94, 0, 0x18c7, 0xe8, 0, 0xf6, 0, 0xde922d4b)
crash02     1  TPASS  :  we're still here, OS seems to be robust
[971.636615 0:1031 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE crash02 : 0
RUN LTP CASE creat01
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
creat01.c:50: TPASS: creat() truncated file to 0 bytes
creat01.c:55: TPASS: file was created and written to successfully
creat01.c:60: TPASS: read failed expectedly: EACCES (13)
creat01.c:50: TPASS: creat() truncated file to 0 bytes
creat01.c:55: TPASS: file was created and written to successfully
creat01.c:60: TPASS: read failed expectedly: EACCES (13)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[975.344080 0:1111 axfs::fops:297] [AxError::NotADirectory]
[975.345891 0:1111 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE creat01 : 0
RUN LTP CASE creat03
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
creat03.c:36: TINFO: Created file has mode = 0100674
creat03.c:41: TPASS: save text bit cleared

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[978.959268 0:1116 axfs::fops:297] [AxError::NotADirectory]
[978.960895 0:1116 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE creat03 : 0
RUN LTP CASE creat04
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
creat04.c:40: TFAIL: call succeeded unexpectedly
tst_test.c:1464: TBROK: Test 0 haven't reported results!

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[982.409410 0:1121 axfs::fops:297] [AxError::NotADirectory]
[982.410938 0:1121 axfs::root:433] [AxError::IsADirectory]
[982.412010 0:1121 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE creat04 : 2
RUN LTP CASE creat05
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
creat05.c:49: TINFO: getdtablesize() = 1024
creat05.c:59: TINFO: Opened additional #1021 fds
creat05.c:30: TFAIL: call succeeded unexpectedly

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[985.934677 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.936478 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.938665 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.939264 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.939872 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.940467 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.942198 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.944051 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.945211 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.945807 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.946370 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.947676 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.949497 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.950961 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.952343 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.953609 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.955305 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.956459 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.957557 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.958224 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.959115 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.960856 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.962500 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.964027 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.964699 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.966537 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.967955 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.968524 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.969118 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.969745 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.971800 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.973299 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.974700 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.975278 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.976292 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.978062 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.978916 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.979775 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.981709 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.982315 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.982913 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.983469 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.984764 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.986286 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.988380 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.989840 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.991015 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.992169 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.992831 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.994534 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.995256 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.996244 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.997883 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.998448 0:1127 axfs::fops:297] [AxError::NotADirectory]
[985.999968 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.001173 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.001810 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.002416 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.003957 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.005833 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.007102 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.007790 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.009558 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.010353 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.011773 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.012907 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.013547 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.015342 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.016420 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.017794 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.018537 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.019902 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.021413 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.022677 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.023782 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.024368 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.025568 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.027355 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.028194 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.028875 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.029535 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.031397 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.033034 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.034658 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.035266 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.036406 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.038048 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.039082 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.039710 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.040396 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.042114 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.043766 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.044788 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.045353 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.046551 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.048233 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.049325 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.049942 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.051114 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.052697 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.054113 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.055170 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.056160 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.057774 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.058630 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.059211 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.059991 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.061791 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.063446 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.064339 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.065017 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.065645 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.067288 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.068964 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.070350 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.070950 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.072407 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.073817 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.074425 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.076142 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.077343 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.078491 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.079652 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.080863 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.081461 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.082218 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.083315 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.084182 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.084823 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.086107 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.086765 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.087347 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.088270 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.089451 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.090252 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.091259 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.091964 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.092546 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.093339 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.094477 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.095150 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.095764 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.097106 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.098111 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.098718 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.099988 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.100817 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.101510 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.102442 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.103125 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.103738 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.105001 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.105849 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.106422 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.107444 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.108249 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.108999 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.109562 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.111050 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.112177 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.112836 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.114140 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.114934 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.115573 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.116851 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.117745 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.118309 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.119156 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.120449 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.121132 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.122082 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.123414 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.124425 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.125091 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.125777 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.127018 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.127817 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.128378 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.129351 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.130733 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.131361 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.132281 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.133187 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.134230 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.135060 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.136322 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.137212 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.138028 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.139110 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.139686 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.140391 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.141671 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.142557 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.143394 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.144895 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.145572 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.146187 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.146808 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.148111 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.149331 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.149940 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.151207 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.151893 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.152457 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.153352 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.154446 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.155807 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.156442 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.157530 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.158834 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.159557 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.160761 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.161444 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.162045 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.162639 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.163488 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.164280 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.165531 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.166196 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.167083 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.168369 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.169048 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.169794 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.171152 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.172067 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.172664 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.173665 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.174615 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.175175 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.176299 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.177292 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.178216 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.178917 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.180267 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.181205 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.182179 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.182769 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.183334 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.184336 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.185622 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.186639 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.187727 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.188765 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.189345 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.190399 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.191723 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.192400 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.193398 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.194166 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.194754 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.195312 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.196159 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.197134 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.198170 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.199348 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.199976 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.200559 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.201994 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.203306 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.203912 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.204475 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.205605 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.206718 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.207284 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.208043 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.209487 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.210740 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.211369 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.211975 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.212667 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.213985 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.215267 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.216030 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.216627 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.217896 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.218672 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.219239 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.220747 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.221776 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.222359 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.223538 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.225188 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.226236 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.226841 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.227429 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.229176 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.231133 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.232768 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.233381 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.235160 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.236455 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.237946 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.238522 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.239118 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.239771 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.241773 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.243435 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.245008 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.245605 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.246191 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.246822 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.248480 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.250258 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.251911 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.252641 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.253207 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.253898 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.254555 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.255209 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.255841 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.256434 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.257101 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.257719 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.258291 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.258879 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.259439 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.260016 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.260697 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.261370 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.262021 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.262631 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.263223 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.263913 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.264483 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.265103 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.265690 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.266257 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.266845 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.267416 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.268024 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.268643 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.269271 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.269893 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.270694 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.271364 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.271958 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.272530 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.273121 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.273702 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.274282 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.274897 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.275506 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.276109 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.276739 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.277350 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.277999 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.278568 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.279176 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.279762 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.280463 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.281142 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.281793 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.282410 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.283044 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.283650 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.284310 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.284915 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.285483 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.286068 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.286666 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.287235 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.287827 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.288419 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.289049 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.289664 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.290243 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.290990 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.291563 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.292380 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.292983 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.293543 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.294141 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.294760 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.295356 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.295985 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.296561 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.297180 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.297835 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.298422 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.299023 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.299610 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.300187 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.301035 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.301661 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.302256 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.302893 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.303469 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.304103 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.304736 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.305305 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.305894 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.306456 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.307069 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.307659 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.308256 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.308868 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.309460 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.310082 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.310941 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.311537 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.312139 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.312739 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.313312 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.313904 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.314488 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.315107 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.315747 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.316333 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.316959 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.317633 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.318225 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.318841 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.319415 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.320012 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.320790 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.321374 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.321984 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.322616 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.323217 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.323834 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.324467 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.325069 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.325655 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.326230 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.326815 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.327378 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.327975 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.328571 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.329195 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.329829 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.330678 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.331332 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.331958 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.332533 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.333127 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.333707 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.334262 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.334862 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.335446 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.336052 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.336656 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.337245 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.337847 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.338470 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.339090 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.339669 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.340375 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.340979 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.341534 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.342159 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.342777 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.343392 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.344018 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.344618 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.345266 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.345875 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.346438 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.347028 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.347606 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.348168 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.348778 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.349375 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.350080 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.350874 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.351464 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.352139 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.352754 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.353325 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.353916 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.354478 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.355051 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.355643 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.356246 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.356725 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.357319 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.357949 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.358533 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.359145 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.359898 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.360467 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.361159 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.361768 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.362340 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.362942 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.363492 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.364076 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.364681 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.365277 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.365912 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.366500 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.367139 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.367780 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.368343 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.368943 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.369609 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.370181 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.370967 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.371553 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.372165 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.372811 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.373397 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.374013 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.374681 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.375255 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.375842 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.376417 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.377015 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.377596 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.378185 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.378784 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.379438 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.380137 0:1127 axfs::fops:297] [AxError::NotADirectory]
[986.381843 0:1127 axfs::root:433] [AxError::IsADirectory]
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_creaMkmNi) failed: remove(/tmp/LTP_creaMkmNi) failed; errno=39: ENOTEMPTY
FAIL LTP CASE creat05 : 0
RUN LTP CASE creat06
tst_test.c:1003: TINFO: Can't mount (null) at mntpoint (tmpfs): ENOSYS (38)
tst_test.c:1303: TINFO: Can't mount tmpfs read-only, falling back to block device...
tst_device.c:293: TWARN: Failed to create test_dev.img: ENOSPC (28)
tst_device.c:354: TBROK: Failed to acquire device

Summary:
passed   0
failed   0
broken   1
skipped  0
warnings 1
[993.364943 0:1132 axfs::root:433] [AxError::IsADirectory]
[993.366001 0:1132 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE creat06 : 6
RUN LTP CASE creat07
tst_test.c:949: TBROK: Failed to copy resource 'creat07_child'

Summary:
passed   0
failed   0
broken   1
skipped  0
warnings 0
[996.725916 0:1134 axfs::fops:297] [AxError::NotADirectory]
[996.727621 0:1134 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE creat07 : 2
RUN LTP CASE creat07_child
tst_test.c:162: TBROK: LTP_IPC_PATH is not defined
FAIL LTP CASE creat07_child : 2
RUN LTP CASE creat08
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
creat08.c:44: TINFO: User nobody: uid = 65534, gid = 65534
creat08.c:46: TBROK: Group ID lookup failed: ENOTSOCK (88)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[1003.546274 0:1138 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE creat08 : 2
RUN LTP CASE creat09
tst_device.c:293: TWARN: Failed to create test_dev.img: ENOSPC (28)
tst_device.c:354: TBROK: Failed to acquire device

HINT: You _MAY_ be missing kernel fixes:

https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=0fa3ecd87848
https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=01ea173e103e
https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=1639a49ccdce
https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=426b4ca2d6a5

HINT: You _MAY_ be vulnerable to CVE(s):

https://cve.mitre.org/cgi-bin/cvename.cgi?name=CVE-2018-13405
https://cve.mitre.org/cgi-bin/cvename.cgi?name=CVE-2021-4037

Summary:
passed   0
failed   0
broken   1
skipped  0
warnings 1
[1007.999035 0:1143 axfs::root:433] [AxError::IsADirectory]
[1008.000171 0:1143 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE creat09 : 6
RUN LTP CASE create_datafile
usage:
	create_file <# of 1048576 buffers to write> <name of file to create>
	 ex. # create_file 10 /tmp/testfile
FAIL LTP CASE create_datafile : 3
RUN LTP CASE create_file
Usage: create_file filename filesize
FAIL LTP CASE create_file : 1
RUN LTP CASE crypto_user01
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
crypto_user01.c:31: TCONF: socket(16, 524290, 21) failed: EAFNOSUPPORT (97)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[1013.528744 0:1149 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE crypto_user01 : 32
RUN LTP CASE crypto_user02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
crypto_user02.c:64: TCONF: socket(16, 524290, 21) failed: EAFNOSUPPORT (97)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[1017.162112 0:1154 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE crypto_user02 : 32
RUN LTP CASE cve-2014-0196
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 01m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
cve-2014-0196.c:52: TBROK: pty creation failed: ENOENT (2)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[1020.724273 0:1159 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE cve-2014-0196 : 2
RUN LTP CASE cve-2015-3290
tst_test.c:1175: TCONF: not (i386 or x86_64)
FAIL LTP CASE cve-2015-3290 : 32
RUN LTP CASE cve-2016-10044
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
cve-2016-10044.c:36: TCONF: syscall(0) __NR_io_setup not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[1027.336296 0:1166 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE cve-2016-10044 : 32
RUN LTP CASE cve-2016-7042
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
cve-2016-7042.c:58: TCONF: /proc/keys does not exist

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[1030.721928 0:1171 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE cve-2016-7042 : 32
RUN LTP CASE cve-2016-7117
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 01m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
cve-2016-7117.c:84: TCONF: syscall(243) __NR_recvmmsg not supported on your arch

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[1034.423554 0:1176 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE cve-2016-7117 : 32
RUN LTP CASE cve-2017-16939
tst_kconfig.c:71: TINFO: Couldn't locate kernel config!
tst_kconfig.c:207: TBROK: Cannot parse kernel .config
FAIL LTP CASE cve-2017-16939 : 2
RUN LTP CASE cve-2017-17052
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
[1086.072472 0:1200 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1088.954579 0:1195 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1091.743113 0:1193 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1094.731937 0:1192 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1097.814838 0:1188 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1100.852681 0:1190 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1104.345175 0:1187 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
cve-2017-17052.c:106: TFAIL: child exited with 2
[1104.549010 0:4777 axmm:27] Mapping error: BadState
cve-2017-17052.c:67: TBROK: fork() failed: EFAULT (14)
[1104.580551 0:1189 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1104.611628 0:4779 axmm:27] Mapping error: BadState
cve-2017-17052.c:67: TBROK: fork() failed: EFAULT (14)
cve-2017-17052.c:106: TFAIL: child exited with 2
[1104.914870 0:4782 axmm:27] Mapping error: BadState
cve-2017-17052.c:66: TBROK: fork() failed: EFAULT (14)
cve-2017-17052.c:106: TFAIL: child exited with 2
[1105.355712 0:4783 axmm:27] Mapping error: BadState
cve-2017-17052.c:66: TBROK: fork() failed: EFAULT (14)
cve-2017-17052.c:106: TFAIL: child exited with 2
cve-2017-17052.c:113: TPASS: kernel survived 4 runs

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[1105.798177 0:1183 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE cve-2017-17052 : 0
RUN LTP CASE cve-2017-17053
tst_test.c:1175: TCONF: no asm/ldt.h header (only for i386 or x86_64)
FAIL LTP CASE cve-2017-17053 : 32
RUN LTP CASE cve-2017-2618
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
cve-2017-2618.c:29: TCONF: /proc/self/attr/fscreate does not exist

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[1112.516457 0:4787 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE cve-2017-2618 : 32
RUN LTP CASE cve-2017-2671
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 01m 10s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
cve-2017-2671.c:57: TCONF: socket() does not support IPPROTO_ICMP

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[1116.253149 0:4792 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE cve-2017-2671 : 32
RUN LTP CASE cve-2022-4378
tst_kconfig.c:71: TINFO: Couldn't locate kernel config!
tst_kconfig.c:207: TBROK: Cannot parse kernel .config
FAIL LTP CASE cve-2022-4378 : 2
RUN LTP CASE daemonlib.sh
SKIP LTP CASE daemonlib.sh : LTP shell helper library is not a standalone test
FAIL LTP CASE daemonlib.sh : 32
RUN LTP CASE data
/tmp/testsuite/musl-ltp-script/ltp_testcode.sh: line 15: ltp/testcases/bin/data: Exec format error
FAIL LTP CASE data : 126
RUN LTP CASE data_space
[1125.755075 0:4802 axmm:27] Mapping error: BadState
data_space    1  TBROK  :  data_space.c:159: fork failed: errno=EFAULT(14): Bad address
data_space    2  TBROK  :  data_space.c:159: Remaining cases broken
[1125.772060 0:4802 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE data_space : 2
RUN LTP CASE dccp01.sh
[1125.896376 0:4806 axmm:27] Mapping error: BadState
FAIL LTP CASE dccp01.sh : 2
RUN LTP CASE dccp_ipsec.sh
[1125.984576 0:4807 axmm:27] Mapping error: BadState
FAIL LTP CASE dccp_ipsec.sh : 2
RUN LTP CASE dccp_ipsec_vti.sh
[1126.085469 0:4808 axmm:27] Mapping error: BadState
FAIL LTP CASE dccp_ipsec_vti.sh : 2
RUN LTP CASE dctcp01.sh
[1126.188727 0:4809 axmm:27] Mapping error: BadState
FAIL LTP CASE dctcp01.sh : 2
RUN LTP CASE delete_module01
[1126.289911 0:4810 axmm::aspace:112] [AxError::BadState] failed to materialize child page
FAIL LTP CASE delete_module01 : 2
RUN LTP CASE delete_module02
[1126.382988 0:4811 page_table_multiarch::bits64:490] failed to map page: 0x1400000(Size4K) -> PA:0x0, NoMemory
[1126.384464 0:4811 axmm:27] Mapping error: BadState
FAIL LTP CASE delete_module02 : 2
RUN LTP CASE delete_module03
[1126.487251 0:4812 axmm:27] Mapping error: BadState
FAIL LTP CASE delete_module03 : 2
RUN LTP CASE df01.sh
[1126.589256 0:4813 axmm:27] Mapping error: BadState
FAIL LTP CASE df01.sh : 2
RUN LTP CASE dhcp_lib.sh
SKIP LTP CASE dhcp_lib.sh : LTP shell helper library is not a standalone test
FAIL LTP CASE dhcp_lib.sh : 32
RUN LTP CASE dhcpd_tests.sh
[1126.795422 0:4815 axmm:27] Mapping error: BadState
FAIL LTP CASE dhcpd_tests.sh : 2
RUN LTP CASE dio_append
[1126.902815 0:4816 axmm:27] Mapping error: BadState
FAIL LTP CASE dio_append : 2
RUN LTP CASE dio_read
[1126.997372 0:4817 axmm:27] Mapping error: BadState
FAIL LTP CASE dio_read : 2
RUN LTP CASE dio_sparse
[1127.098252 0:4818 axmm:27] Mapping error: BadState
FAIL LTP CASE dio_sparse : 2
RUN LTP CASE dio_truncate
[1127.194089 0:4819 axmm:27] Mapping error: BadState
FAIL LTP CASE dio_truncate : 2
RUN LTP CASE diotest1
[1127.279225 0:280 axmm:27] Mapping error: BadState
/tmp/testsuite/musl-ltp-script/ltp_testcode.sh: line 15: can't fork: Bad address
#### OS COMP TEST GROUP START lua-musl ####
[1128.786051 0:4820 axmm:27] Mapping error: BadState
./lua_testcode.sh: line 3: can't fork: Bad address
[1129.121264 0:4823 axmm:27] Mapping error: BadState
sh: can't fork: Bad address
#### OS COMP TEST GROUP START unixbench-musl ####
SKIP: unixbench currently blocks on unresolved executable/runtime compatibility
#### OS COMP TEST GROUP END unixbench-musl ####
[1129.732684 0:4824 axmm:27] Mapping error: BadState
sh: can't fork: Bad address
#### OS COMP TEST GROUP START busybox-glibc ####
#### independent command test
testcase busybox echo "#### independent command test" success
testcase busybox ash -c exit success
testcase busybox sh -c exit success
bbb
testcase busybox basename /aaa/bbb success
    January 1970
Su Mo Tu We Th Fr Sa
             1  2  3
 4  5  6  7  8  9 10
11 12 13 14 15 16 17
18 19 20 21 22 23 24
25 26 27 28 29 30 31
                     
testcase busybox cal success
testcase busybox clear success
Thu Jan  1 00:18:54 UTC 1970
testcase busybox date success
Filesystem           1K-blocks      Used Available Use% Mounted on
devfs                  1045228   1037624      7604  99% /dev
tmpfs                  1045228   1037624      7604  99% /tmp
tmpfs                  1045228   1037624      7604  99% /var
proc                   1045228   1037624      7604  99% /proc
sysfs                  1045228   1037624      7604  99% /sys
testcase busybox df success
/aaa
testcase busybox dirname /aaa/bbb success
testcase busybox dmesg success
0	.
testcase busybox du success
2
testcase busybox expr 1 + 1 success
testcase busybox false success
testcase busybox true success
testcase busybox which ls fail
return: 1, cmd: which ls
Linux
testcase busybox uname success
 00:19:01 up 0 min,  0 users,  load average: 0.00, 0.00, 0.00
testcase busybox uptime success
abc
testcase busybox printf "abc\n" success
PID   USER     TIME  COMMAND
testcase busybox ps success
/tmp/testsuite/glibc/busybox
testcase busybox pwd success
              total        used        free      shared  buff/cache   available
Mem:              0           0           0           0           0     1039813
-/+ buffers/cache:            0           0
Swap:             0           0           0
testcase busybox free success
Thu Jan  1 00:19:05 1970  0.000000 seconds
testcase busybox hwclock success
[1145.682893 0:4847 axmm:27] Mapping error: BadState
sh: can't fork: Bad address
testcase busybox sh -c 'sleep 5' & /glibc/busybox kill $! fail
return: 2, cmd: sh -c 'sleep 5' & /glibc/busybox kill $!
busybox_cmd.txt
busybox_testcode.sh
line
testcase busybox ls success
testcase busybox sleep 1 success
#### file opration test
testcase busybox echo "#### file opration test" success
testcase busybox touch test.txt success
testcase busybox echo "hello world" > test.txt success
hello world
testcase busybox cat test.txt success
l
testcase busybox cut -c 3 test.txt success
0000000 062550 066154 020157 067567 066162 005144
0000014
testcase busybox od test.txt success
hello world
testcase busybox head test.txt success
hello world
testcase busybox tail test.txt success
00000000  68 65 6c 6c 6f 20 77 6f  72 6c 64 0a              |hello world.|
0000000c
testcase busybox hexdump -C test.txt success
6f5902ac237024bdd0c176cb93063dc4  test.txt
testcase busybox md5sum test.txt success
testcase busybox echo "ccccccc" >> test.txt success
testcase busybox echo "bbbbbbb" >> test.txt success
testcase busybox echo "aaaaaaa" >> test.txt success
testcase busybox echo "2222222" >> test.txt success
testcase busybox echo "1111111" >> test.txt success
testcase busybox echo "bbbbbbb" >> test.txt success
[1159.672067 0:4866 axmm:27] Mapping error: BadState
sh: can't fork: Bad address
testcase busybox sort test.txt | /glibc/busybox uniq fail
return: 2, cmd: sort test.txt | /glibc/busybox uniq
  File: test.txt
  Size: 60        	Blocks: 0          IO Block: 512    regular file
Device: 1h/1d	Inode: 4368043057645409086  Links: 1
Access: (0666/-rw-rw-rw-)  Uid: (    0/    root)   Gid: (    0/    root)
Access: 1970-01-01 00:00:00.000000000 +0000
Modify: 1970-01-01 00:00:00.000000000 +0000
Change: 1970-01-01 00:00:00.000000000 +0000
testcase busybox stat test.txt success
hello world
ccccccc
bbbbbbb
aaaaaaa
2222222
1111111
bbbbbbb
testcase busybox strings test.txt success
        7         8        60 test.txt
testcase busybox wc test.txt success
testcase busybox [ -f test.txt ] success
hello world
ccccccc
bbbbbbb
aaaaaaa
2222222
1111111
bbbbbbb
testcase busybox more test.txt success
testcase busybox rm test.txt -f success
testcase busybox mkdir test_dir success
testcase busybox mv test_dir test success
testcase busybox rmdir test success
echo "hello world" > test.txt
grep hello busybox_cmd.txt
testcase busybox grep hello busybox_cmd.txt success
testcase busybox cp busybox_cmd.txt busybox_cmd.bak success
testcase busybox rm busybox_cmd.bak -f success
./busybox_cmd.txt
testcase busybox find -name "busybox_cmd.txt" success
#### OS COMP TEST GROUP END busybox-glibc ####
[1169.517493 0:2 axmm:27] Mapping error: BadState
autorun: /tmp/testsuite/glibc/cyclictest/cyclictest_testcode.sh failed: failed to map user stack: Bad internal state
[1169.817269 0:2 axmm:27] Mapping error: BadState
autorun: /tmp/testsuite/glibc/iozone/iozone_testcode.sh failed: failed to map user stack: Bad internal state
[1170.270717 0:2 axmm:27] Mapping error: BadState
autorun: /tmp/testsuite/glibc/iperf/iperf_testcode.sh failed: failed to map user stack: Bad internal state
#### OS COMP TEST GROUP START libcbench-glibc ####
SKIP: libcbench currently triggers an unrecovered allocator exhaustion path
#### OS COMP TEST GROUP END libcbench-glibc ####
#### OS COMP TEST GROUP START libctest-glibc ####
SKIP: libctest still trips unresolved pthread cancellation paths
#### OS COMP TEST GROUP END libctest-glibc ####
#### OS COMP TEST GROUP START lmbench-glibc ####
SKIP: lmbench still triggers an unresolved user-space page-fault path
#### OS COMP TEST GROUP END lmbench-glibc ####
[1170.605723 0:2 axmm:27] Mapping error: BadState
autorun: /glibc/ltp_testcode.sh failed: failed to map user stack: Bad internal state
[1171.043578 0:2 axmm:27] Mapping error: BadState
autorun: /tmp/testsuite/glibc/lua/lua_testcode.sh failed: failed to map user stack: Bad internal state
[1171.371606 0:2 axmm:27] Mapping error: BadState
autorun: /tmp/testsuite/glibc/netperf/netperf_testcode.sh failed: failed to map user stack: Bad internal state
#### OS COMP TEST GROUP START unixbench-glibc ####
SKIP: unixbench currently blocks on unresolved executable/runtime compatibility
#### OS COMP TEST GROUP END unixbench-glibc ####
[1171.385464 0:2 axplat_riscv64_qemu_virt::power:28] Shutting down...
```
