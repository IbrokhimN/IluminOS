// idt and irq foundation for networking

use core::arch::asm;
use crate::port::{inb, outb};

// pic master and slave controller ports
const PIC1_CMD: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_CMD: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;
const PIC_EOI: u8 = 0x20; // end of interrupt signal

// after remap hardware irqs sit at 0x20 to 0x2f in idt
const IRQ_BASE: u8 = 0x20;

// one idt entry format defined by x86-64
#[repr(C, packed)] // no padding bytes match hardware layout
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,   // low bits of handler address
    selector: u16,     // code segment
    ist: u8,           // interrupt stack table 0 means normal stack
    type_attr: u8,     // entry type 0x8e interrupt gate present
    offset_mid: u16,   // mid bits of address
    offset_high: u32,  // high bits of address
    zero: u32,         // reserved always 0
}

impl IdtEntry {
    const fn empty() -> Self {
        IdtEntry { offset_low:0, selector:0, ist:0, type_attr:0,
                   offset_mid:0, offset_high:0, zero:0 }
    }

    // write handler address into idt entry
    fn set_handler(&mut self, handler: u64, selector: u16) {
        self.offset_low  = (handler & 0xFFFF) as u16;
        self.offset_mid  = ((handler >> 16) & 0xFFFF) as u16;
        self.offset_high = ((handler >> 32) & 0xFFFFFFFF) as u32;
        self.selector = selector;
        self.ist = 0;
        self.type_attr = 0x8E; // present interrupt gate
        self.zero = 0;
    }
}

// table of 256 entries
static mut IDT: [IdtEntry; 256] = [IdtEntry::empty(); 256];

// pointer to idt for lidt instruction
#[repr(C, packed)]
struct IdtPointer {
    limit: u16, // table size minus 1
    base: u64,  // table address
}

// remap irqs from bios numbers to 0x20 plus
fn remap_pic() {
    unsafe {
        // save current masks
        let mask1 = inb(PIC1_DATA);
        let mask2 = inb(PIC2_DATA);

        // start init of both pics icw1
        outb(PIC1_CMD, 0x11);
        outb(PIC2_CMD, 0x11);
        // icw2 new base offset master 0x20 slave 0x28
        outb(PIC1_DATA, IRQ_BASE);
        outb(PIC2_DATA, IRQ_BASE + 8);
        // icw3 master slave wiring
        outb(PIC1_DATA, 4);
        outb(PIC2_DATA, 2);
        // icw4 8086 mode
        outb(PIC1_DATA, 0x01);
        outb(PIC2_DATA, 0x01);

        // restore masks
        outb(PIC1_DATA, mask1);
        outb(PIC2_DATA, mask2);
    }
}

// unmask a specific irq
pub fn unmask_irq(irq: u8) {
    unsafe {
        if irq < 8 {
            // master pic
            let mask = inb(PIC1_DATA);
            outb(PIC1_DATA, mask & !(1 << irq)); // clear bit to enable
        } else {
            // slave pic
            let mask = inb(PIC2_DATA);
            outb(PIC2_DATA, mask & !(1 << (irq - 8)));
        }
    }
}

// tell pic we handled it call at end of handler
pub fn send_eoi(irq: u8) {
    unsafe {
        if irq >= 8 {
            outb(PIC2_CMD, PIC_EOI); // notify slave too
        }
        outb(PIC1_CMD, PIC_EOI);
    }
}

// interrupt handler for the network card
extern "x86-interrupt" fn net_interrupt_handler(_frame: InterruptStackFrame) {
    // let driver handle the card event
    crate::tcp::rtl8139::handle_interrupt();
    // notify pic we are done
    send_eoi(11);
}

// interrupt stack frame layout pushed by cpu
#[repr(C)]
pub struct InterruptStackFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

// setup interrupts call at boot after framebuffer before net
pub fn init(net_irq: u8, code_selector: u16) {
    unsafe {
        // install card handler into idt by its index
        let idt_index = (IRQ_BASE + net_irq) as usize;
        let handler_addr = net_interrupt_handler as u64;
        let idt_ptr = core::ptr::addr_of_mut!(IDT);
        (*idt_ptr)[idt_index].set_handler(handler_addr, code_selector);

        // remap pic
        remap_pic();

        // load idt with lidt instruction
        let descriptor = IdtPointer {
            limit: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: idt_ptr as u64,
        };
        asm!("lidt [{}]", in(reg) &descriptor, options(readonly, nostack, preserves_flags));

        // unmask card irq and enable interrupts globally
        unmask_irq(net_irq);
        asm!("sti", options(nomem, nostack)); // start listening for interrupts
    }
}
