use std::sync::atomic::AtomicUsize;

/// Картина хранит в себе количество гостей, которые на неё смотрят
/// и её имя для красивого отображения.
#[derive(Debug)]
pub struct Painting {
    pub watching: AtomicUsize,
    pub name: &'static str,
}

impl Painting {
    pub const fn new(name: &'static str) -> Self {
        Painting {
            watching: AtomicUsize::new(0),
            name
        }
    }
}
