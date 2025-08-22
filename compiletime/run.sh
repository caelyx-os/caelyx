#!/usr/bin/env bash

set -xe

FLAGS="-accel kvm -cpu host -M q35 -serial stdio"

if [ "$GDB" == "true" ]; then
  FLAGS="${FLAGS} -S -s -no-reboot -no-shutdown"
fi

mkdir -p build/boot/grub
mkdir -p dist
cp compiletime/grub.cfg build/boot/grub/grub.cfg
cp $1 build/boot/caelyx.elf
grub-mkrescue build -o dist/caelyx.iso
qemu-system-x86_64 -cdrom dist/caelyx.iso -m 1G ${FLAGS}

