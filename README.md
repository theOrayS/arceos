# ArceOS

[![CI](https://github.com/arceos-org/arceos/actions/workflows/build.yml/badge.svg?branch=main)](https://github.com/arceos-org/arceos/actions/workflows/build.yml)
[![CI](https://github.com/arceos-org/arceos/actions/workflows/test.yml/badge.svg?branch=main)](https://github.com/arceos-org/arceos/actions/workflows/test.yml)
[![Docs](https://img.shields.io/badge/docs-pages-green)](https://arceos-org.github.io/arceos/)

An experimental modular operating system (or unikernel) written in Rust.

ArceOS was inspired a lot by [Unikraft](https://github.com/unikraft/unikraft).

🚧 Working In Progress.

## Features & TODOs

* [x] Architecture: x86_64, riscv64, aarch64, loongarch64
* [x] Platform: QEMU pc-q35 (x86_64), virt (riscv64/aarch64/loongarch64)
* [x] Multi-thread
* [x] FIFO/RR/CFS scheduler
* [x] VirtIO net/blk/gpu drivers
* [x] TCP/UDP net stack using [smoltcp](https://github.com/smoltcp-rs/smoltcp)
* [x] Synchronization/Mutex
* [x] SMP scheduling with [per-cpu run queue](https://github.com/arceos-org/arceos/discussions/181)
* [x] File system
* [ ] Compatible with Linux apps
* [ ] Interrupt driven device I/O
* [ ] Async I/O

## Quick Start

### Build and Run through Docker

Install [Docker](https://www.docker.com/) in your system. The provided image
contains the Rust toolchain selected by `rust-toolchain.toml`, the ArceOS cargo
helpers, the QEMU targets used by this repository, and the musl cross toolchains
used when C user apps need to be rebuilt.

Build the image with the provided Dockerfile:

```bash
docker build -t arceos -f Dockerfile .
```

Create a container and build/run apps:

```bash
docker run --rm -it -v $(pwd):/arceos -w /arceos arceos bash

# Now build/run app in the container
make A=examples/helloworld ARCH=aarch64 run
```

If evaluator images and `run-eval.sh` are provided in the repository directory,
they can also be used inside the container. For local validation that must avoid
Docker, use the direct-server instructions below.

### Manually Build and Run

#### 1. Install Build Dependencies

Install the host packages needed to build and run ArceOS directly on
Debian/Ubuntu:

```bash
sudo apt-get update
sudo apt-get install -y build-essential make git wget ca-certificates \
    python3 python3-venv pkg-config libclang-dev qemu-system
```

The repository pins the Rust toolchain in `rust-toolchain.toml`
(`nightly-2025-05-20` with `rust-src`, `llvm-tools`, `rustfmt`, and `clippy`).
With `rustup` installed, entering the repository or invoking `cargo`/`make`
will use that pinned toolchain and its configured targets.

Install [cargo-binutils](https://github.com/rust-embedded/cargo-binutils) to use
`rust-objcopy` and `rust-objdump` tools, [axconfig-gen](https://github.com/arceos-org/axconfig-gen)
for kernel configuration, and [cargo-axplat](https://github.com/arceos-org/axplat_crates/tree/dev/cargo-axplat)
for platform configuration:

```bash
cargo install cargo-binutils axconfig-gen cargo-axplat
```

##### Dependencies for running apps

```bash
# for Debian/Ubuntu
sudo apt-get install qemu-system
# for macos
brew install qemu
```

The supplied RV/LA evaluator path has been validated on a direct server with
QEMU available as `qemu-system-riscv64` and `qemu-system-loongarch64`. When the
two evaluator images and `run-eval.sh` are present in the repository directory,
run:

```bash
./run-eval.sh rv
./run-eval.sh la
```

##### Dependencies for building C apps (optional)

Install `libclang-dev` if it was not installed with the host packages above:

```bash
sudo apt install libclang-dev
```

Download & install [musl](https://musl.cc) toolchains:

```bash
# download
wget https://musl.cc/aarch64-linux-musl-cross.tgz
wget https://musl.cc/riscv64-linux-musl-cross.tgz
wget https://musl.cc/x86_64-linux-musl-cross.tgz
wget https://github.com/LoongsonLab/oscomp-toolchains-for-oskernel/releases/download/loongarch64-linux-musl-cross-gcc-13.2.0/loongarch64-linux-musl-cross.tgz
# install
tar zxf aarch64-linux-musl-cross.tgz
tar zxf riscv64-linux-musl-cross.tgz
tar zxf x86_64-linux-musl-cross.tgz
tar zxf loongarch64-linux-musl-cross.tgz
# exec below command in bash OR add below info in ~/.bashrc
export PATH=`pwd`/x86_64-linux-musl-cross/bin:`pwd`/aarch64-linux-musl-cross/bin:`pwd`/riscv64-linux-musl-cross/bin:`pwd`/loongarch64-linux-musl-cross/bin:$PATH
```

Other systems and arch please refer to [Qemu Download](https://www.qemu.org/download/#linux)

#### 2. Build & Run

```bash
# build app in arceos directory
make A=path/to/app ARCH=<arch> LOG=<log>
```

Where `path/to/app` is the relative path to the application. Examples applications can be found in the [examples](examples/) directory or the [arceos-apps](https://github.com/arceos-org/arceos-apps) repository.

`<arch>` should be one of `riscv64`, `aarch64`, `x86_64`, `loongarch64`.

`<log>` should be one of `off`, `error`, `warn`, `info`, `debug`, `trace`.

More arguments and targets can be found in [Makefile](Makefile).

For example, to run the [httpserver](examples/httpserver/) on `qemu-system-aarch64` with 4 cores and log level `info`:

```bash
make A=examples/httpserver ARCH=aarch64 LOG=info SMP=4 run NET=y
```

Note that the `NET=y` argument is required to enable the network device in QEMU. These arguments (`BLK`, `GRAPHIC`, etc.) only take effect at runtime not build time.

## How to write ArceOS apps

You can write and build your custom applications outside the ArceOS source tree.
Examples are given below and in the [app-helloworld](https://github.com/arceos-org/app-helloworld) and [arceos-apps](https://github.com/arceos-org/arceos-apps) repositories.

### Rust

1. Create a new rust package with `no_std` and `no_main` environment.
2. Add `axstd` dependency and features to enable to `Cargo.toml`:

    ```toml
    [dependencies]
    axstd = { path = "/path/to/arceos/ulib/axstd", features = ["..."] }
    # or use git repository:
    # axstd = { git = "https://github.com/arceos-org/arceos.git", features = ["..."] }
    ```

3. Call library functions from `axstd` in your code, just like the Rust [std](https://doc.rust-lang.org/std/) library.

    Remember to annotate the `main` function with `#[unsafe(no_mangle)]` (see this [example](examples/helloworld/src/main.rs)).

4. Build your application with ArceOS, by running the `make` command in the application directory:

    ```bash
    # in app directory
    make -C /path/to/arceos A=$(pwd) ARCH=<arch> run
    # more args: LOG=<log> SMP=<smp> NET=[y|n] ...
    ```

    All arguments and targets are the same as above.

### C

1. Create `axbuild.mk` and `features.txt` in your project:

    ```bash
    app/
    ├── foo.c
    ├── bar.c
    ├── axbuild.mk      # optional, if there is only one `main.c`
    └── features.txt    # optional, if only use default features
    ```

2. Add build targets to `axbuild.mk`, add features to enable to `features.txt` (see this [example](examples/httpserver-c/)):

    ```bash
    # in axbuild.mk
    app-objs := foo.o bar.o
    ```

    ```bash
    # in features.txt
    alloc
    paging
    net
    ```

3. Build your application with ArceOS, by running the `make` command in the application directory:

    ```bash
    # in app directory
    make -C /path/to/arceos A=$(pwd) ARCH=<arch> run
    # more args: LOG=<log> SMP=<smp> NET=[y|n] ...
    ```

## How to build ArceOS for specific platforms and devices

You need to manually link your application with the appropriate platform packages:

```rs
// Add this line to your application (for raspi4 platform)
extern crate axplat_aarch64_raspi;
```

Then set the `MYPLAT` variable when run `make`:

```bash
# Build helloworld for raspi4
make MYPLAT=axplat-aarch64-raspi SMP=4 A=examples/helloworld
```

You may also need to select the corrsponding device drivers by setting the `FEATURES` variable:

```bash
# Build the shell app for raspi4, and use the SD card driver
make MYPLAT=axplat-aarch64-raspi SMP=4 A=examples/shell FEATURES=page-alloc-4g,driver-bcm2835-sdhci BUS=mmio
# Build httpserver for the bare-metal x86_64 platform, and use the ixgbe and ramdisk driver
make PLAT_CONFIG=$(pwd)/configs/custom/x86_64-pc-oslab.toml A=examples/httpserver FEATURES=page-alloc-4g,driver-ixgbe,driver-ramdisk SMP=4
```

## How to reuse ArceOS modules in your own project

```toml
# In Cargo.toml
[dependencies]
axalloc = { git = "https://github.com/arceos-org/arceos.git", tag = "v0.2.0" } # kernel/memory/axalloc
axhal = { git = "https://github.com/arceos-org/arceos.git", tag = "v0.2.0" } # kernel/arch/axhal
```

## Design

![ArceOS logo](doc/figures/ArceOS.svg)
