use crate::util::align_ptr_up;

mod bindings;

#[derive(Clone, Copy, Debug)]
pub struct TagIterator {
    tag: *const bindings::multiboot_tag,
}

impl TagIterator {
    pub fn new(info: *const ()) -> Self {
        let mb_info_ptr = info as *const bindings::multiboot_info;
        Self {
            tag: unsafe { (*mb_info_ptr).tags.as_ptr() },
        }
    }

    pub fn get_tag(&self) -> Option<MultibootTag> {
        let tag_type = unsafe { (*self.tag).type_ };
        match tag_type {
            bindings::MULTIBOOT_TAG_TYPE_END => Some(MultibootTag::End),
            bindings::MULTIBOOT_TAG_TYPE_CMDLINE => Some(MultibootTag::CmdLine),
            bindings::MULTIBOOT_TAG_TYPE_BOOT_LOADER_NAME => Some(MultibootTag::BootLoaderName),
            bindings::MULTIBOOT_TAG_TYPE_MODULE => Some(MultibootTag::Module),
            bindings::MULTIBOOT_TAG_TYPE_BASIC_MEMINFO => Some(MultibootTag::BasicMemInfo),
            bindings::MULTIBOOT_TAG_TYPE_BOOTDEV => Some(MultibootTag::BootDev),
            bindings::MULTIBOOT_TAG_TYPE_MMAP => Some(MultibootTag::Mmap),
            bindings::MULTIBOOT_TAG_TYPE_VBE => Some(MultibootTag::Vbe),
            bindings::MULTIBOOT_TAG_TYPE_FRAMEBUFFER => Some(MultibootTag::FrameBuffer),
            bindings::MULTIBOOT_TAG_TYPE_ELF_SECTIONS => Some(MultibootTag::ElfSections),
            bindings::MULTIBOOT_TAG_TYPE_APM => Some(MultibootTag::Apm),
            bindings::MULTIBOOT_TAG_TYPE_EFI32 => Some(MultibootTag::Efi32),
            bindings::MULTIBOOT_TAG_TYPE_EFI64 => Some(MultibootTag::Efi64),
            bindings::MULTIBOOT_TAG_TYPE_SMBIOS => Some(MultibootTag::SmBios),
            bindings::MULTIBOOT_TAG_TYPE_ACPI_OLD => Some(MultibootTag::AcpiOld),
            bindings::MULTIBOOT_TAG_TYPE_ACPI_NEW => Some(MultibootTag::AcpiNew),
            bindings::MULTIBOOT_TAG_TYPE_NETWORK => Some(MultibootTag::Network),
            bindings::MULTIBOOT_TAG_TYPE_EFI_MMAP => Some(MultibootTag::EfiMMap),
            bindings::MULTIBOOT_TAG_TYPE_EFI_BS => Some(MultibootTag::EfiBs),
            bindings::MULTIBOOT_TAG_TYPE_EFI32_IH => Some(MultibootTag::Efi32Ih),
            bindings::MULTIBOOT_TAG_TYPE_EFI64_IH => Some(MultibootTag::Efi64Ih),
            bindings::MULTIBOOT_TAG_TYPE_LOAD_BASE_ADDR => Some(MultibootTag::LoadBaseAddr),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MultibootTag {
    End,
    CmdLine,
    BootLoaderName,
    Module,
    BasicMemInfo,
    BootDev,
    Mmap,
    Vbe,
    FrameBuffer,
    ElfSections,
    Apm,
    Efi32,
    Efi64,
    SmBios,
    AcpiOld,
    AcpiNew,
    Network,
    EfiMMap,
    EfiBs,
    Efi32Ih,
    Efi64Ih,
    LoadBaseAddr,
}

impl Iterator for TagIterator {
    type Item = MultibootTag;
    fn next(&mut self) -> Option<Self::Item> {
        let tag_ref = unsafe { &*self.tag };
        let ret = self.get_tag();

        if matches!(ret, Some(MultibootTag::End)) {
            // end tag
            return None;
        }

        if ret.is_some() {
            // a tag, but not end
            self.tag = align_ptr_up((self.tag as usize + tag_ref.size as usize) as *const u8, 8)
                as *const bindings::multiboot_tag;
            ret
        } else {
            // unknown tag
            self.next()
        }
    }
}
