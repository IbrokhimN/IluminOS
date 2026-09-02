// прерывания IDT + IRQ фундамент для сети

use core::arch::asm;
use crate::port::{inb, outb};

// порты контроллеров прерываний PIC master и slave
const PIC1_CMD: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_CMD: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;
const PIC_EOI: u8 = 0x20; // End Of Interrupt сигнал я обработал

// после ремапа аппаратные IRQ занимают 0x20..0x2F в IDT
const IRQ_BASE: u8 = 0x20;

// запись IDT одна строка таблицы формат задан x86-64
#[repr(C, packed)] // без выравнивания байты как ждёт железо
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,   // младшие биты адреса обработчика
    selector: u16,     // сегмент кода
    ist: u8,           // Interrupt Stack Table 0 обычный стек
    type_attr: u8,     // тип записи 0x8E interrupt gate present
    offset_mid: u16,   // средние биты адреса
    offset_high: u32,  // старшие биты адреса
    zero: u32,         // зарезервировано всегда 0
}

impl IdtEntry {
    const fn empty() -> Self {
        IdtEntry { offset_low:0, selector:0, ist:0, type_attr:0,
                   offset_mid:0, offset_high:0, zero:0 }
    }

    // записать адрес обработчика в запись IDT
    fn set_handler(&mut self, handler: u64, selector: u16) {
        self.offset_low  = (handler & 0xFFFF) as u16;
        self.offset_mid  = ((handler >> 16) & 0xFFFF) as u16;
        self.offset_high = ((handler >> 32) & 0xFFFFFFFF) as u32;
        self.selector = selector;
        self.ist = 0;
        self.type_attr = 0x8E; // present + interrupt gate
        self.zero = 0;
    }
}

// таблица на 256 записей
static mut IDT: [IdtEntry; 256] = [IdtEntry::empty(); 256];

// указатель на IDT для инструкции lidt
#[repr(C, packed)]
struct IdtPointer {
    limit: u16, // размер таблицы минус 1
    base: u64,  // адрес таблицы
}

// переназначить IRQ с системных номеров на 0x20+
fn remap_pic() {
    unsafe {
        // сохранить текущие маски
        let mask1 = inb(PIC1_DATA);
        let mask2 = inb(PIC2_DATA);

        // начать инициализацию обоих PIC ICW1
        outb(PIC1_CMD, 0x11);
        outb(PIC2_CMD, 0x11);
        // ICW2 новый базовый номер master 0x20 slave 0x28
        outb(PIC1_DATA, IRQ_BASE);
        outb(PIC2_DATA, IRQ_BASE + 8);
        // ICW3 связь master и slave
        outb(PIC1_DATA, 4);
        outb(PIC2_DATA, 2);
        // ICW4 режим 8086
        outb(PIC1_DATA, 0x01);
        outb(PIC2_DATA, 0x01);

        // вернуть маски
        outb(PIC1_DATA, mask1);
        outb(PIC2_DATA, mask2);
    }
}

// разрешить конкретный IRQ
pub fn unmask_irq(irq: u8) {
    unsafe {
        if irq < 8 {
            // master PIC
            let mask = inb(PIC1_DATA);
            outb(PIC1_DATA, mask & !(1 << irq)); // сбросить бит разрешить
        } else {
            // slave PIC
            let mask = inb(PIC2_DATA);
            outb(PIC2_DATA, mask & !(1 << (irq - 8)));
        }
    }
}

// сказать PIC я обработал обязательно в конце обработчика
pub fn send_eoi(irq: u8) {
    unsafe {
        if irq >= 8 {
            outb(PIC2_CMD, PIC_EOI); // slave тоже уведомить
        }
        outb(PIC1_CMD, PIC_EOI);
    }
}

// обработчик прерывания от сетевой карты
extern "x86-interrupt" fn net_interrupt_handler(_frame: InterruptStackFrame) {
    // сказать драйверу разобрать событие карты
    crate::tcp::rtl8139::handle_interrupt();
    // уведомить PIC что закончили
    send_eoi(11);
}

// заглушка типа кадра прерывания его кладёт процессор на стек
#[repr(C)]
pub struct InterruptStackFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

// настроить прерывания вызвать при загрузке после framebuffer до сети
pub fn init(net_irq: u8, code_selector: u16) {
    unsafe {
        // поставить обработчик карты в IDT по её номеру
        let idt_index = (IRQ_BASE + net_irq) as usize;
        let handler_addr = net_interrupt_handler as u64;
        let idt_ptr = core::ptr::addr_of_mut!(IDT);
        (*idt_ptr)[idt_index].set_handler(handler_addr, code_selector);

        // ремапнуть PIC
        remap_pic();

        // загрузить IDT инструкцией lidt
        let descriptor = IdtPointer {
            limit: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: idt_ptr as u64,
        };
        asm!("lidt [{}]", in(reg) &descriptor, options(readonly, nostack, preserves_flags));

        // разрешить IRQ карты и включить прерывания глобально
        unmask_irq(net_irq);
        asm!("sti", options(nomem, nostack)); // слушаю прерывания
    }
}
