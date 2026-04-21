# Available arguments:
# * General options:
#     - `ARCH`: Target architecture: x86_64, riscv64, aarch64, loongarch64
#     - `MYPLAT`: Package name of the target platform crate.
#     - `PLAT_CONFIG`: Path to the platform configuration file.
#     - `SMP`: Override maximum CPU number specified in the platform config. For
#       statically configured platforms, this is also the number of CPUs to boot
#       and for platforms with runtime CPU detection, this is the upper limit of
#       CPUs.
#     - `MODE`: Build mode: release, debug
#     - `LOG:` Logging level: warn, error, info, debug, trace
#     - `V`: Verbose level: (empty), 1, 2
#     - `TARGET_DIR`: Artifact output directory (cargo target directory)
#     - `EXTRA_CONFIG`: Extra config specification file
#     - `OUT_CONFIG`: Final config file that takes effect
#     - `UIMAGE`: To generate U-Boot image
#     - `LD_SCRIPT`: Use a custom linker script file.
# * App options:
#     - `A` or `APP`: Path to the application
#     - `FEATURES`: Features os ArceOS modules to be enabled.
#     - `APP_FEATURES`: Features of (rust) apps to be enabled.
# * QEMU options:
#     - `BLK`: Enable storage devices (virtio-blk)
#     - `NET`: Enable network devices (virtio-net)
#     - `GRAPHIC`: Enable display devices and graphic output (virtio-gpu)
#     - `BUS`: Device bus type: mmio, pci
#     - `MEM`: Memory size (default is 128M)
#     - `DISK_IMG`: Path to the virtual disk image
#     - `ACCEL`: Enable hardware acceleration (KVM on linux)
#     - `QEMU_LOG`: Enable QEMU logging (log file is "qemu.log")
#     - `NET_DUMP`: Enable network packet dump (log file is "netdump.pcap")
#     - `NET_DEV`: QEMU netdev backend types: user, tap, bridge
#     - `VFIO_PCI`: PCI device address in the format "bus:dev.func" to passthrough
#     - `VHOST`: Enable vhost-net for tap backend (only for `NET_DEV=tap`)
# * Network options:
#     - `IP`: ArceOS IPv4 address (default is 10.0.2.15 for QEMU user netdev)
#     - `GW`: Gateway IPv4 address (default is 10.0.2.2 for QEMU user netdev)

# General options
ARCH ?= x86_64
MYPLAT ?=
PLAT_CONFIG ?=
SMP ?=
MODE ?= release
LOG ?= warn
V ?=
TARGET_DIR ?= $(PWD)/target
EXTRA_CONFIG ?=
OUT_CONFIG ?= $(PWD)/.axconfig.toml
UIMAGE ?= n

# Kernel build options
KERNEL_APP ?= examples/shell
KERNEL_FEATURES ?= alloc,paging,irq,multitask,fs,net
KERNEL_APP_FEATURES ?= auto-run-tests,uspace
KERNEL_RV_APP_FEATURES ?= $(KERNEL_APP_FEATURES)
KERNEL_LA_APP_FEATURES ?= $(KERNEL_APP_FEATURES)
KERNEL_MODE ?= release
KERNEL_LOG ?= info
KERNEL_SMP ?= 1
KERNEL_BUILD_DIR ?= $(CURDIR)/build/kernels
KERNEL_TARGET_DIR ?= $(KERNEL_BUILD_DIR)/target
KERNEL_RV_OUT_DIR ?= $(KERNEL_BUILD_DIR)/riscv64
KERNEL_LA_OUT_DIR ?= $(KERNEL_BUILD_DIR)/loongarch64
KERNEL_RV_CONFIG ?= $(KERNEL_BUILD_DIR)/riscv64.axconfig.toml
KERNEL_LA_CONFIG ?= $(KERNEL_BUILD_DIR)/loongarch64.axconfig.toml
KERNEL_RV_TARGET_DIR ?= $(KERNEL_TARGET_DIR)/riscv64
KERNEL_LA_TARGET_DIR ?= $(KERNEL_TARGET_DIR)/loongarch64
KERNEL_RV_AXCONFIG_WRITES ?= -w plat.phys-memory-size=0x4000_0000
KERNEL_LA_AXCONFIG_WRITES ?= -w plat.phys-memory-size=0x4000_0000
KERNEL_RV ?= $(CURDIR)/kernel-rv
KERNEL_LA ?= $(CURDIR)/kernel-la
TESTSUITE_DIR ?= $(abspath $(CURDIR)/../testsuits-for-oskernel)
RV_TESTSUITE_IMG ?= $(TESTSUITE_DIR)/sdcard-rv.img
LA_TESTSUITE_IMG ?= $(TESTSUITE_DIR)/sdcard-la.img
RV_TESTSUITE_RUN_IMG ?= /tmp/arceos-sdcard-rv.run.qcow2
LA_TESTSUITE_RUN_IMG ?= /tmp/arceos-sdcard-la.run.qcow2
RV_AUX_DISK ?= $(CURDIR)/disk.img
LA_AUX_DISK ?= $(CURDIR)/disk-la.img
RV_MEM ?= 1G
LA_MEM ?= 1G
RV_NETDEV_ARGS ?= user,id=net
LA_NETDEV_ARGS ?= user,id=net0,hostfwd=tcp::5555-:5555,hostfwd=udp::5555-:5555
DOCKER_IMAGE ?= orays-arceos-dev

