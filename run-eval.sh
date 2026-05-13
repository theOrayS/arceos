#!/usr/bin/env bash
set -e

ARCH="${1:-rv}"

if [ "$ARCH" = "rv" ]; then
    make run-rv ARCH=riscv64 SMP=1 MEM=1G RV_TESTSUITE_IMG=sdcard-rv.img
elif [ "$ARCH" = "la" ]; then
    make run-la ARCH=loongarch64 SMP=1 MEM=1G LA_TESTSUITE_IMG=sdcard-la.img
else
    echo "用法: $0 [rv|la]"
    exit 1
fi
