#!/usr/bin/env bash

set -xe

FLAGS="-M q35 -no-reboot -serial stdio"

if [ "$GDB" == "true" ]; then
  FLAGS="${FLAGS} -S -s"
fi

mkdir -p build/boot/grub
mkdir -p dist
cp compiletime/grub.cfg build/boot/grub/grub.cfg
cp $1 build/boot/caelyx.elf
grub-mkrescue build -o dist/caelyx.iso
qemu-system-x86_64 -cdrom dist/caelyx.iso -m 1G ${FLAGS}

