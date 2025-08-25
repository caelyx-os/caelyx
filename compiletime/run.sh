#!/usr/bin/env bash

set -xe

FLAGS="-M q35 -no-reboot -serial stdio -cpu qemu64"

if [ "$GDB" == "true" ]; then
  FLAGS="${FLAGS} -S -s"
fi

mkdir -p $OUT_DIR/iso_dir/boot/grub
cp compiletime/grub.cfg $OUT_DIR/iso_dir/boot/grub/grub.cfg
cp $1 $OUT_DIR/iso_dir/boot/caelyx.elf
grub-mkrescue $OUT_DIR/iso_dir -o $OUT_DIR/caelyx.iso
qemu-system-x86_64 -cdrom $OUT_DIR/caelyx.iso -m 1G ${FLAGS}

