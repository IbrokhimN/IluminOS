use crate::mem::heap;
use crate::println;
use core::arch::asm;

const VEC_PAGE_FAULT: usize = 14;
const VEC_GENERAL_PROTECTION: usize = 13;
const VEC_DOUBLE_FAULT: usize = 8;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    zero: u32,
}

impl IdtEntry {
    const fn empty() -> Self {
        IdtEntry {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            zero: 0,
        }
    }

    fn set_handler(&mut self, handler: u64, selector: u16) {
        self.offset_low = (handler & 0xFFFF) as u16;
        self.offset_mid = ((handler >> 16) & 0xFFFF) as u16;
        self.offset_high = ((handler >> 32) & 0xFFFFFFFF) as u32;
        self.selector = selector;
        self.ist = 0;
        self.type_attr = 0x8E; // present interrupt gate
        self.zero = 0;
    }
}

static mut IDT: [IdtEntry; 32] = [IdtEntry::empty(); 32];

#[repr(C, packed)]
struct IdtPointer {
    limit: u16,
    base: u64,
}

#[repr(C)]
#[allow(dead_code)] // fields mirror what the cpu pushes, not all of them get read
struct InterruptStackFrame {
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

fn read_cs() -> u16 {
    let cs: u16;
    unsafe { asm!("mov {0:x}, cs", out(reg) cs, options(nomem, nostack, preserves_flags)) };
    cs
}

fn read_cr2() -> u64 {
    let addr: u64;
    unsafe { asm!("mov {}, cr2", out(reg) addr, options(nomem, nostack, preserves_flags)) };
    addr
}

fn halt_forever() -> ! {
    loop {
        unsafe { asm!("cli; hlt", options(nomem, nostack)) };
    }
}

extern "x86-interrupt" fn page_fault(_frame: InterruptStackFrame, error_code: u64) {
    let fault_addr = read_cr2();
    let page_not_present = error_code & 1 == 0;

    if page_not_present && heap::grow(fault_addr) {
        // a fresh frame is now mapped in, just retry the faulting instruction
        return;
    }

    println!(
        "page fault: addr={:#x} code={:#x} (present={} write={} user={})",
        fault_addr,
        error_code,
        error_code & 1 != 0,
        error_code & 2 != 0,
        error_code & 4 != 0,
    );
    halt_forever();
}

extern "x86-interrupt" fn general_protection(_frame: InterruptStackFrame, error_code: u64) {
    println!("general protection fault: code={:#x}", error_code);
    halt_forever();
}

extern "x86-interrupt" fn double_fault(_frame: InterruptStackFrame, _error_code: u64) -> ! {
    println!("double fault");
    halt_forever();
}

pub fn init() {
    let selector = read_cs();

    unsafe {
        let idt = &raw mut IDT;
        (*idt)[VEC_PAGE_FAULT].set_handler(page_fault as *const () as u64, selector);
        (*idt)[VEC_GENERAL_PROTECTION]
            .set_handler(general_protection as *const () as u64, selector);
        (*idt)[VEC_DOUBLE_FAULT].set_handler(double_fault as *const () as u64, selector);

        let descriptor = IdtPointer {
            limit: (core::mem::size_of::<[IdtEntry; 32]>() - 1) as u16,
            base: idt as u64,
        };
        asm!("lidt [{}]", in(reg) &descriptor, options(readonly, nostack, preserves_flags));
    }
}
