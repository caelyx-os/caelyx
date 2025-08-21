[bits 32] ; Multiboot2 puts us into 32-bit protected mode - that's why we set the mode of this code to 32-bits

; A explanation on this is located below, look at the line where we 'push ebx' in the caelyx__start label
section .multiboot
align 8
caelyx_multiboot2_header_start:
dd 0xE85250D6
dd 0
dd caelyx_multiboot2_header_end - caelyx_multiboot2_header_start
dd -(0xE85250D6 + 0 + (caelyx_multiboot2_header_end - caelyx_multiboot2_header_start))

align 8
caelyx_multiboot2_header_mmap_tag_start:
dw 6
dw 0
dd caelyx_multiboot2_header_mmap_tag_end - caelyx_multiboot2_header_mmap_tag_start
caelyx_multiboot2_header_mmap_tag_end:

align 8
caelyx_multiboot2_header_end_tag_start:
dw 0
dw 0
dd caelyx_multiboot2_header_end - caelyx_multiboot2_header_end_tag_start
caelyx_multiboot2_header_end:

extern caelyx_kmain ; We are going to use the external function which is our kernel entry point - so we extern it
global caelyx__start 
section .text
caelyx__start:
  mov [stack_top], ebx
  cli ; Clear interrupt flag
  cld ; Clear direction flag

  mov esp, stack_top ; We need to setup a stack which is needed for pretty much anything in a high-level language (i mean anything higher-level than assembly)
                    ; Since the stack grows downwards, we will need to set the stack pointer to the stack top.

  push dword [stack_top] ; When booting with the multiboot2 boot protocol, the kernel must provide a Multiboot2 Header -
           ; which we do above. In return, we get to skip the pain of rolling our own bootloader, and get
           ; provided with a lot of information by the bootloader. The bootloader must pass the pointer to the
           ; information in the ebx register, the information is a structure commonly referred to as the
           ; Multiboot2 Information Structure, For more information, look here: 
           ; https://www.gnu.org/software/grub/manual/multiboot2/html_node/multiboot2_002eh.html
           ; https://www.gnu.org/software/grub/manual/multiboot2/multiboot.pdf

  jmp caelyx_kmain ; Jump to our kernel entry point in rust - from there we do the rest needed

section .bss
stack_bottom:
; Define a 32K stack.
resb 32768
stack_top:
