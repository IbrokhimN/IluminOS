use linked_list_allocator::LockedHeap;

const HEAP_SIZE: usize = 1024 * 1024;

#[repr(align(16))]
#[allow(dead_code)]
struct HeapMem([u8; HEAP_SIZE]);
static mut HEAP: HeapMem = HeapMem([0; HEAP_SIZE]);

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init() {
    unsafe {
        let start = &raw const HEAP as usize;
        ALLOCATOR.lock().init(start as *mut u8, HEAP_SIZE);
    }
}

// сколько всего байт в куче (для команды df/mem)
pub fn heap_size() -> usize {
    HEAP_SIZE
}

// сколько занято/свободно прямо сейчас
pub fn heap_used() -> usize {
    ALLOCATOR.lock().used()
}

pub fn heap_free() -> usize {
    ALLOCATOR.lock().free()
}
