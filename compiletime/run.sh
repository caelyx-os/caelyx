#!/usr/bin/env bash

set -xe

FLAGS="-accel kvm -cpu host -M q35"

if [ "$GDB" == "true" ]; then
  FLAGS="${FLAGS} -S -s -no-reboot -no-shutdown"
fi

mkdir -p build/boot/grub
mkdir -p dist
cp compiletime/grub.cfg build/boot/grub/grub.cfg
cp $1 build/boot/qos.elf
grub-mkrescue build -o dist/qos.iso
qemu-system-x86_64 -cdrom dist/qos.iso -m 1G ${FLAGS}