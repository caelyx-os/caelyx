use crate::fatal;
use crate::misc::output::raw_print::print_line_ending;
use crate::x86::halt;
use crate::x86::idt::interrupt_control::disable_interrupts;

#[panic_handler]
pub fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    print_line_ending();
    fatal!(r"  -------------            -------------    ");
    fatal!(r"/             \          /             \  ");
    fatal!(r"|             |          |             |  ");
    fatal!(r"|             |          |             |  ");
    fatal!(r"|             |          |             |  ");
    fatal!(r"\             /          \             /  ");
    fatal!(r" -------------            -------------   ");
    fatal!(r"                                          ");
    fatal!(r"   -----------------------------------    ");
    fatal!(r"  /                                   \   ");
    fatal!(r" /                                     \  ");
    print_line_ending();

    if let Some(loc) = info.location() {
        fatal!("panic occured at {}:{}", loc.file(), loc.line());
    } else {
        fatal!("panic occured");
    }

    fatal!("\tmessage: \"{}\"", info.message());
    disable_interrupts();
    fatal!("kernel halted");
    loop {
        halt();
    }
}
