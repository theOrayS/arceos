# RV evaluation output

Source: full RV evaluation log for the iperf TCP getsockopt compatibility cleanup candidate.
Recorded metrics: top-level 116 pass-like / 4 fail-like / 55 skip.
Comparison with previous tracked output: top-level delta +0 pass-like / +0 fail-like / +0 skip; iperf warning delta `getsockopt - Invalid argument` -7 and `Ignoring nonsense TCP MSS 0` -6.
Target note: iperf-musl still reports 6 successful subtests, and the target TCP getsockopt/MSS warning markers are absent (`getsockopt - Invalid argument` markers 0, `Ignoring nonsense TCP MSS 0` markers 0).
Limitation: this is an iperf output-cleanliness improvement, not a pass-count improvement; existing glibc BadState/fork issues and LTP diagnostic failures remain.
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
    --> examples/shell/src/uspace.rs:7310:8
     |
6741 | impl FdTable {
     | ------------ methods in this implementation
...
7310 |     fn insert(&mut self, entry: FdEntry) -> Result<i32, LinuxError> {
     |        ^^^^^^
...
7318 |     fn insert_min(&mut self, entry: FdEntry, min_fd: usize) -> Result<i32, LinuxError> {
     |        ^^^^^^^^^^
     |
     = note: `#[warn(dead_code)]` on by default

warning: `arceos-shell` (bin "arceos-shell") generated 1 warning
    Finished `release` profile [optimized] target(s) in 0.64s
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

[  0.175152 0 axruntime:135] Logging is enabled.
[  0.181401 0 axruntime:136] Primary CPU 0 started, arg = 0xbfe00000.
[  0.185739 0 axruntime:139] Found physcial memory regions:
[  0.186572 0 axruntime:141]   [PA:0x101000, PA:0x102000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.187576 0 axruntime:141]   [PA:0xc000000, PA:0xc210000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.190645 0 axruntime:141]   [PA:0x10000000, PA:0x10001000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.191309 0 axruntime:141]   [PA:0x10001000, PA:0x10009000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.192089 0 axruntime:141]   [PA:0x30000000, PA:0x40000000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.192821 0 axruntime:141]   [PA:0x40000000, PA:0x80000000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.195554 0 axruntime:141]   [PA:0x80200000, PA:0x802b3000) .text (READ | EXECUTE | RESERVED)
[  0.196159 0 axruntime:141]   [PA:0x802b3000, PA:0x802d8000) .rodata (READ | RESERVED)
[  0.196731 0 axruntime:141]   [PA:0x802d8000, PA:0x802dc000) .data .tdata .tbss .percpu (READ | WRITE | RESERVED)
[  0.199701 0 axruntime:141]   [PA:0x802dc000, PA:0x8031c000) boot stack (READ | WRITE | RESERVED)
[  0.202587 0 axruntime:141]   [PA:0x8031c000, PA:0x80345000) .bss (READ | WRITE | RESERVED)
[  0.203332 0 axruntime:141]   [PA:0x80345000, PA:0xc0000000) free memory (READ | WRITE | FREE)
[  0.204249 0 axruntime:216] Initialize global memory allocator...
[  0.204971 0 axruntime:217]   use TLSF allocator.
[  0.215643 0 axmm:103] Initialize virtual memory management...
[  0.432543 0 axruntime:156] Initialize platform devices...
smp = 1
[  0.433615 0 axtask::api:73] Initialize scheduling...
[  0.436897 0 axtask::api:83]   use FIFO scheduler.
[  0.437646 0 axdriver:152] Initialize device drivers...
[  0.438397 0 axdriver:153]   device model: static
[  0.440747 0 virtio_drivers::device::blk:63] found a block device of size 4194304KB
[  0.444052 0 axdriver::bus::mmio:11] registered a new Block device at [PA:0x10001000, PA:0x10002000): "virtio-blk"
[  0.446136 0 virtio_drivers::device::net::dev_raw:33] negotiated_features Features(MAC | STATUS | RING_INDIRECT_DESC | RING_EVENT_IDX)
[  0.463904 0 axdriver::bus::mmio:11] registered a new Net device at [PA:0x10008000, PA:0x10009000): "virtio-net"
[  0.473869 0 axfs:44] Initialize filesystems...
[  0.475937 0 axfs:47]   use block device 0: "virtio-blk"
[  0.491439 0 axfs::root:336]   detected root filesystem: Ext4
[  0.613300 0 axnet:42] Initialize network subsystem...
[  0.614025 0 axnet:45]   use NIC 0: "virtio-net"
[  0.626067 0 axnet::smoltcp_impl:335] created net interface "eth0":
[  0.628794 0 axnet::smoltcp_impl:336]   ether:    52-54-00-12-34-56
[  0.631913 0 axnet::smoltcp_impl:337]   ip:       10.0.2.15/24
[  0.634602 0 axnet::smoltcp_impl:338]   gateway:  10.0.2.2
[  0.636683 0 axruntime:182] Initialize interrupt handlers...
[  0.639979 0 axruntime:194] Primary CPU 0 init OK.
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
start:7408, end:7494
interval: 86
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
Thu Jan  1 00:00:19 UTC 1970
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
 00:00:27 up 0 min,  0 users,  load average: 0.00, 0.00, 0.00
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
Thu Jan  1 00:00:32 1970  0.000000 seconds
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
T: 0 (  113) P:99 I:1000 C:    999 Min:      5 Act:    9 Avg:   17 Max:    1053
====== cyclictest NO_STRESS_P1 end: success ======
====== cyclictest NO_STRESS_P8 begin ======
WARN: stat /dev/cpu_dma_latency failed: No such file or directory
T: 0 (  115) P:99 I:1000 C:    999 Min:      4 Act:   15 Avg:   22 Max:    1698
T: 1 (  116) P:99 I:1500 C:    666 Min:      4 Act:  135 Avg:   21 Max:    1518
T: 2 (  117) P:99 I:2000 C:    500 Min:      4 Act:  645 Avg:   20 Max:     645
T: 3 (  118) P:99 I:2500 C:    400 Min:      5 Act:  630 Avg:   24 Max:    1067
T: 4 (  119) P:99 I:3000 C:    334 Min:      4 Act:   22 Avg:   20 Max:    1155
T: 5 (  120) P:99 I:3500 C:    286 Min:      5 Act:   20 Avg:   26 Max:     628
T: 6 (  121) P:99 I:4000 C:    250 Min:      4 Act:  628 Avg:   28 Max:     628
T: 7 (  122) P:99 I:4500 C:    223 Min:      4 Act:   17 Avg:   22 Max:     972
====== cyclictest NO_STRESS_P8 end: success ======
====== start hackbench ======
Running in process mode with 10 groups using 40 file descriptors each (== 400 tasks)
Each sender will pass 100000000 messages of 100 bytes
Creating fdpair (error: Function not implemented)
====== cyclictest STRESS_P1 begin ======
WARN: stat /dev/cpu_dma_latency failed: No such file or directory
T: 0 (  126) P:99 I:1000 C:    989 Min:      5 Act:    9 Avg:   33 Max:    3509
====== cyclictest STRESS_P1 end: success ======
====== cyclictest STRESS_P8 begin ======
WARN: stat /dev/cpu_dma_latency failed: No such file or directory
T: 0 (  128) P:99 I:1000 C:    964 Min:      4 Act:   26 Avg:   71 Max:    4421
T: 1 (  129) P:99 I:1500 C:    649 Min:      5 Act:  226 Avg:   86 Max:    4520
T: 2 (  130) P:99 I:2000 C:    488 Min:      4 Act:  343 Avg:  118 Max:    4115
T: 3 (  131) P:99 I:2500 C:    394 Min:      4 Act:  357 Avg:  127 Max:    4746
T: 4 (  132) P:99 I:3000 C:    329 Min:      5 Act:   35 Avg:  135 Max:    4015
T: 5 (  133) P:99 I:3500 C:    285 Min:      4 Act:   27 Avg:  133 Max:    3677
T: 6 (  134) P:99 I:4000 C:    250 Min:      5 Act:  243 Avg:  151 Max:    3942
T: 7 (  135) P:99 I:4500 C:    223 Min:      4 Act:   25 Avg:  100 Max:    3892
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

	Run began: Thu Jan  1 00:01:18 1970

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
            4096       1     45168     57257     57339     43970     46974     59554[ 79.822186 0:143 axfs::fops:269] [AxError::InvalidInput]
[ 79.895145 0:143 axfs::fops:269] [AxError::InvalidInput]
     56362      65217      58114     54092     59775     31748     49079

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

	Run began: Thu Jan  1 00:01:21 1970

	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 0 -i 1 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000006 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes
	Each process writes a 1024 kByte file in 1 kByte records

	Children see throughput for  4 initial writers 	=  128578.60 kB/sec
	Parent sees throughput for  4 initial writers 	=    2421.89 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=  128578.60 kB/sec
	Avg throughput per process 			=   32144.65 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for  4 rewriters 	=   96640.24 kB/sec
	Parent sees throughput for  4 rewriters 	=    2954.30 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   96640.24 kB/sec
	Avg throughput per process 			=   24160.06 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for  4 readers 		=   78551.70 kB/sec
	Parent sees throughput for  4 readers 		=    2561.84 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   78551.70 kB/sec
	Avg throughput per process 			=   19637.93 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for 4 re-readers 	=   82295.27 kB/sec
	Parent sees throughput for 4 re-readers 	=    2645.16 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   82295.27 kB/sec
	Avg throughput per process 			=   20573.82 kB/sec
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

	Run began: Thu Jan  1 00:01:40 1970

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

	Children see throughput for  4 initial writers 	=   77475.98 kB/sec
	Parent sees throughput for  4 initial writers 	=    2379.94 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   77475.98 kB/sec
	Avg throughput per process 			=   19368.99 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for  4 rewriters 	=   92235.63 kB/sec
	Parent sees throughput for  4 rewriters 	=    2603.80 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   92235.63 kB/sec
	Avg throughput per process 			=   23058.91 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for 4 random readers 	=   54211.45 kB/sec
	Parent sees throughput for 4 random readers 	=    2591.30 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   54211.45 kB/sec
	Avg throughput per process 			=   13552.86 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for 4 random writers 	=   60030.48 kB/sec
	Parent sees throughput for 4 random writers 	=    2631.59 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   60030.48 kB/sec
	Avg throughput per process 			=   15007.62 kB/sec
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

	Run began: Thu Jan  1 00:02:04 1970

	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 0 -i 3 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000006 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes
	Each process writes a 1024 kByte file in 1 kByte records

	Children see throughput for  4 initial writers 	=   87566.27 kB/sec
	Parent sees throughput for  4 initial writers 	=    2465.03 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   87566.27 kB/sec
	Avg throughput per process 			=   21891.57 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for  4 rewriters 	=   99853.73 kB/sec
	Parent sees throughput for  4 rewriters 	=    2815.12 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   99853.73 kB/sec
	Avg throughput per process 			=   24963.43 kB/sec
	Min xfer 					=       0.00 kB
[133.453363 0:198 axfs::fops:269] [AxError::InvalidInput]

	Children see throughput for 4 reverse readers 	=   51044.32 kB/sec
	Parent sees throughput for 4 reverse readers 	=    2670.42 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   51044.32 kB/sec
	Avg throughput per process 			=   12761.08 kB/sec
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

	Run began: Thu Jan  1 00:02:21 1970

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

	Children see throughput for  4 initial writers 	=   79682.52 kB/sec
	Parent sees throughput for  4 initial writers 	=    2484.99 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   79682.52 kB/sec
	Avg throughput per process 			=   19920.63 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for  4 rewriters 	=   88237.83 kB/sec
	Parent sees throughput for  4 rewriters 	=    2659.23 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   88237.83 kB/sec
	Avg throughput per process 			=   22059.46 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for 4 stride readers 	=   60072.74 kB/sec
	Parent sees throughput for 4 stride readers 	=    2145.60 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   60072.74 kB/sec
	Avg throughput per process 			=   15018.19 kB/sec
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

	Run began: Thu Jan  1 00:02:39 1970

	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 6 -i 7 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000006 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes
	Each process writes a 1024 kByte file in 1 kByte records

	Children see throughput for  4 fwriters 	=  251272.12 kB/sec
	Parent sees throughput for  4 fwriters 		=    9476.90 kB/sec
	Min throughput per process 			=   62370.57 kB/sec
	Max throughput per process 			=   63248.92 kB/sec
	Avg throughput per process 			=   62818.03 kB/sec
	Min xfer 					=    1024.00 kB

	Children see throughput for  4 freaders 	=  189804.02 kB/sec
	Parent sees throughput for  4 freaders 		=    8828.12 kB/sec
	Min throughput per process 			=   47278.27 kB/sec
	Max throughput per process 			=   47517.40 kB/sec
	Avg throughput per process 			=   47451.01 kB/sec
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

	Run began: Thu Jan  1 00:02:53 1970

	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 9 -i 10 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000007 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes
	Each process writes a 1024 kByte file in 1 kByte records

	Children see throughput for 4 pwrite writers 	=   85042.77 kB/sec
	Parent sees throughput for 4 pwrite writers 	=    2697.36 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   85042.77 kB/sec
	Avg throughput per process 			=   21260.69 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for 4 pread readers 	=   82767.54 kB/sec
	Parent sees throughput for 4 pread readers 	=    2489.51 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   82767.54 kB/sec
	Avg throughput per process 			=   20691.88 kB/sec
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

	Run began: Thu Jan  1 00:03:10 1970

	Selected test not available on the version.
	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 11 -i 12 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000007 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes
	Each process writes a 1024 kByte file in 1 kByte records

	Children see throughput for  4 initial writers 	=   80112.66 kB/sec
	Parent sees throughput for  4 initial writers 	=    2494.46 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   80112.66 kB/sec
	Avg throughput per process 			=   20028.16 kB/sec
	Min xfer 					=       0.00 kB

	Children see throughput for  4 rewriters 	=   86676.82 kB/sec
	Parent sees throughput for  4 rewriters 	=    2486.43 kB/sec
	Min throughput per process 			=       0.00 kB/sec
	Max throughput per process 			=   86676.82 kB/sec
	Avg throughput per process 			=   21669.21 kB/sec
	Min xfer 					=       0.00 kB



iozone test complete.
#### OS COMP TEST GROUP END iozone-musl ####
#### OS COMP TEST GROUP START iperf-musl ####
====== iperf BASIC_UDP begin ======
Connecting to host 127.0.0.1, port 5001
[  5] local 0.0.0.0 port 49152 connected to 127.0.0.1 port 5001
[ ID] Interval           Transfer     Bitrate         Total Datagrams
[  5]   0.00-2.00   sec  6.43 MBytes  26.9 Mbits/sec  4616
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-2.00   sec  6.43 MBytes  26.9 Mbits/sec  0.000 ms  0/4616 (0%)  sender
[  5]   0.00-2.01   sec  6.43 MBytes  26.9 Mbits/sec  0.235 ms  0/4616 (0%)  receiver

iperf Done.
====== iperf BASIC_UDP end: success ======

====== iperf BASIC_TCP begin ======
Connecting to host 127.0.0.1, port 5001
[  5] local 127.0.0.1 port 49154 connected to 127.0.0.1 port 5001
[ ID] Interval           Transfer     Bitrate         Retr  Cwnd
[  5]   0.00-2.01   sec  56.2 MBytes   235 Mbits/sec    0   0.00 Bytes
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Retr
[  5]   0.00-2.01   sec  56.2 MBytes   235 Mbits/sec    0             sender
[  5]   0.00-2.02   sec  55.4 MBytes   231 Mbits/sec                  receiver

iperf Done.
====== iperf BASIC_TCP end: success ======

====== iperf PARALLEL_UDP begin ======
Connecting to host 127.0.0.1, port 5001
[  5] local 0.0.0.0 port 49153 connected to 127.0.0.1 port 5001
[  7] local 0.0.0.0 port 49154 connected to 127.0.0.1 port 5001
[  9] local 0.0.0.0 port 49155 connected to 127.0.0.1 port 5001
[ 11] local 0.0.0.0 port 49156 connected to 127.0.0.1 port 5001
[ 13] local 0.0.0.0 port 49157 connected to 127.0.0.1 port 5001
[ ID] Interval           Transfer     Bitrate         Total Datagrams
[  5]   0.00-2.00   sec  1.10 MBytes  4.61 Mbits/sec  791
[  7]   0.00-2.00   sec  1.10 MBytes  4.61 Mbits/sec  791
[  9]   0.00-2.00   sec  1.10 MBytes  4.61 Mbits/sec  791
[ 11]   0.00-2.00   sec  1.10 MBytes  4.61 Mbits/sec  791
[ 13]   0.00-2.00   sec  1.10 MBytes  4.61 Mbits/sec  791
[SUM]   0.00-2.00   sec  5.51 MBytes  23.1 Mbits/sec  3955
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-2.00   sec  1.10 MBytes  4.61 Mbits/sec  0.000 ms  0/791 (0%)  sender
[  5]   0.00-2.01   sec  5.51 MBytes  23.0 Mbits/sec  0.274 ms  959981506/959985462 (1e+02%)  receiver
[  7]   0.00-2.00   sec  1.10 MBytes  4.61 Mbits/sec  0.000 ms  0/791 (0%)  sender
[  7]   0.00-2.01   sec  5.51 MBytes  23.0 Mbits/sec  0.271 ms  959981506/959985462 (1e+02%)  receiver
[  9]   0.00-2.00   sec  1.10 MBytes  4.61 Mbits/sec  0.000 ms  0/791 (0%)  sender
[  9]   0.00-2.01   sec  5.51 MBytes  23.0 Mbits/sec  0.277 ms  1551438472/1551442428 (1e+02%)  receiver
[ 11]   0.00-2.00   sec  1.10 MBytes  4.61 Mbits/sec  0.000 ms  0/791 (0%)  sender
[ 11]   0.00-2.01   sec  5.51 MBytes  23.0 Mbits/sec  0.276 ms  1323731667/1323735623 (1e+02%)  receiver
[ 13]   0.00-2.00   sec  1.10 MBytes  4.61 Mbits/sec  0.000 ms  0/791 (0%)  sender
[ 13]   0.00-2.01   sec  5.51 MBytes  23.0 Mbits/sec  0.276 ms  0/791 (0%)  receiver
[SUM]   0.00-2.00   sec  5.51 MBytes  23.1 Mbits/sec  0.000 ms  0/3955 (0%)  sender
[SUM]   0.00-2.01   sec  27.5 MBytes   115 Mbits/sec  0.275 ms  500165855/500182470 (1.3e+07%)  receiver

iperf Done.
====== iperf PARALLEL_UDP end: success ======

====== iperf PARALLEL_TCP begin ======
Connecting to host 127.0.0.1, port 5001
[  5] local 127.0.0.1 port 49157 connected to 127.0.0.1 port 5001
[  7] local 127.0.0.1 port 49158 connected to 127.0.0.1 port 5001
[  9] local 127.0.0.1 port 49159 connected to 127.0.0.1 port 5001
[ 11] local 127.0.0.1 port 49160 connected to 127.0.0.1 port 5001
[ 13] local 127.0.0.1 port 49161 connected to 127.0.0.1 port 5001
[ ID] Interval           Transfer     Bitrate         Retr  Cwnd
[  5]   0.00-2.04   sec  12.0 MBytes  49.4 Mbits/sec    0   0.00 Bytes
[  7]   0.00-2.04   sec  12.0 MBytes  49.4 Mbits/sec    0   0.00 Bytes
[  9]   0.00-2.04   sec  12.0 MBytes  49.4 Mbits/sec    0   0.00 Bytes
[ 11]   0.00-2.04   sec  12.0 MBytes  49.4 Mbits/sec    0   0.00 Bytes
[ 13]   0.00-2.04   sec  12.0 MBytes  49.4 Mbits/sec    0   0.00 Bytes
[SUM]   0.00-2.04   sec  60.0 MBytes   247 Mbits/sec    0
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Retr
[  5]   0.00-2.04   sec  12.0 MBytes  49.4 Mbits/sec    0             sender
[  5]   0.00-2.06   sec  11.1 MBytes  45.4 Mbits/sec                  receiver
[  7]   0.00-2.04   sec  12.0 MBytes  49.4 Mbits/sec    0             sender
[  7]   0.00-2.06   sec  11.1 MBytes  45.4 Mbits/sec                  receiver
[  9]   0.00-2.04   sec  12.0 MBytes  49.4 Mbits/sec    0             sender
[  9]   0.00-2.06   sec  11.1 MBytes  45.4 Mbits/sec                  receiver
[ 11]   0.00-2.04   sec  12.0 MBytes  49.4 Mbits/sec    0             sender
[ 11]   0.00-2.06   sec  11.1 MBytes  45.4 Mbits/sec                  receiver
[ 13]   0.00-2.04   sec  12.0 MBytes  49.4 Mbits/sec    0             sender
[ 13]   0.00-2.06   sec  11.1 MBytes  45.4 Mbits/sec                  receiver
[SUM]   0.00-2.04   sec  60.0 MBytes   247 Mbits/sec    0             sender
[SUM]   0.00-2.06   sec  55.6 MBytes   227 Mbits/sec                  receiver

iperf Done.
====== iperf PARALLEL_TCP end: success ======

====== iperf REVERSE_UDP begin ======
Connecting to host 127.0.0.1, port 5001
Reverse mode, remote host 127.0.0.1 is sending
[  5] local 0.0.0.0 port 49158 connected to 127.0.0.1 port 5001
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-2.00   sec  5.93 MBytes  24.8 Mbits/sec  0.049 ms  0/4256 (0%)
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-2.01   sec  5.93 MBytes  24.8 Mbits/sec  0.000 ms  0/4257 (0%)  sender
[  5]   0.00-2.00   sec  5.93 MBytes  24.8 Mbits/sec  0.049 ms  0/4256 (0%)  receiver

iperf Done.
====== iperf REVERSE_UDP end: success ======

====== iperf REVERSE_TCP begin ======
Connecting to host 127.0.0.1, port 5001
Reverse mode, remote host 127.0.0.1 is sending
[  5] local 127.0.0.1 port 49164 connected to 127.0.0.1 port 5001
[ ID] Interval           Transfer     Bitrate         Retr  Cwnd
[  5]   0.00-2.00   sec  55.6 MBytes   233 Mbits/sec
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Retr
[  5]   0.00-2.02   sec  56.5 MBytes   235 Mbits/sec    0             sender
[  5]   0.00-2.00   sec  55.6 MBytes   233 Mbits/sec                  receiver

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
[227.780803 0:282 axfs::root:433] [AxError::IsADirectory]
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
[238.615193 0:293 axnet::smoltcp_impl::tcp:284] [AxError::InvalidInput] socket accept() failed: not listen
[238.618735 0:293 arceos_posix_api::imp::net:486] sys_accept => Err(EINVAL)
accept01.c:92: TPASS: invalid socket buffer : EINVAL (22)
[238.621688 0:293 axnet::smoltcp_impl::tcp:284] [AxError::InvalidInput] socket accept() failed: not listen
[238.622379 0:293 arceos_posix_api::imp::net:486] sys_accept => Err(EINVAL)
accept01.c:92: TPASS: invalid salen : EINVAL (22)
[238.623132 0:293 axnet::smoltcp_impl::tcp:284] [AxError::InvalidInput] socket accept() failed: not listen
[238.623676 0:293 arceos_posix_api::imp::net:486] sys_accept => Err(EINVAL)
accept01.c:92: TPASS: no queued connections : EINVAL (22)
[238.628656 0:293 arceos_posix_api::imp::net:486] sys_accept => Err(EOPNOTSUPP)
accept01.c:92: TPASS: UDP accept : EOPNOTSUPP (95)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[238.729964 0:290 axfs::root:433] [AxError::IsADirectory]
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
[244.545455 0:295 axfs::fops:297] [AxError::NotADirectory]
[244.548992 0:295 axfs::root:433] [AxError::IsADirectory]
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
[250.422093 0:302 axfs::root:433] [AxError::IsADirectory]
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
[256.467432 0:307 axfs::root:433] [AxError::IsADirectory]
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
[263.876793 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.879935 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.880633 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.885390 0:318 axfs::root:433] [AxError::IsADirectory]
[263.886536 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.887267 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.890066 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.890867 0:318 axfs::root:433] [AxError::IsADirectory]
[263.891877 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.895260 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.895949 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.896746 0:318 axfs::root:433] [AxError::IsADirectory]
[263.900849 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.901583 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.904232 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.904983 0:318 axfs::root:433] [AxError::IsADirectory]
[263.905877 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.910453 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.911131 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.911896 0:318 axfs::root:433] [AxError::IsADirectory]
[263.915145 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.915863 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.918046 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.918813 0:318 axfs::root:433] [AxError::IsADirectory]
[263.920468 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.921161 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.921863 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.924526 0:318 axfs::fops:297] [AxError::NotADirectory]
[263.925488 0:318 axfs::root:433] [AxError::IsADirectory]
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
[269.398513 0:336 axfs::fops:297] [AxError::NotADirectory]
[269.401912 0:336 axfs::fops:297] [AxError::NotADirectory]
[269.402525 0:336 axfs::fops:297] [AxError::NotADirectory]
[269.403077 0:336 axfs::fops:297] [AxError::NotADirectory]
[269.409192 0:336 axfs::root:433] [AxError::IsADirectory]
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
[275.374648 0:341 axfs::root:433] [AxError::IsADirectory]
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
[283.301779 0:350 axfs::root:433] [AxError::IsADirectory]
[283.304590 0:350 axfs::root:433] [AxError::IsADirectory]
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
[290.779556 0:352 axfs::root:433] [AxError::IsADirectory]
[290.785828 0:352 axfs::root:433] [AxError::IsADirectory]
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
[307.037195 0:362 axfs::root:433] [AxError::IsADirectory]
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
[313.045929 0:367 axfs::root:433] [AxError::IsADirectory]
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
[319.547812 0:372 axfs::root:433] [AxError::IsADirectory]
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
[326.046449 0:377 axfs::root:433] [AxError::IsADirectory]
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
[339.331759 0:384 axfs::root:433] [AxError::IsADirectory]
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
[347.270535 0:389 axfs::root:433] [AxError::IsADirectory]
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
[355.270518 0:394 axfs::root:433] [AxError::IsADirectory]
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
[398.042585 0:418 axfs::root:433] [AxError::IsADirectory]
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
[404.226267 0:426 axfs::root:433] [AxError::IsADirectory]
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
[412.842896 0:433 axfs::root:433] [AxError::IsADirectory]
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
[421.462822 0:440 axfs::root:433] [AxError::IsADirectory]
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
[430.184288 0:446 axfs::root:433] [AxError::IsADirectory]
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
[443.830607 0:459 arceos_posix_api::imp::net:333] sys_bind => Err(EINVAL)
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
[449.719012 0:461 axfs::root:433] [AxError::IsADirectory]
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
[474.519834 0:476 axfs::fops:297] [AxError::NotADirectory]
[474.522422 0:476 axfs::root:433] [AxError::IsADirectory]
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
[482.434788 0:488 arceos_posix_api::imp::net:333] sys_bind => Err(EINVAL)
bind01.c:60: TPASS: invalid salen : EINVAL (22)
bind01.c:60: TPASS: invalid socket : ENOTSOCK (88)
bind01.c:63: TPASS: INADDR_ANYPORT passed
[482.439025 0:488 arceos_posix_api::imp::net:333] sys_bind => Err(EINVAL)
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
[482.544362 0:485 axfs::fops:297] [AxError::NotADirectory]
[482.548481 0:485 axfs::root:433] [AxError::IsADirectory]
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
[488.388554 0:490 axfs::root:433] [AxError::IsADirectory]
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
[494.346082 0:495 axfs::root:433] [AxError::IsADirectory]
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
[500.235559 0:500 axfs::root:433] [AxError::IsADirectory]
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
[506.160613 0:505 axfs::root:433] [AxError::IsADirectory]
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
[521.424940 0:523 axfs::root:433] [AxError::IsADirectory]
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
[525.120345 0:528 axfs::root:433] [AxError::IsADirectory]
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
[528.766083 0:533 axfs::root:433] [AxError::IsADirectory]
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
[532.394840 0:538 axfs::root:433] [AxError::IsADirectory]
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
[535.956806 0:543 axfs::root:433] [AxError::IsADirectory]
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
[539.494427 0:548 axfs::root:433] [AxError::IsADirectory]
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
[543.155776 0:553 axfs::root:433] [AxError::IsADirectory]
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
[546.722048 0:558 axfs::root:433] [AxError::IsADirectory]
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
[550.239755 0:563 axfs::root:433] [AxError::IsADirectory]
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
[553.736748 0:571 axfs::root:433] [AxError::IsADirectory]
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
[591.441903 0:616 axfs::root:433] [AxError::IsADirectory]
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
[594.928952 0:621 axfs::root:433] [AxError::IsADirectory]
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
[598.352186 0:626 axfs::root:433] [AxError::IsADirectory]
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
[601.838936 0:631 axfs::root:433] [AxError::IsADirectory]
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
[605.291999 0:636 axfs::root:433] [AxError::IsADirectory]
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
[608.662120 0:641 axfs::root:433] [AxError::IsADirectory]
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
[626.178072 0:668 axfs::root:433] [AxError::IsADirectory]
[626.180489 0:668 axfs::root:433] [AxError::IsADirectory]
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
[629.551774 0:670 axfs::root:433] [AxError::IsADirectory]
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
[643.884678 0:691 axfs::root:433] [AxError::IsADirectory]
[643.886826 0:691 axfs::fops:297] [AxError::NotADirectory]
[643.888165 0:691 axfs::root:433] [AxError::IsADirectory]
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
[647.411579 0:699 axfs::root:433] [AxError::IsADirectory]
[647.413298 0:699 axfs::fops:297] [AxError::NotADirectory]
[647.414196 0:699 axfs::root:433] [AxError::IsADirectory]
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
[651.975999 0:704 axfs::root:433] [AxError::IsADirectory]
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
[657.118558 0:709 axfs::root:433] [AxError::IsADirectory]
[657.119721 0:709 axfs::root:433] [AxError::IsADirectory]
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
[660.929916 0:711 axfs::root:433] [AxError::IsADirectory]
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
[665.055415 0:716 axfs::fops:297] [AxError::NotADirectory]
[665.057386 0:716 axfs::root:433] [AxError::IsADirectory]
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
[668.610703 0:721 axfs::fops:297] [AxError::NotADirectory]
[668.612374 0:721 axfs::root:433] [AxError::IsADirectory]
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
[671.998517 0:726 axfs::fops:297] [AxError::NotADirectory]
[672.000330 0:726 axfs::fops:297] [AxError::NotADirectory]
[672.002113 0:726 axfs::root:433] [AxError::IsADirectory]
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
[675.461350 0:731 axfs::fops:297] [AxError::NotADirectory]
[675.462151 0:731 axfs::fops:297] [AxError::NotADirectory]
[675.463817 0:731 axfs::root:433] [AxError::IsADirectory]
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
[678.936448 0:736 axfs::fops:297] [AxError::NotADirectory]
[678.938055 0:736 axfs::root:433] [AxError::IsADirectory]
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
[682.367876 0:741 axfs::fops:297] [AxError::NotADirectory]
[682.369533 0:741 axfs::root:433] [AxError::IsADirectory]
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
[687.007569 0:746 axfs::root:433] [AxError::IsADirectory]
[687.008718 0:746 axfs::root:433] [AxError::IsADirectory]
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
[691.567663 0:748 axfs::root:433] [AxError::IsADirectory]
[691.569732 0:748 axfs::root:433] [AxError::IsADirectory]
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
[694.983020 0:750 axfs::fops:297] [AxError::NotADirectory]
[694.985254 0:750 axfs::root:433] [AxError::IsADirectory]
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
[698.415351 0:755 axfs::fops:297] [AxError::NotADirectory]
[698.417603 0:755 axfs::root:433] [AxError::IsADirectory]
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
[701.816094 0:760 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chroot01 : 0
RUN LTP CASE chroot02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
chroot02.c:30: TFAIL: chroot(/tmp/LTP_chroOdmMF) failed: ENOSYS (38)
tst_test.c:1449: TBROK: Test haven't reported results!

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[705.425176 0:765 axfs::fops:297] [AxError::NotADirectory]
[705.426819 0:765 axfs::root:433] [AxError::IsADirectory]
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
[708.785706 0:771 axfs::fops:297] [AxError::NotADirectory]
[708.787824 0:771 axfs::root:433] [AxError::IsADirectory]
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
[712.104160 0:776 axfs::root:433] [AxError::IsADirectory]
[712.105887 0:776 axfs::root:433] [AxError::IsADirectory]
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
[716.380863 0:783 axfs::root:433] [AxError::IsADirectory]
tst_clocks.c:107: TCONF: clock_settime() not available
tst_wallclock.c:64: TBROK: tst_clock_settime() realtime failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   1
skipped  1
warnings 0
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_cloDGPNDK) failed: unlink(/tmp/LTP_cloDGPNDK) failed; errno=2: ENOENT
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
[719.855271 0:788 axfs::root:433] [AxError::IsADirectory]
tst_clocks.c:107: TCONF: clock_settime() not available
tst_wallclock.c:64: TBROK: tst_clock_settime() realtime failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   1
skipped  1
warnings 0
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_cloHoMilh) failed: unlink(/tmp/LTP_cloHoMilh) failed; errno=2: ENOENT
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
[723.646933 0:793 axfs::root:433] [AxError::IsADirectory]
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
[727.187180 0:807 axfs::root:433] [AxError::IsADirectory]
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
[730.592060 0:815 axfs::root:433] [AxError::IsADirectory]
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
clock_gettime04.c:183: TPASS: CLOCK_REALTIME_COARSE: Difference between successive readings is reasonable for following variants:
clock_gettime04.c:188: TINFO: 	- vDSO or syscall with libc spec
clock_gettime04.c:188: TINFO: 	- syscall with old kernel spec
clock_gettime04.c:188: TINFO: 	- vDSO with old kernel spec
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
[738.904684 0:822 axfs::root:433] [AxError::IsADirectory]
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
[803.060501 0:828 axfs::root:433] [AxError::IsADirectory]
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
tst_timer_test.c:305: TINFO: min 1025us, max 1319us, median 1047us, trunc mean 1051.20us (discarded 25)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 2000us 500 iterations, threshold 402.01us
tst_timer_test.c:305: TINFO: min 2024us, max 2172us, median 2045us, trunc mean 2049.60us (discarded 25)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 5000us 300 iterations, threshold 405.04us
tst_timer_test.c:305: TINFO: min 5028us, max 5194us, median 5071us, trunc mean 5067.46us (discarded 15)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 10000us 100 iterations, threshold 410.33us
tst_timer_test.c:305: TINFO: min 10073us, max 10168us, median 10107us, trunc mean 10107.15us (discarded 5)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 25000us 50 iterations, threshold 426.29us
tst_timer_test.c:305: TINFO: min 25106us, max 25421us, median 25146us, trunc mean 25144.98us (discarded 2)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 100000us 10 iterations, threshold 537.00us
tst_timer_test.c:305: TINFO: min 100144us, max 100184us, median 100151us, trunc mean 100155.00us (discarded 1)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 1000000us 2 iterations, threshold 4400.00us
tst_timer_test.c:305: TINFO: min 1000131us, max 1000169us, median 1000131us, trunc mean 1000131.00us (discarded 1)
tst_timer_test.c:326: TPASS: Measured times are within thresholds

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[814.826846 0:839 axfs::root:433] [AxError::IsADirectory]
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
[821.668763 0:847 axfs::root:433] [AxError::IsADirectory]
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
[825.235709 0:855 axfs::root:433] [AxError::IsADirectory]
tst_clocks.c:107: TCONF: clock_settime() not available
tst_wallclock.c:64: TBROK: tst_clock_settime() realtime failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   1
skipped  1
warnings 0
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_cloEImdPD) failed: unlink(/tmp/LTP_cloEImdPD) failed; errno=2: ENOENT
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
[828.667337 0:863 axfs::root:433] [AxError::IsADirectory]
tst_clocks.c:107: TCONF: clock_settime() not available
tst_wallclock.c:64: TBROK: tst_clock_settime() realtime failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   1
skipped  1
warnings 0
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_cloHmGahc) failed: unlink(/tmp/LTP_cloHmGahc) failed; errno=2: ENOENT
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
[832.167118 0:868 axfs::root:433] [AxError::IsADirectory]
tst_clocks.c:107: TCONF: clock_settime() not available
tst_wallclock.c:64: TBROK: tst_clock_settime() realtime failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   1
skipped  1
warnings 0
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_cloanCbnD) failed: unlink(/tmp/LTP_cloanCbnD) failed; errno=2: ENOENT
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
[835.760474 0:873 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clone01 : 0
RUN LTP CASE clone02
clone02     1  TFAIL  :  clone02.c:139: clone() failed: TEST_ERRNO=ENOSYS(38): Function not implemented
clone02     2  TPASS  :  Test Passed
[839.263756 0:879 axfs::root:433] [AxError::IsADirectory]
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
[842.728158 0:882 axfs::root:433] [AxError::IsADirectory]
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
[846.055162 0:888 axfs::root:433] [AxError::IsADirectory]
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
[849.572098 0:893 axfs::root:433] [AxError::IsADirectory]
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
[853.056047 0:899 axfs::root:433] [AxError::IsADirectory]
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
[856.601322 0:905 axfs::root:433] [AxError::IsADirectory]
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
[860.583846 0:911 axfs::root:433] [AxError::IsADirectory]
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
[864.103472 0:917 axfs::root:433] [AxError::IsADirectory]
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
[867.625930 0:922 axfs::fops:297] [AxError::NotADirectory]
[867.629419 0:922 axfs::root:433] [AxError::IsADirectory]
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
[871.031536 0:927 axfs::root:433] [AxError::IsADirectory]
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
[874.406505 0:932 axfs::fops:297] [AxError::NotADirectory]
[874.410254 0:932 axfs::root:433] [AxError::IsADirectory]
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
[878.120413 0:934 axfs::fops:297] [AxError::NotADirectory]
[878.122633 0:934 axfs::root:433] [AxError::IsADirectory]
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
[881.525528 0:939 axfs::root:433] [AxError::IsADirectory]
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
[886.199733 0:944 axfs::root:433] [AxError::IsADirectory]
[886.201656 0:944 axfs::root:433] [AxError::IsADirectory]
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
[889.560717 0:946 axfs::root:433] [AxError::IsADirectory]
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
[894.136344 0:954 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE confstr01 : 0
RUN LTP CASE connect01
connect01    1  TPASS  :  bad file descriptor successful
connect01    2  TPASS  :  invalid socket buffer successful
[897.707136 0:959 arceos_posix_api::imp::net:352] sys_connect => Err(EINVAL)
connect01    3  TPASS  :  invalid salen successful
connect01    4  TPASS  :  invalid socket successful
[897.714335 0:959 axnet::smoltcp_impl::tcp:197] [AxError::ConnectionRefused] socket connect() failed
[897.715792 0:959 arceos_posix_api::imp::net:352] sys_connect => Err(ECONNREFUSED)
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
[901.552096 0:962 axfs::root:433] [AxError::IsADirectory]
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
[906.214958 0:967 axfs::root:433] [AxError::IsADirectory]
[906.216077 0:967 axfs::root:433] [AxError::IsADirectory]
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
[910.682810 0:969 axfs::root:433] [AxError::IsADirectory]
[910.684736 0:969 axfs::root:433] [AxError::IsADirectory]
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
[915.644446 0:971 axfs::fops:297] [AxError::NotADirectory]
[915.645295 0:971 axfs::fops:297] [AxError::NotADirectory]
[915.649386 0:971 axfs::root:433] [AxError::IsADirectory]
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
crash01     0  TINFO  :  crashme +2000.80 961 100
crash01     1  TPASS  :  we're still here, OS seems to be robust
exit status ... number of cases
[961.852051 0:1026 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE crash01 : 0
RUN LTP CASE crash02
crash02     0  TINFO  :  crashme02 127 965 100
0000: syscall(82, 0x5b1d585b, 0xab, 0xf, 0xe704d93b, 0, 0xcde4665, 0xd08ba1bc)
0001: syscall(80, 0, 0x4783d568, 0x952e8bbd, 0, 0, 0, 0)
0002: syscall(12, 0, 0, 0x88, 0xb6ca53ef, 0x31731f58, 0x97, 0)
0003: syscall(11, 0, 0, 0x127da708, 0x5791e057, 0x16e78232, 0, 0xc98a4086)
0004: syscall(60, 0xfe66a56a, 0x168b, 0x7271604d, 0x91205272, 0, 0, 0xdc2c0793)
0005: syscall(102, 0xd6c551c4, 0x9e, 0, 0, 0, 0, 0x9c7ed674)
0006: syscall(124, 0x9b583e50, 0x856d1512, 0x7c1d2409, 0, 0, 0x75, 0)
0007: syscall(108, 0xcaf36fc9, 0, 0xe7236f93, 0, 0xb06d69a, 0x186a0ca0, 0x3b8a33fe)
0008: syscall(83, 0xeb22010b, 0, 0, 0xd9982511, 0, 0x28a7f244, 0)
0009: syscall(56, 0, 0x3, 0x8935845e, 0, 0x4ed62856, 0xd9, 0)
0010: syscall(61, 0xcec4b3ca, 0x73917a9f, 0x6b4cb551, 0, 0x5039e93f, 0, 0x5727e9fd)
0011: syscall(82, 0x4930e86e, 0, 0x2c4f9b4c, 0x6f66b98f, 0, 0, 0)
0012: syscall(56, 0, 0, 0xf4553deb, 0xa8c1da6e, 0x1e451bbe, 0, 0)
0013: syscall(96, 0xcc3d33e0, 0x5c65, 0, 0x24504569, 0, 0, 0x9a630119)
0014: syscall(46, 0, 0x3a, 0, 0xf7ccd354, 0x18e196a7, 0, 0xe97fbb2c)
0015: syscall(11, 0x7aa25141, 0, 0x358f, 0x5f, 0xc382657b, 0x71984675, 0x37abeedc)
0016: syscall(69, 0x87a3a0f, 0, 0xcb, 0, 0, 0x887dac08, 0x74c95aa3)
0017: syscall(124, 0, 0x7db83487, 0x8874420b, 0x887896fc, 0xdb1e14f1, 0x827ad54c, 0xf0dc842e)
0018: syscall(11, 0, 0xf0f4, 0, 0xe80d5963, 0xfc, 0xe91d215c, 0)
0019: syscall(33, 0xaf3f4db7, 0, 0xd247c796, 0x653a45e7, 0x6f1892c8, 0xa6aba79c, 0x77e4841a)
0020: syscall(92, 0x8f50451a, 0xd7cad5f5, 0xabfb, 0x568ed786, 0x46ef8b9e, 0x590ba2e5, 0)
0021: syscall(79, 0xbd069619, 0xa8, 0x4dbbff7b, 0, 0x3facadf0, 0x3e36cc7a, 0)
0022: syscall(65, 0, 0x354ea555, 0x83648110, 0, 0, 0x16bdccb5, 0)
0023: syscall(26, 0x94cf6eaf, 0x7f379ae0, 0xb2, 0, 0xae1e502c, 0x3ffed3a0, 0)
0024: syscall(29, 0x24, 0xe05ed660, 0, 0xe4515578, 0xe7c3, 0x2e5800c4, 0x8af7)
0025: syscall(15, 0x51d6eabe, 0x43d4ca55, 0, 0xbe, 0, 0, 0x12c8)
0026: syscall(78, 0, 0xf835567d, 0xfdf0b71d, 0x9a979c6c, 0, 0, 0xa065e4fb)
0027: syscall(113, 0, 0, 0, 0, 0x26771a2d, 0x2aa59322, 0x4c08de3)
0028: syscall(40, 0x4910cc0e, 0x94fc152, 0x140a6da4, 0x98d47df8, 0xa1, 0x5da32b54, 0)
0029: syscall(43, 0x17e, 0x490c5555, 0x8699102c, 0, 0, 0, 0)
0030: syscall(47, 0xa7e2, 0, 0, 0x56fda073, 0, 0xb664206c, 0xdf38)
0031: syscall(95, 0xa5bf6f48, 0, 0, 0x22761380, 0x1783b75b, 0x15c75f49, 0)
0032: syscall(41, 0xc0ee, 0x33b0, 0, 0xb6023127, 0xb51f15db, 0xe33d8dd0, 0x3b9107c5)
0033: syscall(79, 0, 0x3e6ee1f6, 0, 0xc4, 0x31fb8ce, 0, 0x4b3d44dc)
0034: syscall(124, 0, 0x40930630, 0x2da73935, 0x32444b0, 0, 0, 0x7efd94c)
0035: syscall(59, 0, 0xd7e5, 0x7c, 0, 0, 0x3a49cd44, 0x4a41ad0c)
0036: syscall(115, 0xa1909b46, 0x62678d20, 0, 0x77ab04dc, 0x770c71d8, 0, 0xcd2533b3)
0037: syscall(99, 0, 0x2b918720, 0x816516ce, 0, 0x84ec8ff5, 0x2979e807, 0x729943c)
0038: syscall(113, 0, 0xdc21f87f, 0, 0x8ef3b8e4, 0x16f8, 0, 0xb60abc2b)
0039: syscall(34, 0xd, 0, 0xb2618623, 0x8f1160ba, 0, 0xe626de18, 0xfbd0d908)
0040: syscall(14, 0, 0xa94f0d55, 0x5ffbfdf5, 0xa5174401, 0, 0, 0xacbc425e)
0041: syscall(92, 0x9970c083, 0x30, 0, 0xb82dd4fe, 0xfc03b5e0, 0xc06283cb, 0x8cbfb2f7)
0042: syscall(72, 0x59b096a1, 0x30a821b2, 0x484f2674, 0, 0x136ba50, 0, 0x86a6bafa)
0043: syscall(7, 0, 0, 0, 0, 0, 0x819ce847, 0)
0044: syscall(121, 0x998590d, 0x2c8c60bf, 0xea0d88d6, 0xab2125ab, 0, 0x4394, 0)
0045: syscall(19, 0, 0x4724b6b0, 0x93eadc61, 0, 0xa9, 0x67baacc, 0)
0046: syscall(43, 0, 0x3a662c7, 0, 0, 0x34293b7b, 0, 0)
0047: syscall(80, 0, 0xc1fedf8b, 0xde7beb98, 0, 0xb5e54144, 0x85525537, 0)
0048: syscall(48, 0, 0xe3aa, 0, 0xca7f0ded, 0x54234b34, 0, 0xef528ad)
0049: syscall(91, 0x268f63c0, 0, 0x48f1290d, 0x66fc564b, 0xdcee1d8b, 0xfae408a0, 0)
0050: syscall(24, 0, 0, 0, 0xd4, 0, 0x958, 0)
0051: syscall(109, 0, 0x9f, 0x7805, 0x413c68a, 0, 0, 0x46882f7e)
0052: syscall(18, 0, 0x671c32eb, 0, 0, 0xed900ba9, 0, 0x9076cabe)
0053: syscall(97, 0x9b4004b3, 0x2ea1bace, 0, 0, 0x4768b10a, 0xf385e784, 0x4886d74e)
0054: syscall(45, 0, 0xd9e4b596, 0, 0xdd42, 0xbaf5, 0xc5a4d93c, 0x5fc0756a)
0055: syscall(85, 0xe960e092, 0x5a71bc7a, 0xd060a01c, 0, 0x4012f5d0, 0xcf9b7c6, 0xca83c41b)
0056: syscall(72, 0x6acc, 0x6f2982e6, 0x66c0aef5, 0x1a, 0, 0, 0)
0057: syscall(125, 0xe177, 0xd0efd5ae, 0x1d701ab3, 0x30, 0, 0x7f0b62c4, 0x88b8)
0058: syscall(24, 0x1adbe5ce, 0, 0x6142007c, 0x5172b42b, 0, 0xf83, 0xa0)
0059: syscall(86, 0, 0x4c87f850, 0xd1fab440, 0, 0x5847fa43, 0x75d0, 0)
0060: syscall(119, 0xd09ac49c, 0x144e6687, 0x4d0fbdb1, 0, 0, 0, 0)
0061: syscall(69, 0xf, 0x580e152c, 0xeef25721, 0xd085, 0, 0xe50ab9fc, 0)
0062: syscall(104, 0, 0x3270cb73, 0, 0, 0x8b, 0, 0)
0063: syscall(119, 0xd03dead9, 0x9d5eb2c0, 0, 0, 0, 0xa9f5e5ca, 0x8453d2ed)
0064: syscall(49, 0, 0x57ef96e5, 0xd692bce3, 0x5d6e, 0xe0dc669a, 0x2d4bcc47, 0xac8a003c)
0065: syscall(65, 0x9925bead, 0x6f016315, 0x54ea4e46, 0x43a1, 0, 0, 0x72060a2e)
0066: syscall(0, 0x369104df, 0, 0, 0x512d4a81, 0, 0, 0)
0067: syscall(85, 0, 0x2ec71c3, 0, 0x4f05, 0, 0, 0)
0068: syscall(11, 0x5219099, 0, 0x9cd5b776, 0xa37a2d38, 0, 0x1d9f06e4, 0x1c8fdeaf)
0069: syscall(4, 0x3c508b9, 0xe7962e3d, 0x977497f8, 0, 0xf962b115, 0x1b973e04, 0xb60dad4e)
0070: syscall(85, 0, 0xfc18a0c1, 0, 0x6222, 0xf6, 0x16fbc4f, 0x312)
0071: syscall(122, 0, 0, 0xcb7f1903, 0xa1dfe626, 0, 0x89, 0xfebf59b8)
0072: syscall(30, 0, 0, 0x7afe49fd, 0xa93e75f1, 0x8be8f0a1, 0x45b4, 0x5e6c6615)
0073: syscall(111, 0, 0x163c5888, 0xa2f8, 0xdb1cc49, 0, 0xc3, 0x37814d33)
0074: syscall(18, 0xfc, 0x54, 0, 0, 0xa7c3c09e, 0, 0)
0075: syscall(116, 0x5f26dc8d, 0, 0x2ad6f278, 0, 0xf8410e55, 0xf0f6, 0x502a1b1e)
0076: syscall(56, 0, 0, 0x62, 0xc4182aa6, 0x440c1198, 0, 0xceabb1ed)
0077: syscall(38, 0, 0, 0, 0xfa12bd68, 0, 0xf07a2865, 0)
0078: syscall(89, 0xa2332987, 0, 0xaf710a79, 0xbc, 0xf3560e38, 0x35c0d313, 0xd7f3c647)
0079: syscall(23, 0, 0, 0, 0x2db36798, 0, 0, 0)
0080: syscall(88, 0x1dc6e098, 0, 0x6c4a2a59, 0x2875ec25, 0, 0, 0xd4c4013)
0081: syscall(70, 0xc25b88fd, 0xd9bf2c81, 0, 0xa1, 0x62f0e424, 0, 0x66)
0082: syscall(42, 0x1d0c, 0x2f7f5e3d, 0, 0xe549f965, 0xc5c23014, 0x7ea4295f, 0)
0083: syscall(96, 0x4a6948d1, 0, 0x1f36e568, 0x1274816b, 0, 0, 0)
0084: syscall(62, 0xf2475af5, 0x729ecbf1, 0xa5685e1b, 0, 0xc01a77fe, 0xaadf, 0x642c)
0085: syscall(31, 0x201220fa, 0xe90c4881, 0xc1bc87b1, 0x53d428bd, 0, 0, 0xcf4f919f)
0086: syscall(27, 0, 0x6bbcfc37, 0xb4ea5f63, 0, 0x4b2879cd, 0, 0x1e4a427a)
0087: syscall(104, 0, 0x5315377d, 0, 0x2819fa6b, 0, 0xb8fd39ab, 0x22c84ecc)
0088: syscall(32, 0, 0x63b34e2b, 0xeaf38fff, 0, 0x50cd25d8, 0xbf8c, 0x6437)
0089: syscall(103, 0, 0x8bba53b8, 0, 0xfe715713, 0x1b72, 0xe5cc, 0)
0090: syscall(89, 0xe161bd40, 0, 0x4508, 0x96, 0xb63fb247, 0, 0)
0091: syscall(86, 0xc788a0f3, 0, 0, 0x378df037, 0, 0, 0)
0092: syscall(105, 0, 0, 0, 0x67963ea7, 0xd1ba, 0xf04695a3, 0)
0093: syscall(74, 0x86f8, 0x973a31a1, 0, 0xa4fedaaa, 0x4b640fe1, 0, 0)
0094: syscall(61, 0, 0x6a, 0x5a40b87b, 0xf7a4, 0xef765d70, 0x290ffb38, 0x173e2004)
0095: syscall(116, 0x5151dd7d, 0x1f826469, 0, 0x662f22aa, 0x4ccef303, 0xde25aa83, 0x24)
0096: syscall(4, 0x4f193898, 0, 0xda3abba2, 0, 0x9829d1de, 0x7f802864, 0)
0097: syscall(79, 0, 0xca, 0, 0xcc9f2e2a, 0, 0xdf65aa29, 0x46d82585)
0098: syscall(111, 0x9421070a, 0xe21c, 0xd1414f46, 0xc1e055be, 0, 0xc9b85778, 0)
0099: syscall(5, 0, 0x5715d4ee, 0xed1dbf11, 0, 0x8c8e6b8f, 0x95c9ae68, 0x9eecbbb3)
errno status ... number of cases
           3 ...     3
           9 ...     4
          14 ...    25
          22 ...    10
          25 ...     1
          38 ...    57
crash02     1  TPASS  :  we're still here, OS seems to be robust
[965.594841 0:1031 axfs::root:433] [AxError::IsADirectory]
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
[969.389015 0:1135 axfs::fops:297] [AxError::NotADirectory]
[969.391928 0:1135 axfs::root:433] [AxError::IsADirectory]
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
[972.975972 0:1140 axfs::fops:297] [AxError::NotADirectory]
[972.979960 0:1140 axfs::root:433] [AxError::IsADirectory]
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
[976.453984 0:1145 axfs::fops:297] [AxError::NotADirectory]
[976.455437 0:1145 axfs::root:433] [AxError::IsADirectory]
[976.457040 0:1145 axfs::root:433] [AxError::IsADirectory]
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
[979.875986 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.878054 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.879198 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.880977 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.882344 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.882910 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.883486 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.884033 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.885425 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.887163 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.888926 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.889900 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.890828 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.892282 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.893359 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.895197 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.896757 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.897429 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.899142 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.899907 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.900915 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.902649 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.903438 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.905612 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.906325 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.907678 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.908276 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.908840 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.909767 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.911480 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.913393 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.914661 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.915288 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.915857 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.916742 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.918386 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.920020 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.920797 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.921455 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.923165 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.924251 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.924881 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.926513 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.927554 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.928543 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.930107 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.931545 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.932195 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.933803 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.934701 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.935349 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.936941 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.938081 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.939449 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.940052 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.941319 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.942738 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.943689 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.944269 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.944831 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.945557 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.947173 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.948935 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.950672 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.951558 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.952572 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.953769 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.954365 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.954924 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.955947 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.957522 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.959125 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.960839 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.961670 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.963379 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.964806 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.965451 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.967085 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.967867 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.968758 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.970346 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.971431 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.971993 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.973727 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.974935 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.976067 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.977393 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.978027 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.979470 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.981046 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.981846 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.982510 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.984113 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.985288 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.986495 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.987054 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.988364 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.989983 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.990940 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.991526 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.992176 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.993761 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.995551 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.996591 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.997147 0:1151 axfs::fops:297] [AxError::NotADirectory]
[979.998650 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.000085 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.001372 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.001952 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.003632 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.004864 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.005486 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.006155 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.007805 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.009404 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.010294 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.010877 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.011818 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.013617 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.015292 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.015884 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.016840 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.018520 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.019819 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.020415 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.020979 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.022337 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.023955 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.025560 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.026412 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.027002 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.028242 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.029854 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.030936 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.031527 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.032187 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.033823 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.035469 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.036345 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.036911 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.037871 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.039547 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.041088 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.041700 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.042745 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.044418 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.045667 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.046255 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.046837 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.047606 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.049162 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.050801 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.052385 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.052971 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.053666 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.055280 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.056889 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.057580 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.058421 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.059992 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.061583 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.062231 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.062819 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.063649 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.065242 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.066842 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.068342 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.068911 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.069919 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.071508 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.072803 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.073827 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.074425 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.074975 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.076173 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.077800 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.079465 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.080437 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.080995 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.082172 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.083958 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.084986 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.085593 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.086149 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.087672 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.089319 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.090712 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.091332 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.091904 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.093390 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.095084 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.096595 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.097175 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.098760 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.099808 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.100564 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.102248 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.103045 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.104379 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.105372 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.106032 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.107688 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.108731 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.109366 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.110945 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.111995 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.113562 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.114434 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.114992 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.116131 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.117772 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.118849 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.119463 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.120021 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.121312 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.122796 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.124456 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.125331 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.126569 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.127130 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.128688 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.129948 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.131136 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.132222 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.132801 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.134551 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.135847 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.136445 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.136997 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.138178 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.139803 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.141442 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.142276 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.142944 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.143543 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.144094 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.145526 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.147172 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.148958 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.149620 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.150200 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.151864 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.153481 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.154288 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.154844 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.155588 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.157249 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.158851 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.159440 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.159987 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.161275 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.163155 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.164333 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.164901 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.165977 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.167748 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.168654 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.169259 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.169856 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.170824 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.172605 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.174246 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.175678 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.176281 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.176846 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.177652 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.179431 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.181102 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.181884 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.183552 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.184260 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.184832 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.185589 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.187227 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.189016 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.189767 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.190399 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.191991 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.193339 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.194349 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.194911 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.195912 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.197539 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.198902 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.199552 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.200114 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.201693 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.203445 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.204800 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.205600 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.206175 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.207961 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.209683 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.210467 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.211045 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.212668 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.214235 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.214807 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.215430 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.217254 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.218490 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.219046 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.219847 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.220562 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.221141 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.221867 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.222455 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.223054 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.223700 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.224292 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.224869 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.225463 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.226050 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.226635 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.227231 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.227813 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.228404 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.229008 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.229668 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.230262 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.230832 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.231419 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.231979 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.232795 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.233420 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.233989 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.234627 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.235187 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.235781 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.236356 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.236911 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.237505 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.238096 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.238702 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.239393 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.240001 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.240593 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.241154 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.241744 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.242427 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.242999 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.243607 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.244177 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.244802 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.245432 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.246010 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.246604 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.247159 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.247753 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.248330 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.248985 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.249588 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.250161 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.250796 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.251387 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.251960 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.252713 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.253284 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.253850 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.254448 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.255034 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.255619 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.256238 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.256809 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.257395 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.257947 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.258616 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.259176 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.259760 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.260358 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.260941 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.261574 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.262231 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.262810 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.263404 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.263963 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.264543 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.265105 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.265706 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.266292 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.266874 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.267504 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.268154 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.268745 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.269334 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.269890 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.270472 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.271045 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.271650 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.272278 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.272884 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.273486 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.274051 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.274647 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.275202 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.275782 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.276365 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.276948 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.277649 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.278279 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.278863 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.279460 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.280025 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.280611 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.281164 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.281755 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.282432 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.283016 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.283640 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.284246 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.284815 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.285409 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.285966 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.286556 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.287224 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.287808 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.288410 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.288988 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.289619 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.290179 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.290761 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.291347 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.291906 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.292626 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.293230 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.293815 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.294402 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.295028 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.295650 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.296236 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.296894 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.297484 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.298057 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.298663 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.299271 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.299839 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.300462 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.301032 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.301622 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.302289 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.302857 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.303438 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.304002 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.304597 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.304986 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.305530 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.306165 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.306807 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.307404 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.307979 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.308567 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.309129 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.309705 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.310296 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.310871 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.311457 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.312010 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.312757 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.313349 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.313961 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.314566 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.315127 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.315853 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.316476 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.317056 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.317662 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.318270 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.318882 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.319472 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.320026 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.320633 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.321184 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.321779 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.322502 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.323095 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.323691 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.324333 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.324896 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.325583 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.326160 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.326751 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.327360 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.327948 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.328560 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.329149 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.329785 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.330383 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.330954 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.331555 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.332193 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.332797 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.333386 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.333962 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.334585 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.335311 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.335905 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.336504 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.337068 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.337663 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.338241 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.338823 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.339435 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.340007 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.340635 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.341223 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.341811 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.342547 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.343130 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.343723 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.344328 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.344992 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.345618 0:1151 axfs::fops:297] [AxError::NotADirectory]
[980.347087 0:1151 axfs::root:433] [AxError::IsADirectory]
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_crekmCpCh) failed: remove(/tmp/LTP_crekmCpCh) failed; errno=39: ENOTEMPTY
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
[987.016831 0:1156 axfs::root:433] [AxError::IsADirectory]
[987.017998 0:1156 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE creat06 : 6
RUN LTP CASE creat07
tst_test.c:949: TBROK: Failed to copy resource 'creat07_child'

Summary:
passed   0
failed   0
broken   1
skipped  0
warnings 0
[990.306467 0:1158 axfs::fops:297] [AxError::NotADirectory]
[990.307985 0:1158 axfs::root:433] [AxError::IsADirectory]
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
[997.080770 0:1162 axfs::root:433] [AxError::IsADirectory]
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
[1001.521333 0:1167 axfs::root:433] [AxError::IsADirectory]
[1001.522526 0:1167 axfs::root:433] [AxError::IsADirectory]
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
[1006.948007 0:1173 axfs::root:433] [AxError::IsADirectory]
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
[1010.556634 0:1178 axfs::root:433] [AxError::IsADirectory]
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
[1014.126521 0:1183 axfs::root:433] [AxError::IsADirectory]
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
[1020.637765 0:1190 axfs::root:433] [AxError::IsADirectory]
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
[1023.979960 0:1195 axfs::root:433] [AxError::IsADirectory]
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
[1027.583722 0:1200 axfs::root:433] [AxError::IsADirectory]
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
[1078.473691 0:1224 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1081.318143 0:1219 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1084.110450 0:1217 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1087.079309 0:1216 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1090.210644 0:1212 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1093.308858 0:1214 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1096.777300 0:1211 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
cve-2017-17052.c:106: TFAIL: child exited with 2
[1096.979274 0:4801 axmm:27] Mapping error: BadState
cve-2017-17052.c:67: TBROK: fork() failed: EFAULT (14)
[1097.008958 0:1213 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1097.036762 0:4803 axmm:27] Mapping error: BadState
cve-2017-17052.c:67: TBROK: fork() failed: EFAULT (14)
cve-2017-17052.c:106: TFAIL: child exited with 2
[1097.344304 0:4806 axmm:27] Mapping error: BadState
cve-2017-17052.c:66: TBROK: fork() failed: EFAULT (14)
cve-2017-17052.c:106: TFAIL: child exited with 2
[1097.784298 0:4807 axmm:27] Mapping error: BadState
cve-2017-17052.c:66: TBROK: fork() failed: EFAULT (14)
cve-2017-17052.c:106: TFAIL: child exited with 2
cve-2017-17052.c:113: TPASS: kernel survived 4 runs

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[1098.222685 0:1207 axfs::root:433] [AxError::IsADirectory]
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
[1104.813824 0:4811 axfs::root:433] [AxError::IsADirectory]
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
[1108.440892 0:4816 axfs::root:433] [AxError::IsADirectory]
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
[1117.885320 0:4826 axmm:27] Mapping error: BadState
data_space    1  TBROK  :  data_space.c:159: fork failed: errno=EFAULT(14): Bad address
data_space    2  TBROK  :  data_space.c:159: Remaining cases broken
[1117.902195 0:4826 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE data_space : 2
RUN LTP CASE dccp01.sh
[1118.018471 0:4830 axmm:27] Mapping error: BadState
FAIL LTP CASE dccp01.sh : 2
RUN LTP CASE dccp_ipsec.sh
[1118.102858 0:4831 axmm:27] Mapping error: BadState
FAIL LTP CASE dccp_ipsec.sh : 2
RUN LTP CASE dccp_ipsec_vti.sh
[1118.205790 0:4832 axmm:27] Mapping error: BadState
FAIL LTP CASE dccp_ipsec_vti.sh : 2
RUN LTP CASE dctcp01.sh
[1118.303148 0:4833 axmm:27] Mapping error: BadState
FAIL LTP CASE dctcp01.sh : 2
RUN LTP CASE delete_module01
[1118.397669 0:4834 axmm::aspace:112] [AxError::BadState] failed to materialize child page
FAIL LTP CASE delete_module01 : 2
RUN LTP CASE delete_module02
[1118.486762 0:4835 page_table_multiarch::bits64:490] failed to map page: 0x1400000(Size4K) -> PA:0x0, NoMemory
[1118.488243 0:4835 axmm:27] Mapping error: BadState
FAIL LTP CASE delete_module02 : 2
RUN LTP CASE delete_module03
[1118.591176 0:4836 axmm:27] Mapping error: BadState
FAIL LTP CASE delete_module03 : 2
RUN LTP CASE df01.sh
[1118.682057 0:4837 axmm:27] Mapping error: BadState
FAIL LTP CASE df01.sh : 2
RUN LTP CASE dhcp_lib.sh
SKIP LTP CASE dhcp_lib.sh : LTP shell helper library is not a standalone test
FAIL LTP CASE dhcp_lib.sh : 32
RUN LTP CASE dhcpd_tests.sh
[1118.868529 0:4839 axmm:27] Mapping error: BadState
FAIL LTP CASE dhcpd_tests.sh : 2
RUN LTP CASE dio_append
[1118.969523 0:4840 axmm:27] Mapping error: BadState
FAIL LTP CASE dio_append : 2
RUN LTP CASE dio_read
[1119.061974 0:4841 axmm:27] Mapping error: BadState
FAIL LTP CASE dio_read : 2
RUN LTP CASE dio_sparse
[1119.158332 0:4842 axmm:27] Mapping error: BadState
FAIL LTP CASE dio_sparse : 2
RUN LTP CASE dio_truncate
[1119.251162 0:4843 axmm:27] Mapping error: BadState
FAIL LTP CASE dio_truncate : 2
RUN LTP CASE diotest1
[1119.333066 0:280 axmm:27] Mapping error: BadState
/tmp/testsuite/musl-ltp-script/ltp_testcode.sh: line 15: can't fork: Bad address
#### OS COMP TEST GROUP START lua-musl ####
[1120.853647 0:4844 axmm:27] Mapping error: BadState
./lua_testcode.sh: line 3: can't fork: Bad address
[1121.180581 0:4847 axmm:27] Mapping error: BadState
sh: can't fork: Bad address
#### OS COMP TEST GROUP START unixbench-musl ####
SKIP: unixbench currently blocks on unresolved executable/runtime compatibility
#### OS COMP TEST GROUP END unixbench-musl ####
[1121.759511 0:4848 axmm:27] Mapping error: BadState
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
Thu Jan  1 00:18:46 UTC 1970
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
 00:18:53 up 0 min,  0 users,  load average: 0.00, 0.00, 0.00
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
Thu Jan  1 00:18:57 1970  0.000000 seconds
testcase busybox hwclock success
[1137.416695 0:4871 axmm:27] Mapping error: BadState
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
[1151.244039 0:4890 axmm:27] Mapping error: BadState
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
[1160.974082 0:2 axmm:27] Mapping error: BadState
autorun: /tmp/testsuite/glibc/cyclictest/cyclictest_testcode.sh failed: failed to map user stack: Bad internal state
[1161.253481 0:2 axmm:27] Mapping error: BadState
autorun: /tmp/testsuite/glibc/iozone/iozone_testcode.sh failed: failed to map user stack: Bad internal state
[1161.677235 0:2 axmm:27] Mapping error: BadState
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
[1161.994541 0:2 axmm:27] Mapping error: BadState
autorun: /glibc/ltp_testcode.sh failed: failed to map user stack: Bad internal state
[1162.379734 0:2 axmm:27] Mapping error: BadState
autorun: /tmp/testsuite/glibc/lua/lua_testcode.sh failed: failed to map user stack: Bad internal state
[1162.683558 0:2 axmm:27] Mapping error: BadState
autorun: /tmp/testsuite/glibc/netperf/netperf_testcode.sh failed: failed to map user stack: Bad internal state
#### OS COMP TEST GROUP START unixbench-glibc ####
SKIP: unixbench currently blocks on unresolved executable/runtime compatibility
#### OS COMP TEST GROUP END unixbench-glibc ####
[1162.698539 0:2 axplat_riscv64_qemu_virt::power:28] Shutting down...
```