# App options
A ?= examples/helloworld
APP ?= $(A)
FEATURES ?=
APP_FEATURES ?=

# QEMU options
BLK ?= n
NET ?= n
GRAPHIC ?= n
BUS ?= pci
MEM ?= 128M
ACCEL ?=
QEMU_ARGS ?=

DISK_IMG ?= disk.img
QEMU_LOG ?= n
NET_DUMP ?= n
NET_DEV ?= user
VFIO_PCI ?=
VHOST ?= n

# Network options
IP ?= 10.0.2.15
GW ?= 10.0.2.2

# App type
ifeq ($(wildcard $(APP)),)
  $(error Application path "$(APP)" is not valid)
endif

ifneq ($(wildcard $(APP)/Cargo.toml),)
  APP_TYPE := rust
else
  APP_TYPE := c
endif

.DEFAULT_GOAL := all

ifneq ($(filter $(or $(MAKECMDGOALS), $(.DEFAULT_GOAL)), build disasm run justrun debug defconfig oldconfig),)
# Install dependencies
include scripts/make/deps.mk
# Platform resolving
include scripts/make/platform.mk
# Configuration generation
include scripts/make/config.mk
# Feature parsing
include scripts/make/features.mk
endif

# Target
ifeq ($(ARCH), x86_64)
  TARGET := x86_64-unknown-none
else ifeq ($(ARCH), aarch64)
  TARGET := aarch64-unknown-none-softfloat
else ifeq ($(ARCH), riscv64)
  TARGET := riscv64gc-unknown-none-elf
else ifeq ($(ARCH), loongarch64)
  TARGET := loongarch64-unknown-none-softfloat
else
  $(error "ARCH" must be one of "x86_64", "riscv64", "aarch64" or "loongarch64")
endif

export AX_ARCH=$(ARCH)
export AX_PLATFORM=$(PLAT_NAME)
export AX_MODE=$(MODE)
export AX_LOG=$(LOG)
export AX_TARGET=$(TARGET)
export AX_IP=$(IP)
export AX_GW=$(GW)

ifneq ($(filter $(MAKECMDGOALS),unittest unittest_no_fail_fast clippy doc doc_check_missing),)
  # When running unit tests or other tests unrelated to a specific platform,
  # set `AX_CONFIG_PATH` to empty for dummy config
  unexport AX_CONFIG_PATH
else
  export AX_CONFIG_PATH=$(OUT_CONFIG)
endif

# Binutils
CROSS_COMPILE ?= $(ARCH)-linux-musl-
CC := $(CROSS_COMPILE)gcc
AR := $(CROSS_COMPILE)ar
RANLIB := $(CROSS_COMPILE)ranlib
LD := rust-lld -flavor gnu

OBJDUMP ?= rust-objdump -d --print-imm-hex --x86-asm-syntax=intel
OBJCOPY ?= rust-objcopy --binary-architecture=$(ARCH)
GDB ?= gdb-multiarch

# Paths
OUT_DIR ?= $(APP)
LD_SCRIPT ?= $(TARGET_DIR)/$(TARGET)/$(MODE)/linker_$(PLAT_NAME).lds

APP_NAME := $(shell basename $(APP))
OUT_ELF := $(OUT_DIR)/$(APP_NAME)_$(PLAT_NAME).elf
OUT_BIN := $(patsubst %.elf,%.bin,$(OUT_ELF))
OUT_UIMG := $(patsubst %.elf,%.uimg,$(OUT_ELF))
ifeq ($(UIMAGE), y)
  FINAL_IMG := $(OUT_UIMG)
else
  FINAL_IMG := $(OUT_BIN)
endif

