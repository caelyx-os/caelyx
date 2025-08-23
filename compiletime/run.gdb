file target/target/debug/caelyx.elf
set architecture i386:x86-64
set disassembly-flavor intel
set demangle-style rust
set pagination off
target remote localhost:1234
b caelyx_kmain
c
