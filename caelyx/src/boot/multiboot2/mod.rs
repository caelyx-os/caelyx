use core::marker::PhantomData;

use crate::misc::ptr_align::align_ptr_up;

mod bindings;

#[derive(Clone, Copy, Debug)]
pub struct TagIterator<'a> {
    tag: *const bindings::multiboot_tag,
    __phantom: PhantomData<&'a ()>,
}

impl<'a> TagIterator<'a> {
    pub fn new(info: *const ()) -> Self {
        let mb_info_ptr = info as *const bindings::multiboot_info;
        Self {
            tag: unsafe { (*mb_info_ptr).tags.as_ptr() },
            __phantom: PhantomData,
        }
    }

    pub fn get_tag(&self) -> Option<MultibootTag<'a>> {
        let tag_type = unsafe { (*self.tag).type_ };
        match tag_type {
            bindings::MULTIBOOT_TAG_TYPE_END => Some(MultibootTag::End),
            bindings::MULTIBOOT_TAG_TYPE_CMDLINE => Some(MultibootTag::CmdLine),
            bindings::MULTIBOOT_TAG_TYPE_BOOT_LOADER_NAME => Some(MultibootTag::BootLoaderName),
            bindings::MULTIBOOT_TAG_TYPE_MODULE => Some(MultibootTag::Module),
            bindings::MULTIBOOT_TAG_TYPE_BASIC_MEMINFO => Some(MultibootTag::BasicMemInfo),
            bindings::MULTIBOOT_TAG_TYPE_BOOTDEV => Some(MultibootTag::BootDev),
            bindings::MULTIBOOT_TAG_TYPE_MMAP => {
                let mmap_tag = self.tag as *const bindings::multiboot_tag_mmap;
                Some(MultibootTag::Mmap(unsafe {
                    core::slice::from_raw_parts(
                        (*mmap_tag).entries.as_ptr() as *const _MultibootMmapPart,
                        ((*mmap_tag).size as usize
                            - core::mem::size_of::<bindings::multiboot_tag_mmap>())
                            / core::mem::size_of::<bindings::multiboot_mmap_entry>(),
                    )
                }))
            }
            bindings::MULTIBOOT_TAG_TYPE_VBE => Some(MultibootTag::Vbe),
            bindings::MULTIBOOT_TAG_TYPE_FRAMEBUFFER => {
                Some(MultibootTag::FrameBuffer(_MultibootFramebuffer(unsafe {
                    &*(self.tag as *const bindings::multiboot_tag_framebuffer)
                })))
            }
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
pub struct MultibootMmapEntry {
    pub start: u64,
    pub size: u64,
    pub type_: MultibootMmapEntryType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultibootMmapEntryType {
    Available = 1,
    Reserved = 2,
    AcpiReclaim = 3,
    Nvs = 4,
    Badram = 5,
    Unknown,
}

impl MultibootMmapEntryType {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Available,
            2 => Self::Reserved,
            3 => Self::AcpiReclaim,
            4 => Self::Nvs,
            5 => Self::Badram,
            _ => Self::Unknown,
        }
    }
}

#[repr(C, packed)]
pub struct _MultibootMmapPart(bindings::multiboot_mmap_entry);
impl _MultibootMmapPart {
    pub fn as_mmap_entry(&self) -> MultibootMmapEntry {
        MultibootMmapEntry {
            start: self.0.addr,
            size: self.0.len,
            type_: MultibootMmapEntryType::from_u32(self.0.type_),
        }
    }
}

impl core::fmt::Debug for _MultibootMmapPart {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "_MultibootMmapPart")
    }
}

#[derive(Debug)]
pub enum MultibootTag<'a> {
    End,
    CmdLine,
    BootLoaderName,
    Module,
    BasicMemInfo,
    BootDev,
    Mmap(&'a [_MultibootMmapPart]),
    Vbe,
    FrameBuffer(_MultibootFramebuffer),
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct MultibootFramebuffer {
    pub addr: u32,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub red_mask_size: u8,
    pub red_mask_shift: u8,
    pub green_mask_size: u8,
    pub green_mask_shift: u8,
    pub blue_mask_size: u8,
    pub blue_mask_shift: u8,
}

#[repr(C, packed)]
pub struct _MultibootFramebuffer(&'static bindings::multiboot_tag_framebuffer);
impl _MultibootFramebuffer {
    pub fn as_fb(&self) -> MultibootFramebuffer {
        let common = self.0.common;
        assert_eq!(common.framebuffer_bpp, 32);
        let addr = common.framebuffer_addr as u32;
        let width = common.framebuffer_width;
        let height = common.framebuffer_height;
        let pitch = common.framebuffer_pitch;
        let color_info = unsafe { self.0.__bindgen_anon_1.__bindgen_anon_2.as_ref() };
        let red_mask_size = color_info.framebuffer_red_mask_size;
        let red_mask_shift = color_info.framebuffer_red_field_position;
        let green_mask_size = color_info.framebuffer_green_mask_size;
        let green_mask_shift = color_info.framebuffer_green_field_position;
        let blue_mask_size = color_info.framebuffer_blue_mask_size;
        let blue_mask_shift = color_info.framebuffer_blue_field_position;
        MultibootFramebuffer {
            addr,
            width,
            height,
            pitch,
            red_mask_size,
            red_mask_shift,
            green_mask_size,
            green_mask_shift,
            blue_mask_size,
            blue_mask_shift,
        }
    }
}

impl core::fmt::Debug for _MultibootFramebuffer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "_MultibootFramebuffer")
    }
}

impl<'a> Iterator for TagIterator<'a> {
    type Item = MultibootTag<'a>;
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
