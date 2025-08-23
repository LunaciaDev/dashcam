use std::sync::{atomic::{AtomicBool, Ordering}, Arc, LazyLock};

pub static HALT_FLAG: LazyLock<Arc<AtomicBool>> =
    LazyLock::new(|| Arc::new(AtomicBool::new(false)));

pub fn is_halt() -> bool {
    HALT_FLAG.load(Ordering::Relaxed)
}

pub fn set_halt() {
    HALT_FLAG.store(true, Ordering::Relaxed);
}