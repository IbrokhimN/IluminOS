// memory
pub mod allocator;
pub mod fault;
pub mod heap;
pub mod paging;

pub fn init() {
    allocator::init();
    paging::init();
    fault::init();
    heap::init();
}
