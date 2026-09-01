#![no_std]
#![no_main]

extern crate alloc;

mod allocator;
mod random;
mod time;
mod framebuffer;
mod banner;
mod port;
mod keyboard;
mod ata;
mod fs;
mod shell;
mod wasm;
mod script;
mod gui;
mod apps;
mod mouse;
mod html;
mod editor;
mod monitor;
mod sound;
mod piano;
mod login;
mod tcp;

use core::arch::asm;
use limine::BaseRevision;
use limine::request::{FramebufferRequest, RequestsEndMarker, RequestsStartMarker};
use framebuffer::{GREEN, GRAY};

#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[unsafe(link_section = ".requests_start_marker")]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".requests_end_marker")]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    assert!(BASE_REVISION.is_supported());

    if let Some(fb_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(fb) = fb_response.framebuffers().next() {
            framebuffer::init(
                fb.addr(),
                fb.width() as usize,
                fb.height() as usize,
                fb.pitch() as usize,
            );
        }
    }

    allocator::init();
    random::init();
    time::init();

    banner::show();

    print_color!(GRAY, "booting...\n");
    print_color!(GREEN, "[ok]");
    println!(" framebuffer initialized");

    fs::init();
    print_color!(GREEN, "[ok]");
    println!(" filesystem mounted");

    login::run();

    shell::run();
}

#[panic_handler]
fn rust_panic(_info: &core::panic::PanicInfo) -> ! {
    hcf();
}

fn hcf() -> ! {
    loop {
        unsafe {
            #[cfg(target_arch = "x86_64")]
            asm!("hlt");
            #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
            asm!("wfi");
            #[cfg(target_arch = "loongarch64")]
            asm!("idle 0");
        }
    }
}
