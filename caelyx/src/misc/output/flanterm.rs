use core::{
    fmt::{Arguments, Write},
    ptr::null_mut,
};

use crate::{
    boot::multiboot2,
    misc::{isituninit::IsItUninit, str_writer::StrWriter},
    mm::{virt_page_alloc, vmm},
    sync::mutex::Mutex,
};
use flanterm::{flanterm_context, flanterm_fb_context, flanterm_fb_init, flanterm_write};

static FLANTERM_CONTEXT: Mutex<IsItUninit<&flanterm_context>> = Mutex::new(IsItUninit::uninit());
static DISABLE_FLANTERM_LOGGING: Mutex<bool> = Mutex::new(false);

pub fn init(tag_iter: &mut multiboot2::TagIterator) {
    let tag_ = tag_iter.find(|x| matches!(x, multiboot2::MultibootTag::FrameBuffer(_)));
    if tag_.is_none() {
        return;
    }
    let tag = match tag_.unwrap() {
        multiboot2::MultibootTag::FrameBuffer(fb) => fb,
        _ => unreachable!(),
    }
    .as_fb();
    let fb = tag.addr;
    let fb_width = tag.width as usize;
    let fb_height = tag.height as usize;
    let fb_pitch = tag.pitch as usize;
    let fb_red_mask_size = tag.red_mask_size;
    let fb_red_mask_shift = tag.red_mask_shift;
    let fb_green_mask_size = tag.green_mask_size;
    let fb_green_mask_shift = tag.green_mask_shift;
    let fb_blue_mask_size = tag.blue_mask_size;
    let fb_blue_mask_shift = tag.blue_mask_shift;

    unsafe {
        let ctx = flanterm_fb_init(
            None,
            None,
            fb as *mut u32,
            fb_width,
            fb_height,
            fb_pitch,
            fb_red_mask_size,
            fb_red_mask_shift,
            fb_green_mask_size,
            fb_green_mask_shift,
            fb_blue_mask_size,
            fb_blue_mask_shift,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            1,
            2,
            1,
            1,
            1,
            0,
        );

        let mut lock = FLANTERM_CONTEXT.lock();
        lock.write(&*ctx);
    }
}

pub fn print_fmt(args: Arguments<'_>) {
    let _ = StrWriter {
        write: |s| {
            if *DISABLE_FLANTERM_LOGGING.lock() {
                return;
            }

            let lock = FLANTERM_CONTEXT.lock();
            if !lock.initialized() {
                return;
            }

            unsafe {
                flanterm_write(
                    *lock.get_ref() as *const _ as *mut flanterm_context,
                    s.as_ptr() as *const i8,
                    s.len(),
                )
            };
        },
    }
    .write_fmt(args);
}

pub fn paging_fix() {
    *DISABLE_FLANTERM_LOGGING.lock() = true;
    let lock = FLANTERM_CONTEXT.lock();
    if !lock.initialized() {
        return;
    }

    #[allow(invalid_reference_casting)]
    let ft_ctx = unsafe { &mut *((*lock.get_ref()) as *const _ as *mut flanterm_fb_context) };

    let pitch = ft_ctx.pitch;
    let height = ft_ctx.height;
    let addr = ft_ctx.framebuffer as u32;
    let pages = u32::div_ceil(pitch as u32 * height as u32, 4096);
    let virt_addr =
        virt_page_alloc::allocate(pages as usize).expect("Could not allocate virt pages for fb");
    for i in 0..pages {
        vmm::map(
            addr + i * 4096,
            virt_addr + i * 4096,
            false,
            true,
            true,
            false,
        );
    }

    ft_ctx.framebuffer = virt_addr as *mut u32;
    *DISABLE_FLANTERM_LOGGING.lock() = false;
}