kernel_build_args := \
	A=$(KERNEL_APP) \
	MODE=$(KERNEL_MODE) \
	LOG=$(KERNEL_LOG) \
	SMP=$(KERNEL_SMP) \
	FEATURES=$(KERNEL_FEATURES)

KERNEL_RV_ELF := $(KERNEL_RV_OUT_DIR)/$(notdir $(KERNEL_APP))_riscv64-qemu-virt.elf
KERNEL_LA_ELF := $(KERNEL_LA_OUT_DIR)/$(notdir $(KERNEL_APP))_loongarch64-qemu-virt.elf
KERNEL_RV_BIN := $(patsubst %.elf,%.bin,$(KERNEL_RV_ELF))
KERNEL_RV_WRAP_OBJ := $(KERNEL_BUILD_DIR)/kernel-rv.wrap.o

ifneq ($(wildcard $(RV_AUX_DISK)),)
rv_aux_drive := \
	-drive file=$(RV_AUX_DISK),if=none,format=raw,id=x1 \
	-device virtio-blk-device,drive=x1,bus=virtio-mmio-bus.1
endif

ifneq ($(wildcard $(LA_AUX_DISK)),)
la_aux_drive := \
	-drive file=$(LA_AUX_DISK),if=none,format=raw,id=x1 \
	-device virtio-blk-pci,drive=x1,bus=virtio-mmio-bus.1
endif

all:
	$(MAKE) test_build ARCH=riscv64 BUS=mmio \
		APP_FEATURES="$(KERNEL_RV_APP_FEATURES)" \
		AXCONFIG_WRITES="$(KERNEL_RV_AXCONFIG_WRITES)" \
		OUT_DIR=$(KERNEL_RV_OUT_DIR) \
		OUT_CONFIG=$(KERNEL_RV_CONFIG) \
		TARGET_DIR=$(KERNEL_RV_TARGET_DIR)
	$(MAKE) test_build ARCH=loongarch64 BUS=pci \
		APP_FEATURES="$(KERNEL_LA_APP_FEATURES)" \
		AXCONFIG_WRITES="$(KERNEL_LA_AXCONFIG_WRITES)" \
		OUT_DIR=$(KERNEL_LA_OUT_DIR) \
		OUT_CONFIG=$(KERNEL_LA_CONFIG) \
		TARGET_DIR=$(KERNEL_LA_TARGET_DIR)

include scripts/make/utils.mk
include scripts/make/build.mk
include scripts/make/qemu.mk
ifeq ($(PLAT_NAME), aarch64-raspi4)
  include scripts/make/raspi4.mk
else ifeq ($(PLAT_NAME), aarch64-bsta1000b)
  include scripts/make/bsta1000b-fada.mk
endif

defconfig:
	$(call defconfig)

oldconfig:
	$(call oldconfig)

build: $(OUT_DIR) $(FINAL_IMG)

test_build:
	$(MAKE) $(kernel_build_args) \
		ARCH=$(ARCH) BUS=$(BUS) \
		APP_FEATURES="$(APP_FEATURES)" \
		AXCONFIG_WRITES="$(AXCONFIG_WRITES)" \
		OUT_DIR=$(OUT_DIR) \
		OUT_CONFIG=$(OUT_CONFIG) \
		TARGET_DIR=$(TARGET_DIR) \
		build
ifeq ($(ARCH),riscv64)
	@mkdir -p $(dir $(KERNEL_RV))
	rust-objcopy -I binary -O elf64-littleriscv --rename-section .data=.text,alloc,load,readonly,code $(KERNEL_RV_BIN) $(KERNEL_RV_WRAP_OBJ)
	rust-lld -flavor gnu -m elf64lriscv -T scripts/make/riscv64-kernel-wrap.lds $(KERNEL_RV_WRAP_OBJ) -o $(KERNEL_RV)
else ifeq ($(ARCH),loongarch64)
	@mkdir -p $(dir $(KERNEL_LA))
	cp $(KERNEL_LA_ELF) $(KERNEL_LA)
else
	$(error "test_build" only supports "ARCH=riscv64" or "ARCH=loongarch64")
endif

kernel-rv:
	$(MAKE) test_build ARCH=riscv64 BUS=mmio \
		APP_FEATURES="$(KERNEL_RV_APP_FEATURES)" \
		AXCONFIG_WRITES="$(KERNEL_RV_AXCONFIG_WRITES)" \
		OUT_DIR=$(KERNEL_RV_OUT_DIR) \
		OUT_CONFIG=$(KERNEL_RV_CONFIG) \
		TARGET_DIR=$(KERNEL_RV_TARGET_DIR)

