# RV evaluation output

Source: full RV evaluation log for the current personality-syscall compatibility candidate.
Recorded metrics: top-level 116 pass-like / 4 fail-like / 55 skip.
Comparison with previous pushed output: top-level delta 0 pass-like / 0 fail-like / 0 skip; LTP diagnostic delta TPASS 0, TFAIL 0, TBROK -1, TCONF 1.
Target note: cve-2016-10044 no longer fails on personality() ENOSYS, but it still does not pass because io_setup is unsupported.
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
    --> examples/shell/src/uspace.rs:7102:8
     |
6551 | impl FdTable {
     | ------------ methods in this implementation
...
7102 |     fn insert(&mut self, entry: FdEntry) -> Result<i32, LinuxError> {
     |        ^^^^^^
...
7110 |     fn insert_min(&mut self, entry: FdEntry, min_fd: usize) -> Result<i32, LinuxError> {
     |        ^^^^^^^^^^
     |
     = note: `#[warn(dead_code)]` on by default

warning: `arceos-shell` (bin "arceos-shell") generated 1 warning
    Finished `release` profile [optimized] target(s) in 0.51s
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

[  0.174720 0 axruntime:135] Logging is enabled.
[  0.181521 0 axruntime:136] Primary CPU 0 started, arg = 0xbfe00000.
[  0.184849 0 axruntime:139] Found physcial memory regions:
[  0.187199 0 axruntime:141]   [PA:0x101000, PA:0x102000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.190383 0 axruntime:141]   [PA:0xc000000, PA:0xc210000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.194461 0 axruntime:141]   [PA:0x10000000, PA:0x10001000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.195105 0 axruntime:141]   [PA:0x10001000, PA:0x10009000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.197479 0 axruntime:141]   [PA:0x30000000, PA:0x40000000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.200742 0 axruntime:141]   [PA:0x40000000, PA:0x80000000) mmio (READ | WRITE | DEVICE | RESERVED)
[  0.201446 0 axruntime:141]   [PA:0x80200000, PA:0x802af000) .text (READ | EXECUTE | RESERVED)
[  0.202055 0 axruntime:141]   [PA:0x802af000, PA:0x802d4000) .rodata (READ | RESERVED)
[  0.202634 0 axruntime:141]   [PA:0x802d4000, PA:0x802d7000) .data .tdata .tbss .percpu (READ | WRITE | RESERVED)
[  0.206391 0 axruntime:141]   [PA:0x802d7000, PA:0x80317000) boot stack (READ | WRITE | RESERVED)
[  0.208752 0 axruntime:141]   [PA:0x80317000, PA:0x80340000) .bss (READ | WRITE | RESERVED)
[  0.211352 0 axruntime:141]   [PA:0x80340000, PA:0xc0000000) free memory (READ | WRITE | FREE)
[  0.213930 0 axruntime:216] Initialize global memory allocator...
[  0.215423 0 axruntime:217]   use TLSF allocator.
[  0.226525 0 axmm:103] Initialize virtual memory management...
[  0.511017 0 axruntime:156] Initialize platform devices...
smp = 1
[  0.514122 0 axtask::api:73] Initialize scheduling...
[  0.521714 0 axtask::api:83]   use FIFO scheduler.
[  0.522429 0 axdriver:152] Initialize device drivers...
[  0.523139 0 axdriver:153]   device model: static
[  0.527224 0 virtio_drivers::device::blk:63] found a block device of size 4194304KB
[  0.534475 0 axdriver::bus::mmio:11] registered a new Block device at [PA:0x10001000, PA:0x10002000): "virtio-blk"
[  0.538604 0 virtio_drivers::device::net::dev_raw:33] negotiated_features Features(MAC | STATUS | RING_INDIRECT_DESC | RING_EVENT_IDX)
[  0.550435 0 axdriver::bus::mmio:11] registered a new Net device at [PA:0x10008000, PA:0x10009000): "virtio-net"
[  0.552751 0 axfs:44] Initialize filesystems...
[  0.555391 0 axfs:47]   use block device 0: "virtio-blk"
[  0.562511 0 axfs::root:336]   detected root filesystem: Ext4
[  0.642097 0 axnet:42] Initialize network subsystem...
[  0.644146 0 axnet:45]   use NIC 0: "virtio-net"
[  0.661561 0 axnet::smoltcp_impl:335] created net interface "eth0":
[  0.665126 0 axnet::smoltcp_impl:336]   ether:    52-54-00-12-34-56
[  0.668410 0 axnet::smoltcp_impl:337]   ip:       10.0.2.15/24
[  0.672608 0 axnet::smoltcp_impl:338]   gateway:  10.0.2.2
[  0.675473 0 axruntime:182] Initialize interrupt handlers...
[  0.677427 0 axruntime:194] Primary CPU 0 init OK.
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
start:10857, end:11102
interval: 245
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
  I am child process: 47. iteration -2144509952.
  I am child process: 48. iteration -2144509952.
  I am child process: 49. iteration -2144509952.
  I am child process: 47. iteration -2144509952.
  I am child process: 48. iteration -2144509952.
  I am child process: 49. iteration -2144509952.
  I am child process: 47. iteration -2144509952.
  I am child process: 48. iteration -2144509952.
  I am child process: 49. iteration -2144509952.
  I am child process: 47. iteration -2144509952.
  I am child process: 48. iteration -2144509952.
  I am child process: 49. iteration -2144509952.
  I am child process: 47. iteration -2144509952.
  I am child process: 48. iteration -2144509952.
  I am child process: 49. iteration -2144509952.
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
Thu Jan  1 00:00:25 UTC 1970
testcase busybox date success
Filesystem           1K-blocks      Used Available Use% Mounted on
devfs                  1045248     36648   1008600   4% /dev
tmpfs                  1045248     36648   1008600   4% /tmp
tmpfs                  1045248     36648   1008600   4% /var
proc                   1045248     36648   1008600   4% /proc
sysfs                  1045248     36648   1008600   4% /sys
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
 00:00:34 up 0 min,  0 users,  load average: 0.00, 0.00, 0.00
testcase busybox uptime success
abc
testcase busybox printf "abc\n" success
PID   USER     TIME  COMMAND
testcase busybox ps success
/tmp/testsuite/musl/busybox
testcase busybox pwd success
              total        used        free      shared  buff/cache   available
Mem:              0           0           0           0           0     1039833
-/+ buffers/cache:            0           0
Swap:             0           0           0
testcase busybox free success
Thu Jan  1 00:00:38 1970  0.000000 seconds
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
T: 0 (  113) P:99 I:1000 C:    992 Min:      5 Act:    7 Avg:   35 Max:    2115
====== cyclictest NO_STRESS_P1 end: success ======
====== cyclictest NO_STRESS_P8 begin ======
WARN: stat /dev/cpu_dma_latency failed: No such file or directory
T: 0 (  115) P:99 I:1000 C:    999 Min:      4 Act:   25 Avg:   30 Max:    1222
T: 1 (  116) P:99 I:1500 C:    666 Min:      4 Act:  131 Avg:   33 Max:    1566
T: 2 (  117) P:99 I:2000 C:    499 Min:      4 Act:  643 Avg:   32 Max:    2028
T: 3 (  118) P:99 I:2500 C:    400 Min:      4 Act:  638 Avg:   25 Max:     638
T: 4 (  119) P:99 I:3000 C:    334 Min:      4 Act:   27 Avg:   23 Max:     772
T: 5 (  120) P:99 I:3500 C:    286 Min:      4 Act:   29 Avg:   30 Max:     510
T: 6 (  121) P:99 I:4000 C:    250 Min:      3 Act:  775 Avg:   24 Max:     775
T: 7 (  122) P:99 I:4500 C:    223 Min:      4 Act:   13 Avg:   25 Max:     681
====== cyclictest NO_STRESS_P8 end: success ======
====== start hackbench ======
Running in process mode with 10 groups using 40 file descriptors each (== 400 tasks)
Each sender will pass 100000000 messages of 100 bytes
Creating fdpair (error: Function not implemented)
====== cyclictest STRESS_P1 begin ======
WARN: stat /dev/cpu_dma_latency failed: No such file or directory
T: 0 (  126) P:99 I:1000 C:    885 Min:      4 Act:   11 Avg:  202 Max:    3781
====== cyclictest STRESS_P1 end: success ======
====== cyclictest STRESS_P8 begin ======
WARN: stat /dev/cpu_dma_latency failed: No such file or directory
T: 0 (  128) P:99 I:1000 C:    992 Min:      4 Act:   12 Avg:   40 Max:    2725
T: 1 (  129) P:99 I:1500 C:    662 Min:      5 Act:   75 Avg:   42 Max:    1983
T: 2 (  130) P:99 I:2000 C:    500 Min:      4 Act:  581 Avg:   29 Max:    1397
T: 3 (  131) P:99 I:2500 C:    400 Min:      5 Act:  573 Avg:   40 Max:    1854
T: 4 (  132) P:99 I:3000 C:    334 Min:      4 Act:   22 Avg:   21 Max:     311
T: 5 (  133) P:99 I:3500 C:    286 Min:      4 Act:   31 Avg:   35 Max:    2267
T: 6 (  134) P:99 I:4000 C:    250 Min:      5 Act:  450 Avg:   54 Max:    1752
T: 7 (  135) P:99 I:4500 C:    222 Min:      5 Act:   18 Avg:   46 Max:    1102
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

	Run began: Thu Jan  1 00:01:25 1970

	Auto Mode
	Record Size 1 kB
	File size set to 4096 kB
	Command line used: ./iozone -a -r 1k -s 4m
	Output is in kBytes/sec
	Time Resolution = 0.000006 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
                                                                    random    random      bkwd     record     stride                                        
              kB  reclen    write    rewrite      read    reread      read     write      read    rewrite       read    fwrite  frewrite     fread   freread
            4096       1     24944     37650     43048     44222     27858     57680[ 86.739619 0:143 axfs::fops:269] [AxError::InvalidInput]
[ 86.812264 0:143 axfs::fops:269] [AxError::InvalidInput]
     32558      63478      57570     79719     79237     51143     52293

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

	Run began: Thu Jan  1 00:01:28 1970

	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 0 -i 1 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000007 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes

Unable to get shared memory segment(shmget)
shmid = -1, size = 32, size1 = 8192, Error 38
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

	Run began: Thu Jan  1 00:01:30 1970

	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 0 -i 2 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000006 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes

Unable to get shared memory segment(shmget)
shmid = -1, size = 32, size1 = 8192, Error 38
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

	Run began: Thu Jan  1 00:01:32 1970

	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 0 -i 3 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000006 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes

Unable to get shared memory segment(shmget)
shmid = -1, size = 32, size1 = 8192, Error 38
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

	Run began: Thu Jan  1 00:01:34 1970

	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 0 -i 5 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000007 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes

Unable to get shared memory segment(shmget)
shmid = -1, size = 32, size1 = 8192, Error 38
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

	Run began: Thu Jan  1 00:01:36 1970

	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 6 -i 7 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000006 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes

Unable to get shared memory segment(shmget)
shmid = -1, size = 32, size1 = 8192, Error 38
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

	Run began: Thu Jan  1 00:01:38 1970

	Record Size 1 kB
	File size set to 1024 kB
	Command line used: ./iozone -t 4 -i 9 -i 10 -r 1k -s 1m
	Output is in kBytes/sec
	Time Resolution = 0.000007 seconds.
	Processor cache size set to 1024 kBytes.
	Processor cache line size set to 32 bytes.
	File stride size set to 17 * record size.
	Throughput test with 4 processes

Unable to get shared memory segment(shmget)
shmid = -1, size = 32, size1 = 8192, Error 38
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

	Run began: Thu Jan  1 00:01:40 1970

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

Unable to get shared memory segment(shmget)
shmid = -1, size = 32, size1 = 8192, Error 38
#### OS COMP TEST GROUP END iozone-musl ####
#### OS COMP TEST GROUP START iperf-musl ####
====== iperf BASIC_UDP begin ======
warning: Ignoring nonsense TCP MSS 0
Connecting to host 127.0.0.1, port 5001
[  5] local 0.0.0.0 port 49152 connected to 127.0.0.1 port 5001
[ ID] Interval           Transfer     Bitrate         Total Datagrams
[  5]   0.00-2.00   sec  6.79 MBytes  28.5 Mbits/sec  4874  
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-2.00   sec  6.79 MBytes  28.5 Mbits/sec  0.000 ms  0/4874 (0%)  sender
[  5]   0.00-2.00   sec  6.79 MBytes  28.4 Mbits/sec  0.188 ms  0/4874 (0%)  receiver

iperf Done.
====== iperf BASIC_UDP end: success ======

====== iperf BASIC_TCP begin ======
warning: Ignoring nonsense TCP MSS 0
Connecting to host 127.0.0.1, port 5001
[  5] local 127.0.0.1 port 49154 connected to 127.0.0.1 port 5001
iperf3: getsockopt - Invalid argument
[ ID] Interval           Transfer     Bitrate         Retr  Cwnd
[  5]   0.00-2.00   sec  54.5 MBytes   228 Mbits/sec    0   21.7 MBytes       
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Retr
[  5]   0.00-2.00   sec  54.5 MBytes   228 Mbits/sec    0             sender
[  5]   0.00-2.02   sec  53.6 MBytes   223 Mbits/sec                  receiver

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
[  5]   0.00-2.00   sec  1.13 MBytes  4.74 Mbits/sec  812  
[  7]   0.00-2.00   sec  1.13 MBytes  4.73 Mbits/sec  812  
[  9]   0.00-2.00   sec  1.13 MBytes  4.73 Mbits/sec  812  
[ 11]   0.00-2.00   sec  1.13 MBytes  4.73 Mbits/sec  812  
[ 13]   0.00-2.00   sec  1.13 MBytes  4.73 Mbits/sec  812  
[SUM]   0.00-2.00   sec  5.65 MBytes  23.7 Mbits/sec  4060  
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-2.00   sec  1.13 MBytes  4.74 Mbits/sec  0.000 ms  0/812 (0%)  sender
[  5]   0.00-2.01   sec  5.65 MBytes  23.6 Mbits/sec  0.258 ms  959981401/959985462 (1e+02%)  receiver
[  7]   0.00-2.00   sec  1.13 MBytes  4.74 Mbits/sec  0.000 ms  0/812 (0%)  sender
[  7]   0.00-2.01   sec  5.65 MBytes  23.6 Mbits/sec  0.259 ms  959981401/959985462 (1e+02%)  receiver
[  9]   0.00-2.00   sec  1.13 MBytes  4.74 Mbits/sec  0.000 ms  0/812 (0%)  sender
[  9]   0.00-2.01   sec  5.65 MBytes  23.6 Mbits/sec  0.257 ms  1551438367/1551442428 (1e+02%)  receiver
[ 11]   0.00-2.00   sec  1.13 MBytes  4.74 Mbits/sec  0.000 ms  0/812 (0%)  sender
[ 11]   0.00-2.01   sec  5.65 MBytes  23.6 Mbits/sec  0.257 ms  1323731562/1323735623 (1e+02%)  receiver
[ 13]   0.00-2.00   sec  1.13 MBytes  4.74 Mbits/sec  0.000 ms  0/812 (0%)  sender
[ 13]   0.00-2.01   sec  5.65 MBytes  23.6 Mbits/sec  0.256 ms  0/812 (0%)  receiver
[SUM]   0.00-2.00   sec  5.65 MBytes  23.7 Mbits/sec  0.000 ms  0/4060 (0%)  sender
[SUM]   0.00-2.01   sec  28.3 MBytes   118 Mbits/sec  0.257 ms  500165435/500182491 (1.2e+07%)  receiver

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
[  5]   0.00-2.01   sec  11.0 MBytes  46.0 Mbits/sec    0   22.7 MBytes       
[  7]   0.00-2.01   sec  11.0 MBytes  45.8 Mbits/sec    0   22.7 MBytes       
[  9]   0.00-2.02   sec  11.0 MBytes  45.8 Mbits/sec    0   22.7 MBytes       
[ 11]   0.00-2.02   sec  11.0 MBytes  45.8 Mbits/sec    0   22.7 MBytes       
[ 13]   0.00-2.02   sec  11.0 MBytes  45.7 Mbits/sec    0   22.7 MBytes       
[SUM]   0.00-2.01   sec  55.0 MBytes   230 Mbits/sec    0             
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Retr
[  5]   0.00-2.01   sec  11.0 MBytes  46.0 Mbits/sec    0             sender
[  5]   0.00-2.05   sec  10.1 MBytes  41.5 Mbits/sec                  receiver
[  7]   0.00-2.01   sec  11.0 MBytes  46.0 Mbits/sec    0             sender
[  7]   0.00-2.05   sec  10.1 MBytes  41.5 Mbits/sec                  receiver
[  9]   0.00-2.01   sec  11.0 MBytes  46.0 Mbits/sec    0             sender
[  9]   0.00-2.05   sec  10.1 MBytes  41.5 Mbits/sec                  receiver
[ 11]   0.00-2.01   sec  11.0 MBytes  46.0 Mbits/sec    0             sender
[ 11]   0.00-2.05   sec  10.1 MBytes  41.5 Mbits/sec                  receiver
[ 13]   0.00-2.01   sec  11.0 MBytes  46.0 Mbits/sec    0             sender
[ 13]   0.00-2.05   sec  10.1 MBytes  41.5 Mbits/sec                  receiver
[SUM]   0.00-2.01   sec  55.0 MBytes   230 Mbits/sec    0             sender
[SUM]   0.00-2.05   sec  50.6 MBytes   207 Mbits/sec                  receiver

iperf Done.
====== iperf PARALLEL_TCP end: success ======

====== iperf REVERSE_UDP begin ======
warning: Ignoring nonsense TCP MSS 0
Connecting to host 127.0.0.1, port 5001
Reverse mode, remote host 127.0.0.1 is sending
[  5] local 0.0.0.0 port 49158 connected to 127.0.0.1 port 5001
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-2.00   sec  6.15 MBytes  25.8 Mbits/sec  0.091 ms  0/4415 (0%)  
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-2.00   sec  6.15 MBytes  25.7 Mbits/sec  0.000 ms  0/4416 (0%)  sender
[  5]   0.00-2.00   sec  6.15 MBytes  25.8 Mbits/sec  0.091 ms  0/4415 (0%)  receiver

iperf Done.
====== iperf REVERSE_UDP end: success ======

====== iperf REVERSE_TCP begin ======
warning: Ignoring nonsense TCP MSS 0
Connecting to host 127.0.0.1, port 5001
Reverse mode, remote host 127.0.0.1 is sending
[  5] local 127.0.0.1 port 49164 connected to 127.0.0.1 port 5001
iperf3: getsockopt - Invalid argument
[ ID] Interval           Transfer     Bitrate         Retr  Cwnd
[  5]   0.00-2.01   sec  56.6 MBytes   237 Mbits/sec                  
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Retr
[  5]   0.00-2.03   sec  57.5 MBytes   238 Mbits/sec    0             sender
[  5]   0.00-2.01   sec  56.6 MBytes   237 Mbits/sec                  receiver

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
[127.189978 0:174 axfs::root:433] [AxError::IsADirectory]
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
[138.505016 0:185 axnet::smoltcp_impl::tcp:284] [AxError::InvalidInput] socket accept() failed: not listen
[138.508032 0:185 arceos_posix_api::imp::net:486] sys_accept => Err(EINVAL)
accept01.c:92: TPASS: invalid socket buffer : EINVAL (22)
[138.509235 0:185 axnet::smoltcp_impl::tcp:284] [AxError::InvalidInput] socket accept() failed: not listen
[138.513832 0:185 arceos_posix_api::imp::net:486] sys_accept => Err(EINVAL)
accept01.c:92: TPASS: invalid salen : EINVAL (22)
[138.517587 0:185 axnet::smoltcp_impl::tcp:284] [AxError::InvalidInput] socket accept() failed: not listen
[138.518183 0:185 arceos_posix_api::imp::net:486] sys_accept => Err(EINVAL)
accept01.c:92: TPASS: no queued connections : EINVAL (22)
[138.519123 0:185 arceos_posix_api::imp::net:486] sys_accept => Err(EOPNOTSUPP)
accept01.c:92: TPASS: UDP accept : EOPNOTSUPP (95)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[138.587003 0:182 axfs::root:433] [AxError::IsADirectory]
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
[144.514557 0:187 axfs::fops:297] [AxError::NotADirectory]
[144.518146 0:187 axfs::root:433] [AxError::IsADirectory]
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
[150.451597 0:194 axfs::root:433] [AxError::IsADirectory]
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
[156.782625 0:199 axfs::root:433] [AxError::IsADirectory]
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
[164.066491 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.067929 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.068719 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.069930 0:210 axfs::root:433] [AxError::IsADirectory]
[164.072353 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.073509 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.074628 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.075436 0:210 axfs::root:433] [AxError::IsADirectory]
[164.077372 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.078704 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.079314 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.079992 0:210 axfs::root:433] [AxError::IsADirectory]
[164.081622 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.083231 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.085018 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.085921 0:210 axfs::root:433] [AxError::IsADirectory]
[164.086904 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.088592 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.089508 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.090133 0:210 axfs::root:433] [AxError::IsADirectory]
[164.091907 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.093544 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.094734 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.095688 0:210 axfs::root:433] [AxError::IsADirectory]
[164.096425 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.098008 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.099499 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.100370 0:210 axfs::fops:297] [AxError::NotADirectory]
[164.102140 0:210 axfs::root:433] [AxError::IsADirectory]
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
[169.825163 0:228 axfs::fops:297] [AxError::NotADirectory]
[169.826644 0:228 axfs::fops:297] [AxError::NotADirectory]
[169.827743 0:228 axfs::fops:297] [AxError::NotADirectory]
[169.828365 0:228 axfs::fops:297] [AxError::NotADirectory]
[169.829875 0:228 axfs::root:433] [AxError::IsADirectory]
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
[176.154842 0:233 axfs::root:433] [AxError::IsADirectory]
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
[184.386448 0:242 axfs::root:433] [AxError::IsADirectory]
[184.392190 0:242 axfs::root:433] [AxError::IsADirectory]
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
[192.175366 0:244 axfs::root:433] [AxError::IsADirectory]
[192.177545 0:244 axfs::root:433] [AxError::IsADirectory]
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
[207.243960 0:254 axfs::root:433] [AxError::IsADirectory]
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
[214.232681 0:259 axfs::root:433] [AxError::IsADirectory]
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
[219.997855 0:264 axfs::root:433] [AxError::IsADirectory]
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
[224.917787 0:269 axfs::root:433] [AxError::IsADirectory]
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
[231.623204 0:276 axfs::root:433] [AxError::IsADirectory]
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
[234.986168 0:281 axfs::root:433] [AxError::IsADirectory]
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
[238.355060 0:286 axfs::root:433] [AxError::IsADirectory]
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
[262.496739 0:310 axfs::root:433] [AxError::IsADirectory]
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
[265.914857 0:318 axfs::root:433] [AxError::IsADirectory]
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
[272.285717 0:325 axfs::root:433] [AxError::IsADirectory]
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
[278.580994 0:332 axfs::root:433] [AxError::IsADirectory]
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
[284.890030 0:338 axfs::root:433] [AxError::IsADirectory]
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
[293.240485 0:351 arceos_posix_api::imp::net:333] sys_bind => Err(EINVAL)
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
[296.746383 0:353 axfs::root:433] [AxError::IsADirectory]
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
[311.701243 0:368 axfs::fops:297] [AxError::NotADirectory]
[311.702919 0:368 axfs::root:433] [AxError::IsADirectory]
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
[316.882110 0:380 arceos_posix_api::imp::net:333] sys_bind => Err(EINVAL)
bind01.c:60: TPASS: invalid salen : EINVAL (22)
bind01.c:60: TPASS: invalid socket : ENOTSOCK (88)
bind01.c:63: TPASS: INADDR_ANYPORT passed
[316.886513 0:380 arceos_posix_api::imp::net:333] sys_bind => Err(EINVAL)
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
[316.936977 0:377 axfs::fops:297] [AxError::NotADirectory]
[316.939192 0:377 axfs::root:433] [AxError::IsADirectory]
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
[320.440116 0:382 axfs::root:433] [AxError::IsADirectory]
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
[323.984361 0:387 axfs::root:433] [AxError::IsADirectory]
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
[327.715010 0:392 axfs::root:433] [AxError::IsADirectory]
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
[331.301800 0:397 axfs::root:433] [AxError::IsADirectory]
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
[344.637011 0:415 axfs::root:433] [AxError::IsADirectory]
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
[348.211155 0:420 axfs::root:433] [AxError::IsADirectory]
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
[351.805275 0:425 axfs::root:433] [AxError::IsADirectory]
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
[355.414934 0:430 axfs::root:433] [AxError::IsADirectory]
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
[359.000683 0:435 axfs::root:433] [AxError::IsADirectory]
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
[362.604438 0:440 axfs::root:433] [AxError::IsADirectory]
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
[366.249063 0:445 axfs::root:433] [AxError::IsADirectory]
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
[369.766576 0:450 axfs::root:433] [AxError::IsADirectory]
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
[373.341184 0:455 axfs::root:433] [AxError::IsADirectory]
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
[376.741476 0:463 axfs::root:433] [AxError::IsADirectory]
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
[413.838101 0:508 axfs::root:433] [AxError::IsADirectory]
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
[417.270205 0:513 axfs::root:433] [AxError::IsADirectory]
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
[420.696675 0:518 axfs::root:433] [AxError::IsADirectory]
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
[424.214672 0:523 axfs::root:433] [AxError::IsADirectory]
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
[427.606453 0:528 axfs::root:433] [AxError::IsADirectory]
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
[430.961959 0:533 axfs::root:433] [AxError::IsADirectory]
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
[441.016600 0:560 axfs::root:433] [AxError::IsADirectory]
[441.018887 0:560 axfs::root:433] [AxError::IsADirectory]
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
[444.419783 0:562 axfs::root:433] [AxError::IsADirectory]
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
[458.990595 0:583 axfs::root:433] [AxError::IsADirectory]
[458.992658 0:583 axfs::fops:297] [AxError::NotADirectory]
[458.994255 0:583 axfs::root:433] [AxError::IsADirectory]
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
[463.240627 0:591 axfs::root:433] [AxError::IsADirectory]
[463.242068 0:591 axfs::fops:297] [AxError::NotADirectory]
[463.243739 0:591 axfs::root:433] [AxError::IsADirectory]
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
[467.009053 0:596 axfs::root:433] [AxError::IsADirectory]
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
[471.984280 0:601 axfs::root:433] [AxError::IsADirectory]
[471.985477 0:601 axfs::root:433] [AxError::IsADirectory]
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
[475.332006 0:603 axfs::root:433] [AxError::IsADirectory]
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
[478.744809 0:608 axfs::fops:297] [AxError::NotADirectory]
[478.746450 0:608 axfs::root:433] [AxError::IsADirectory]
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
[482.140105 0:613 axfs::fops:297] [AxError::NotADirectory]
[482.141770 0:613 axfs::root:433] [AxError::IsADirectory]
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
[485.530577 0:618 axfs::fops:297] [AxError::NotADirectory]
[485.531476 0:618 axfs::fops:297] [AxError::NotADirectory]
[485.532939 0:618 axfs::root:433] [AxError::IsADirectory]
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
[488.979081 0:623 axfs::fops:297] [AxError::NotADirectory]
[488.979976 0:623 axfs::fops:297] [AxError::NotADirectory]
[488.981410 0:623 axfs::root:433] [AxError::IsADirectory]
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
[492.423044 0:628 axfs::fops:297] [AxError::NotADirectory]
[492.424692 0:628 axfs::root:433] [AxError::IsADirectory]
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
[495.898582 0:633 axfs::fops:297] [AxError::NotADirectory]
[495.900207 0:633 axfs::root:433] [AxError::IsADirectory]
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
[500.495966 0:638 axfs::root:433] [AxError::IsADirectory]
[500.497070 0:638 axfs::root:433] [AxError::IsADirectory]
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
[505.063307 0:640 axfs::root:433] [AxError::IsADirectory]
[505.064725 0:640 axfs::root:433] [AxError::IsADirectory]
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
[508.476265 0:642 axfs::fops:297] [AxError::NotADirectory]
[508.477887 0:642 axfs::root:433] [AxError::IsADirectory]
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
[511.898526 0:647 axfs::fops:297] [AxError::NotADirectory]
[511.900105 0:647 axfs::root:433] [AxError::IsADirectory]
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
[515.247142 0:652 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE chroot01 : 0
RUN LTP CASE chroot02
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
chroot02.c:30: TFAIL: chroot(/tmp/LTP_chrdCkGAa) failed: ENOSYS (38)
tst_test.c:1449: TBROK: Test haven't reported results!

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[518.669249 0:657 axfs::fops:297] [AxError::NotADirectory]
[518.670927 0:657 axfs::root:433] [AxError::IsADirectory]
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
[522.067220 0:663 axfs::fops:297] [AxError::NotADirectory]
[522.069234 0:663 axfs::root:433] [AxError::IsADirectory]
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
[525.412042 0:668 axfs::root:433] [AxError::IsADirectory]
[525.413337 0:668 axfs::root:433] [AxError::IsADirectory]
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
[529.705746 0:675 axfs::root:433] [AxError::IsADirectory]
tst_clocks.c:107: TCONF: clock_settime() not available
tst_wallclock.c:64: TBROK: tst_clock_settime() realtime failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   1
skipped  1
warnings 0
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_cloeaDKMd) failed: unlink(/tmp/LTP_cloeaDKMd) failed; errno=2: ENOENT
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
[533.268595 0:680 axfs::root:433] [AxError::IsADirectory]
tst_clocks.c:107: TCONF: clock_settime() not available
tst_wallclock.c:64: TBROK: tst_clock_settime() realtime failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   1
skipped  1
warnings 0
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_clonnDeMG) failed: unlink(/tmp/LTP_clonnDeMG) failed; errno=2: ENOENT
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
[537.059484 0:685 axfs::root:433] [AxError::IsADirectory]
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
[540.672849 0:699 axfs::root:433] [AxError::IsADirectory]
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
[544.173026 0:707 axfs::root:433] [AxError::IsADirectory]
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
[551.748484 0:714 axfs::root:433] [AxError::IsADirectory]
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
[615.960281 0:720 axfs::root:433] [AxError::IsADirectory]
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
tst_timer_test.c:305: TINFO: min 1022us, max 1330us, median 1046us, trunc mean 1047.67us (discarded 25)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 2000us 500 iterations, threshold 402.01us
tst_timer_test.c:305: TINFO: min 2023us, max 3918us, median 2046us, trunc mean 2049.82us (discarded 25)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 5000us 300 iterations, threshold 405.04us
tst_timer_test.c:305: TINFO: min 5029us, max 5164us, median 5075us, trunc mean 5070.99us (discarded 15)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 10000us 100 iterations, threshold 410.33us
tst_timer_test.c:305: TINFO: min 10075us, max 10788us, median 10112us, trunc mean 10112.28us (discarded 5)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 25000us 50 iterations, threshold 426.29us
tst_timer_test.c:305: TINFO: min 25111us, max 25237us, median 25163us, trunc mean 25162.79us (discarded 2)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 100000us 10 iterations, threshold 537.00us
tst_timer_test.c:305: TINFO: min 100132us, max 100200us, median 100159us, trunc mean 100161.33us (discarded 1)
tst_timer_test.c:326: TPASS: Measured times are within thresholds
tst_timer_test.c:263: TINFO: clock_nanosleep() sleeping for 1000000us 2 iterations, threshold 4400.00us
tst_timer_test.c:305: TINFO: min 1000155us, max 1000168us, median 1000155us, trunc mean 1000155.00us (discarded 1)
tst_timer_test.c:326: TPASS: Measured times are within thresholds

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[627.752398 0:731 axfs::root:433] [AxError::IsADirectory]
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
[634.543915 0:739 axfs::root:433] [AxError::IsADirectory]
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
[638.017271 0:747 axfs::root:433] [AxError::IsADirectory]
tst_clocks.c:107: TCONF: clock_settime() not available
tst_wallclock.c:64: TBROK: tst_clock_settime() realtime failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   1
skipped  1
warnings 0
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_cloiIKhKk) failed: unlink(/tmp/LTP_cloiIKhKk) failed; errno=2: ENOENT
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
[642.352248 0:755 axfs::root:433] [AxError::IsADirectory]
tst_clocks.c:107: TCONF: clock_settime() not available
tst_wallclock.c:64: TBROK: tst_clock_settime() realtime failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   1
skipped  1
warnings 0
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_clonNBBCJ) failed: unlink(/tmp/LTP_clonNBBCJ) failed; errno=2: ENOENT
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
[645.887797 0:760 axfs::root:433] [AxError::IsADirectory]
tst_clocks.c:107: TCONF: clock_settime() not available
tst_wallclock.c:64: TBROK: tst_clock_settime() realtime failed: ENOSYS (38)