kernel-la:
	$(MAKE) test_build ARCH=loongarch64 BUS=pci \
		APP_FEATURES="$(KERNEL_LA_APP_FEATURES)" \
		AXCONFIG_WRITES="$(KERNEL_LA_AXCONFIG_WRITES)" \
		OUT_DIR=$(KERNEL_LA_OUT_DIR) \
		OUT_CONFIG=$(KERNEL_LA_CONFIG) \
		TARGET_DIR=$(KERNEL_LA_TARGET_DIR)

docker-image:
	docker build -t $(DOCKER_IMAGE) -f Dockerfile .

docker: docker-image
	docker run --rm -it -v $(abspath $(CURDIR)/..):/code -w /code/arceos $(DOCKER_IMAGE) bash

testsuite-sdcard:
	$(MAKE) -C $(TESTSUITE_DIR) sdcard

prepare-rv-testsuite-img:
	@mkdir -p $(dir $(RV_TESTSUITE_RUN_IMG))
	rm -f $(RV_TESTSUITE_RUN_IMG)
	qemu-img create -f qcow2 -F raw -b $(RV_TESTSUITE_IMG) $(RV_TESTSUITE_RUN_IMG)

prepare-la-testsuite-img:
	@mkdir -p $(dir $(LA_TESTSUITE_RUN_IMG))
	rm -f $(LA_TESTSUITE_RUN_IMG)
	qemu-img create -f qcow2 -F raw -b $(LA_TESTSUITE_IMG) $(LA_TESTSUITE_RUN_IMG)

run-rv: kernel-rv prepare-rv-testsuite-img
	qemu-system-riscv64 -machine virt -kernel $(KERNEL_RV) -m $(RV_MEM) -nographic -smp $(KERNEL_SMP) -bios default -drive file=$(RV_TESTSUITE_RUN_IMG),if=none,format=qcow2,id=x0 \
		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 -no-reboot -device virtio-net-device,netdev=net -netdev $(RV_NETDEV_ARGS) \
		-rtc base=utc $(rv_aux_drive)

run-la: kernel-la prepare-la-testsuite-img
	qemu-system-loongarch64 -kernel $(KERNEL_LA) -m $(LA_MEM) -nographic -smp $(KERNEL_SMP) -drive file=$(LA_TESTSUITE_RUN_IMG),if=none,format=qcow2,id=x0 \
		-device virtio-blk-pci,drive=x0 -no-reboot -device virtio-net-pci,netdev=net0 \
		-netdev $(LA_NETDEV_ARGS) -rtc base=utc $(la_aux_drive)

disasm:
	$(OBJDUMP) $(OUT_ELF) | less

run: build justrun

justrun:
	$(call run_qemu)

debug: build
	$(call run_qemu_debug) &
	sleep 1
	$(GDB) $(OUT_ELF) \
	  -ex 'target remote localhost:1234' \
	  -ex 'b rust_entry' \
	  -ex 'continue' \
	  -ex 'disp /16i $$pc'

clippy:
ifeq ($(origin ARCH), command line)
	$(call cargo_clippy,--target $(TARGET))
else
	$(call cargo_clippy)
endif

doc:
	$(call cargo_doc)

doc_check_missing:
	$(call cargo_doc)

fmt:
	cargo fmt --all

fmt_c:
	@clang-format --style=file -i $(shell find ulib/axlibc -iname '*.c' -o -iname '*.h')

unittest:
	$(call unit_test)

unittest_no_fail_fast:
	$(call unit_test,--no-fail-fast)

disk_img:
ifneq ($(wildcard $(DISK_IMG)),)
	@printf "$(YELLOW_C)warning$(END_C): disk image \"$(DISK_IMG)\" already exists!\n"
else
	$(call make_disk_image,fat32,$(DISK_IMG))
endif

clean: clean_c
	rm -rf $(APP)/*.bin $(APP)/*.elf $(OUT_CONFIG)
	rm -rf $(KERNEL_BUILD_DIR) $(KERNEL_RV) $(KERNEL_LA)
	cargo clean

clean_c::
	rm -rf ulib/axlibc/build_*
	rm -rf $(app-objs)

.PHONY: all defconfig oldconfig \
	build disasm run justrun debug \
	clippy doc doc_check_missing fmt fmt_c unittest unittest_no_fail_fast \
	disk_img clean clean_c \
	test_build kernel-rv kernel-la docker-image docker testsuite-sdcard \
	prepare-rv-testsuite-img prepare-la-testsuite-img run-rv run-la