Summary:
passed   0
failed   0
broken   1
skipped  1
warnings 0
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_cloNlkhAj) failed: unlink(/tmp/LTP_cloNlkhAj) failed; errno=2: ENOENT
FAIL LTP CASE clock_settime03 : 34
RUN LTP CASE clone01
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clone01.c:37: TPASS: clone returned 769
clone01.c:43: TPASS: Child exited with 0

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[649.339086 0:765 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clone01 : 0
RUN LTP CASE clone02
clone02     1  TFAIL  :  clone02.c:139: clone() failed: TEST_ERRNO=ENOSYS(38): Function not implemented
clone02     2  TPASS  :  Test Passed
[653.103454 0:771 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE clone02 : 1
RUN LTP CASE clone03
tst_buffers.c:57: TINFO: Test is using guarded buffers
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
tst_test.c:1733: TINFO: LTP version: 20240524
tst_test.c:1617: TINFO: Timeout per run is 0h 00m 30s
tst_memutils.c:152: TINFO: oom_score_adj does not exist, skipping the adjustment
clone03.c:38: TFAIL: pid(0) retval 778 != 0: SUCCESS (0)

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[656.438585 0:774 axfs::root:433] [AxError::IsADirectory]
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
[660.075164 0:780 axfs::root:433] [AxError::IsADirectory]
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
[663.549102 0:785 axfs::root:433] [AxError::IsADirectory]
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
[666.992944 0:791 axfs::root:433] [AxError::IsADirectory]
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
[670.398975 0:797 axfs::root:433] [AxError::IsADirectory]
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
[674.022131 0:803 axfs::root:433] [AxError::IsADirectory]
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
[677.339902 0:809 axfs::root:433] [AxError::IsADirectory]
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
[680.705158 0:814 axfs::fops:297] [AxError::NotADirectory]
[680.708130 0:814 axfs::root:433] [AxError::IsADirectory]
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
[684.114667 0:819 axfs::root:433] [AxError::IsADirectory]
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
[687.395347 0:824 axfs::fops:297] [AxError::NotADirectory]
[687.397041 0:824 axfs::root:433] [AxError::IsADirectory]
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
[690.849153 0:826 axfs::fops:297] [AxError::NotADirectory]
[690.851842 0:826 axfs::root:433] [AxError::IsADirectory]
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
[694.270611 0:831 axfs::root:433] [AxError::IsADirectory]
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
[698.837537 0:836 axfs::root:433] [AxError::IsADirectory]
[698.838676 0:836 axfs::root:433] [AxError::IsADirectory]
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
[702.220938 0:838 axfs::root:433] [AxError::IsADirectory]
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
[707.068106 0:846 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE confstr01 : 0
RUN LTP CASE connect01
connect01    1  TPASS  :  bad file descriptor successful
connect01    2  TPASS  :  invalid socket buffer successful
[710.692358 0:851 arceos_posix_api::imp::net:352] sys_connect => Err(EINVAL)
connect01    3  TPASS  :  invalid salen successful
connect01    4  TPASS  :  invalid socket successful
[710.697673 0:851 axnet::smoltcp_impl::tcp:197] [AxError::ConnectionRefused] socket connect() failed
[710.698609 0:851 arceos_posix_api::imp::net:352] sys_connect => Err(ECONNREFUSED)
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
[715.231786 0:854 axfs::root:433] [AxError::IsADirectory]
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
[720.246551 0:859 axfs::root:433] [AxError::IsADirectory]
[720.247662 0:859 axfs::root:433] [AxError::IsADirectory]
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
[724.754762 0:861 axfs::root:433] [AxError::IsADirectory]
[724.756986 0:861 axfs::root:433] [AxError::IsADirectory]
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
[730.168005 0:863 axfs::fops:297] [AxError::NotADirectory]
[730.168879 0:863 axfs::fops:297] [AxError::NotADirectory]
[730.170242 0:863 axfs::root:433] [AxError::IsADirectory]
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
crash01     0  TINFO  :  crashme +2000.80 812 100
crash01     1  TPASS  :  we're still here, OS seems to be robust
exit status ... number of cases
[812.410924 0:918 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE crash01 : 0
RUN LTP CASE crash02
crash02     0  TINFO  :  crashme02 127 820 100
0000: syscall(64, 0, 0xae290b56, 0, 0x8f0cc945, 0x6d4761b0, 0x92, 0x261862e3)
0001: syscall(103, 0, 0, 0, 0, 0x48fff8e7, 0xe5ae2e7a, 0)
0002: syscall(42, 0, 0x83276b52, 0, 0xa2deff7f, 0, 0x8ef3, 0x953085ca)
0003: syscall(124, 0xc6b3c990, 0x70abbf40, 0, 0x743150bb, 0x6f0, 0, 0x4dbcf117)
0004: syscall(46, 0x2d27240b, 0x61b32343, 0x66cca9a3, 0x4310a347, 0, 0x18b75975, 0x209e12e6)
0005: syscall(112, 0x61bdae1b, 0xb9b27713, 0xd7c3d477, 0x27dcc8c3, 0, 0x94eb7cb4, 0)
0006: syscall(104, 0x10858ff1, 0, 0, 0, 0xc6, 0xe905ea67, 0)
0007: syscall(45, 0x4af613e8, 0x132116f1, 0xbcdb, 0x30d1ca01, 0xeeda9f4f, 0x39bb, 0xe992783c)
0008: syscall(56, 0xc2261084, 0xc1a2e06c, 0x8eb0c45f, 0xf1e453c2, 0xbd, 0x414cb2aa, 0xa0bb4507)
0009: syscall(102, 0, 0x56d4a08f, 0, 0x6b05b6bb, 0xb0ea58f5, 0, 0xa18853a0)
0010: syscall(75, 0x439e1045, 0x9fcad1e1, 0xb5c8af9b, 0, 0xdfe9d997, 0, 0x667b4c2d)
0011: syscall(49, 0, 0x50, 0, 0x60, 0, 0, 0x7e546964)
0012: syscall(108, 0xf92dca3f, 0, 0x8e27cd8d, 0x2ecf, 0x134cd00f, 0, 0x417e)
0013: syscall(36, 0xf7cc37eb, 0x50ea784b, 0, 0, 0xca1bd229, 0xaeaa8b14, 0xf7c4e4fd)
0014: syscall(61, 0, 0, 0, 0, 0x38371201, 0, 0)
0015: syscall(3, 0x7770dde6, 0xa9a57938, 0xf33e410e, 0, 0xf3f12131, 0x7488f5c7, 0x25)
0016: syscall(114, 0x6b43, 0x56, 0x9aa9, 0, 0x539f6aff, 0xa5a3e648, 0x4709837e)
0017: syscall(68, 0, 0x125009a9, 0x1161, 0, 0, 0, 0)
0018: syscall(99, 0x6702, 0x648c4f3, 0xd9bd840b, 0x69, 0x88d077be, 0, 0)
0019: syscall(5, 0x68127e09, 0x979352c5, 0, 0xa03ad6cc, 0, 0x12126acd, 0x6cbacc37)
0020: syscall(3, 0x4de4f3ad, 0xf8, 0xf99763b9, 0, 0x297e9666, 0x4b, 0xa8f598f9)
0021: syscall(74, 0x257e86b5, 0, 0xdb3d, 0, 0, 0xb9aed020, 0x3e7b31d4)
0022: syscall(84, 0x2d458400, 0xc1ee9aff, 0x7ed9cd56, 0x3c, 0, 0x79ae6a52, 0)
0023: syscall(12, 0, 0x71bd, 0, 0xbe, 0, 0, 0)
0024: syscall(26, 0x2278476a, 0xfb2926fb, 0, 0x643bb8b6, 0xc36148d6, 0x9f26ee93, 0x11)
0025: syscall(8, 0, 0, 0xc07dbbbc, 0xc6df00ee, 0xc9, 0xdd45, 0x31d74203)
0026: syscall(98, 0, 0x8bfc01c7, 0xe87214d4, 0, 0xd23072db, 0x7c, 0x71b55d11)
0027: syscall(95, 0x58, 0xe3f8, 0x55, 0, 0x74a558c0, 0, 0)
0028: syscall(41, 0, 0xe5b5b4a, 0, 0, 0x7969a1ac, 0, 0x22)
0029: syscall(118, 0, 0, 0x37e2514e, 0xfb176d75, 0, 0x96775ac2, 0xf3)
0030: syscall(9, 0x1a, 0x48, 0x421f6040, 0x37913f47, 0x8c0d8798, 0x4c46c1d0, 0xafcede6c)
0031: syscall(83, 0, 0xf1936a1c, 0, 0, 0, 0xf7284e7a, 0)
0032: syscall(48, 0xb2b23ab5, 0, 0x5ce17222, 0x4bf6aa96, 0xbfd5395e, 0xf1a73b00, 0xff1557e4)
0033: syscall(23, 0x7482172c, 0, 0x58a33a3c, 0, 0, 0, 0x1147f1e6)
0034: syscall(74, 0xac95bcb7, 0xd0d98d1f, 0, 0, 0xf478d89, 0x2cbe2654, 0xa7c50b37)
0035: syscall(56, 0xa52606, 0xfb0ba7a8, 0x216e59c3, 0, 0xefd18bf3, 0, 0x22a2be20)
0036: syscall(48, 0, 0xe9652b31, 0xf760e41b, 0, 0x80741f7a, 0, 0)
0037: syscall(61, 0, 0xd090dfbb, 0, 0xbb63, 0x43fd, 0, 0xe7e)
0038: syscall(10, 0, 0x5fdc3a8a, 0, 0x4e, 0x5c5f7254, 0x33b55970, 0xd7909e90)
0039: syscall(112, 0xff1c, 0x66f7677c, 0, 0x9d3db07d, 0x8429, 0, 0x34a0f44b)
0040: syscall(7, 0xe12a536, 0, 0x56a17ac7, 0x22, 0x4c5a5bca, 0, 0)
0041: syscall(39, 0x339ff3df, 0, 0x2a2b, 0, 0x98acac9b, 0xa1b444ac, 0x4d8abeb)
0042: syscall(98, 0x2221bc32, 0xfd, 0xcfa3bd47, 0, 0xb55b6ab4, 0, 0)
0043: syscall(56, 0x21582960, 0x22d1ed6c, 0x9249, 0x63ad, 0, 0, 0x40ec590e)
0044: syscall(38, 0x96c3596d, 0x123a0794, 0, 0xef36f457, 0x15ffaef9, 0, 0)
0045: syscall(115, 0, 0, 0, 0xe5c35771, 0xc225c722, 0x61eee830, 0)
0046: syscall(108, 0, 0xec, 0, 0x16df1515, 0xd025012a, 0xbab1cd0b, 0xaf721dd0)
0047: syscall(96, 0xaf63cbd2, 0x2624f127, 0, 0, 0x16, 0x3a17ad47, 0)
0048: syscall(39, 0xe77079b, 0xd5770e35, 0, 0xee, 0x6fc8a57d, 0xa97faf68, 0x730efda1)
0049: syscall(94, 0x13, 0, 0, 0x2, 0xddb4270e, 0x7472d8c6, 0x5e5e4b56)
crash02     1  TPASS  :  we're still here, OS seems to be robust
[821.158982 0:923 axfs::root:433] [AxError::IsADirectory]
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
[830.270201 0:977 axfs::fops:297] [AxError::NotADirectory]
[830.276855 0:977 axfs::root:433] [AxError::IsADirectory]
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
[836.302723 0:982 axfs::fops:297] [AxError::NotADirectory]
[836.307460 0:982 axfs::root:433] [AxError::IsADirectory]
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
[842.247068 0:987 axfs::fops:297] [AxError::NotADirectory]
[842.253222 0:987 axfs::root:433] [AxError::IsADirectory]
[842.257764 0:987 axfs::root:433] [AxError::IsADirectory]
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
[848.016579 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.021848 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.024278 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.027956 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.030915 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.031973 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.032564 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.033133 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.035845 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.036486 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.037063 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.040739 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.043010 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.045896 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.049680 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.052399 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.055518 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.056107 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.059830 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.060405 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.060943 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.061898 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.064898 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.066586 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.068877 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.070988 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.071557 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.072079 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.075225 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.076791 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.078951 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.079717 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.081012 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.081733 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.084490 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.085198 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.088103 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.088775 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.091395 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.092020 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.092687 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.095691 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.096472 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.097167 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.101653 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.103473 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.105329 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.106067 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.107980 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.108695 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.111434 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.112052 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.114835 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.115505 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.116183 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.119071 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.119766 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.121452 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.122170 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.122830 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.125588 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.126202 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.129633 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.130344 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.130933 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.133610 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.134303 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.134900 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.137564 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.138431 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.139131 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.141042 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.144533 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.145220 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.147056 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.149837 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.150636 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.151276 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.153141 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.153833 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.156624 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.157250 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.157862 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.160546 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.163408 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.164119 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.164758 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.167353 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.168216 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.168914 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.169664 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.170239 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.172978 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.173665 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.176425 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.179490 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.180086 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.180715 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.181585 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.183926 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.184525 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.185205 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.186047 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.188836 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.189513 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.190077 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.192702 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.195453 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.196186 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.198494 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.199157 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.199781 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.201428 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.202071 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.202651 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.203202 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.207075 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.207760 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.208443 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.209199 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.213926 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.214522 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.215174 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.218081 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.218731 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.221267 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.222097 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.222769 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.224698 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.225434 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.226103 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.229491 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.230176 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.230820 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.233463 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.234174 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.234887 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.237591 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.238224 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.238935 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.241675 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.242215 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.242957 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.245124 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.245808 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.249492 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.250166 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.250807 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.253471 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.254155 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.254806 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.256617 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.257313 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.257995 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.259685 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.262578 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.263275 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.264153 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.267544 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.268268 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.268982 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.271746 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.272435 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.273058 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.275693 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.276461 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.277095 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.277747 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.280454 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.281156 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.281837 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.284244 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.284845 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.288004 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.288766 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.291426 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.292101 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.292772 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.294463 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.295760 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.297404 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.299716 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.300233 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.300773 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.301362 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.302944 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.304466 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.305976 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.307442 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.308886 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.309715 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.310281 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.311907 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.313359 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.314235 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.315696 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.316208 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.317845 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.318862 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.319428 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.320006 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.321337 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.322813 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.324708 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.325429 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.325964 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.326921 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.328557 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.330051 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.330811 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.331386 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.332936 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.334197 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.335534 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.336106 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.337745 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.339179 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.339765 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.340451 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.342025 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.343529 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.344251 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.344905 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.345482 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.347179 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.348651 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.350063 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.351090 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.352504 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.353449 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.353987 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.354810 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.356484 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.357879 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.358503 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.360018 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.360646 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.361154 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.362563 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.364355 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.365591 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.366134 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.367617 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.369078 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.369710 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.370376 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.371921 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.373417 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.374129 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.374689 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.375226 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.377233 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.378947 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.380223 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.380881 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.381576 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.383209 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.384709 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.385424 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.385974 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.386892 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.388380 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.389914 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.391457 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.392545 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.393281 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.393877 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.394519 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.396017 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.397689 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.398808 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.399427 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.400954 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.402125 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.403682 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.404385 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.404900 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.405503 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.407455 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.408927 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.409833 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.410474 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.412050 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.413036 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.414073 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.415302 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.415927 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.416730 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.418688 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.420334 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.421074 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.422181 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.423374 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.423905 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.424498 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.426047 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.427827 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.428491 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.429701 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.430253 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.431810 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.432882 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.433619 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.435276 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.435965 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.436640 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.437211 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.437813 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.438445 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.438959 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.439499 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.440013 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.440575 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.441074 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.441616 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.442154 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.442719 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.443260 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.443901 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.444449 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.444984 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.445547 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.446089 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.446821 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.447937 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.448591 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.449115 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.449656 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.450169 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.450756 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.451350 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.451921 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.452486 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.453006 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.453723 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.454359 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.454910 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.455465 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.455986 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.456651 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.457211 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.457794 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.458413 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.458952 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.459494 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.460014 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.460580 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.461119 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.461967 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.462827 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.463464 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.463994 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.464560 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.465133 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.465743 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.466353 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.467047 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.467714 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.468256 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.468838 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.469404 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.469918 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.470444 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.470945 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.471467 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.471969 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.472676 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.473220 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.474064 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.474864 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.475458 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.475984 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.476672 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.477979 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.478555 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.479085 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.479631 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.480133 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.480740 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.481369 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.481928 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.482494 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.483083 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.483645 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.484153 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.484761 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.485318 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.485861 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.486719 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.487256 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.487787 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.488317 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.488874 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.489797 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.490386 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.490910 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.491452 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.492008 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.492680 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.493280 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.493922 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.494490 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.495006 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.495559 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.496076 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.496731 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.497258 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.497801 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.498336 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.498894 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.499525 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.500079 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.500649 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.501172 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.501793 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.502331 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.503154 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.504097 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.504656 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.505164 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.505710 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.506221 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.506914 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.507581 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.508206 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.508842 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.509409 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.510006 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.510578 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.511127 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.511795 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.512348 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.512897 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.513452 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.514039 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.514645 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.515199 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.515760 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.516756 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.517267 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.517828 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.518344 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.519235 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.519854 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.520507 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.521031 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.521663 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.522199 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.522904 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.523509 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.524058 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.524618 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.525126 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.525673 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.526179 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.526868 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.527443 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.527972 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.528535 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.529128 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.529684 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.530192 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.531135 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.532017 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.532632 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.533167 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.533716 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.534225 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.534793 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.535405 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.536004 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.536662 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.537204 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.537757 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.538278 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.538855 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.539420 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.539966 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.540604 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.541124 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.541710 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.542233 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.542792 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.543803 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.544470 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.544991 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.545545 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.546045 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.546724 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.547328 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.547913 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.548545 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.549071 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.549624 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.550215 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.550790 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.551349 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.551896 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.552464 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.552994 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.553529 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.554107 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.554661 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.555217 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.555767 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.556731 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.557264 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.557809 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.558348 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.559145 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.559862 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.560458 0:993 axfs::fops:297] [AxError::NotADirectory]
[848.562186 0:993 axfs::root:433] [AxError::IsADirectory]
tst_tmpdir.c:342: TWARN: tst_rmdir: rmobj(/tmp/LTP_creaCLfMg) failed: remove(/tmp/LTP_creaCLfMg) failed; errno=39: ENOTEMPTY
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
[864.197854 0:998 axfs::root:433] [AxError::IsADirectory]
[864.201523 0:998 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE creat06 : 6
RUN LTP CASE creat07
tst_test.c:949: TBROK: Failed to copy resource 'creat07_child'

Summary:
passed   0
failed   0
broken   1
skipped  0
warnings 0
[869.657952 0:1000 axfs::fops:297] [AxError::NotADirectory]
[869.663373 0:1000 axfs::root:433] [AxError::IsADirectory]
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
[880.553786 0:1004 axfs::root:433] [AxError::IsADirectory]
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
[888.479323 0:1009 axfs::root:433] [AxError::IsADirectory]
[888.482995 0:1009 axfs::root:433] [AxError::IsADirectory]
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
[897.579843 0:1015 axfs::root:433] [AxError::IsADirectory]
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
[903.735591 0:1020 axfs::root:433] [AxError::IsADirectory]
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
[909.835763 0:1025 axfs::root:433] [AxError::IsADirectory]
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
[920.754387 0:1032 axfs::root:433] [AxError::IsADirectory]
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
[926.256916 0:1037 axfs::root:433] [AxError::IsADirectory]
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
[932.668241 0:1042 axfs::root:433] [AxError::IsADirectory]
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
[1027.438441 0:1066 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1031.922655 0:1061 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1036.304306 0:1059 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1040.747583 0:1058 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1045.400172 0:1054 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1050.184147 0:1056 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1055.429513 0:1053 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
cve-2017-17052.c:106: TFAIL: child exited with 2
[1055.760965 0:5123 axmm:27] Mapping error: BadState
cve-2017-17052.c:67: TBROK: fork() failed: EFAULT (14)
[1055.846450 0:1055 axmm:27] Mapping error: BadState
cve-2017-17052.c:71: TBROK: fork() failed: EFAULT (14)
[1055.918621 0:5125 axmm:27] Mapping error: BadState
cve-2017-17052.c:67: TBROK: fork() failed: EFAULT (14)
cve-2017-17052.c:106: TFAIL: child exited with 2
[1056.135594 0:5128 axmm:27] Mapping error: BadState
cve-2017-17052.c:66: TBROK: fork() failed: EFAULT (14)
cve-2017-17052.c:106: TFAIL: child exited with 2
[1056.618742 0:5129 axmm:27] Mapping error: BadState
cve-2017-17052.c:66: TBROK: fork() failed: EFAULT (14)
cve-2017-17052.c:106: TFAIL: child exited with 2
cve-2017-17052.c:113: TPASS: kernel survived 4 runs

Summary:
passed   0
failed   0
broken   0
skipped  0
warnings 0
[1057.116428 0:1049 axfs::root:433] [AxError::IsADirectory]
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
[1068.883488 0:5133 axfs::root:433] [AxError::IsADirectory]
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
[1076.909104 0:5138 axfs::root:433] [AxError::IsADirectory]
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
[1094.647208 0:5148 axmm:27] Mapping error: BadState
data_space    1  TBROK  :  data_space.c:159: fork failed: errno=EFAULT(14): Bad address
data_space    2  TBROK  :  data_space.c:159: Remaining cases broken
[1094.674911 0:5148 axfs::root:433] [AxError::IsADirectory]
FAIL LTP CASE data_space : 2
RUN LTP CASE dccp01.sh
[1094.901638 0:5152 axmm:27] Mapping error: BadState
FAIL LTP CASE dccp01.sh : 2
RUN LTP CASE dccp_ipsec.sh
[1095.074223 0:5153 axmm:27] Mapping error: BadState
FAIL LTP CASE dccp_ipsec.sh : 2
RUN LTP CASE dccp_ipsec_vti.sh
[1095.263844 0:5154 axmm:27] Mapping error: BadState
FAIL LTP CASE dccp_ipsec_vti.sh : 2
RUN LTP CASE dctcp01.sh
[1095.446104 0:5155 axmm::aspace:112] [AxError::BadState] failed to materialize child page
FAIL LTP CASE dctcp01.sh : 2
RUN LTP CASE delete_module01
[1095.629470 0:5156 page_table_multiarch::bits64:490] failed to map page: 0x1200000(Size4K) -> PA:0x0, NoMemory
[1095.630966 0:5156 axmm:27] Mapping error: BadState
FAIL LTP CASE delete_module01 : 2
RUN LTP CASE delete_module02
[1095.809776 0:5157 axmm:27] Mapping error: BadState
FAIL LTP CASE delete_module02 : 2
RUN LTP CASE delete_module03
[1095.969126 0:5158 axmm:27] Mapping error: BadState
FAIL LTP CASE delete_module03 : 2
RUN LTP CASE df01.sh
[1096.137589 0:5159 axmm:27] Mapping error: BadState
FAIL LTP CASE df01.sh : 2
RUN LTP CASE dhcp_lib.sh
SKIP LTP CASE dhcp_lib.sh : LTP shell helper library is not a standalone test
FAIL LTP CASE dhcp_lib.sh : 32
RUN LTP CASE dhcpd_tests.sh
[1096.673820 0:5161 axmm:27] Mapping error: BadState
FAIL LTP CASE dhcpd_tests.sh : 2
RUN LTP CASE dio_append
[1097.020030 0:5162 axmm:27] Mapping error: BadState
FAIL LTP CASE dio_append : 2
RUN LTP CASE dio_read
[1097.337776 0:5163 axmm:27] Mapping error: BadState
FAIL LTP CASE dio_read : 2
RUN LTP CASE dio_sparse
[1097.513558 0:5164 axmm:27] Mapping error: BadState
FAIL LTP CASE dio_sparse : 2
RUN LTP CASE dio_truncate
[1097.654977 0:172 axmm:27] Mapping error: BadState
/tmp/testsuite/musl-ltp-script/ltp_testcode.sh: line 15: can't fork: Bad address
#### OS COMP TEST GROUP START lua-musl ####
[1101.466528 0:5165 axmm:27] Mapping error: BadState
./lua_testcode.sh: line 3: can't fork: Bad address
[1102.234888 0:5168 axmm:27] Mapping error: BadState
sh: can't fork: Bad address
#### OS COMP TEST GROUP START unixbench-musl ####
SKIP: unixbench currently blocks on unresolved executable/runtime compatibility
#### OS COMP TEST GROUP END unixbench-musl ####
[1103.701263 0:5169 axmm:27] Mapping error: BadState
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
Thu Jan  1 00:18:35 UTC 1970
testcase busybox date success
Filesystem           1K-blocks      Used Available Use% Mounted on
devfs                  1045248   1037648      7600  99% /dev
tmpfs                  1045248   1037648      7600  99% /tmp
tmpfs                  1045248   1037648      7600  99% /var
proc                   1045248   1037648      7600  99% /proc
sysfs                  1045248   1037648      7600  99% /sys
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
 00:18:52 up 0 min,  0 users,  load average: 0.00, 0.00, 0.00
testcase busybox uptime success
abc
testcase busybox printf "abc\n" success
PID   USER     TIME  COMMAND
testcase busybox ps success
/tmp/testsuite/glibc/busybox
testcase busybox pwd success
              total        used        free      shared  buff/cache   available
Mem:              0           0           0           0           0     1039833
-/+ buffers/cache:            0           0
Swap:             0           0           0
testcase busybox free success
Thu Jan  1 00:19:00 1970  0.000000 seconds
testcase busybox hwclock success
[1141.144021 0:5192 axmm:27] Mapping error: BadState
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
[1164.836378 0:5211 axmm:27] Mapping error: BadState
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
[1181.286934 0:2 axmm:27] Mapping error: BadState
autorun: /tmp/testsuite/glibc/cyclictest/cyclictest_testcode.sh failed: failed to map user stack: Bad internal state
[1181.939656 0:2 axmm:27] Mapping error: BadState
autorun: /tmp/testsuite/glibc/iozone/iozone_testcode.sh failed: failed to map user stack: Bad internal state
[1182.852885 0:2 axmm:27] Mapping error: BadState
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
[1183.396200 0:2 axmm:27] Mapping error: BadState
autorun: /glibc/ltp_testcode.sh failed: failed to map user stack: Bad internal state
[1184.275853 0:2 axmm:27] Mapping error: BadState
autorun: /tmp/testsuite/glibc/lua/lua_testcode.sh failed: failed to map user stack: Bad internal state
[1184.926360 0:2 axmm:27] Mapping error: BadState
autorun: /tmp/testsuite/glibc/netperf/netperf_testcode.sh failed: failed to map user stack: Bad internal state
#### OS COMP TEST GROUP START unixbench-glibc ####
SKIP: unixbench currently blocks on unresolved executable/runtime compatibility
#### OS COMP TEST GROUP END unixbench-glibc ####
[1184.952798 0:2 axplat_riscv64_qemu_virt::power:28] Shutting down...
```
