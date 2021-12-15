use std::{
    lazy::SyncOnceCell,
    num::NonZeroU64,
    sync::{
        atomic::{AtomicBool, Ordering},
        RwLock, Weak,
    },
    time::Duration,
};

use crate::{guest::Guest, painting::Painting};

#[derive(Debug)]
pub struct Gallery {
    pub guests: RwLock<Vec<Weak<Guest>>>,
    pub paintings: [Painting; 5],
    pub watchman: Option<NonZeroU64>,
    pub thread_started: AtomicBool,
}

impl Gallery {
    /// Этот метод возвращает статический экземпляр галлереи,
    /// запуская поток сторожа при необходимости
    pub fn global() -> &'static Self {
        static mut GALLERY: SyncOnceCell<Gallery> = SyncOnceCell::new();
        // Изменение статической переменной небезопасно, но:
        // 1. SyncOnceCell гарантирует, что мы его инициализируем только один раз и полностью.
        // 2. start_adder устроен таким образом, что он выполнится ровно один раз для каждой,
        //     галереи причём это обновление соответсвующего флага происходит атомарно.
        // Поэтому здесь всё корректно и никаких гонок не может возникнуть.
        unsafe {
            let res = GALLERY.get_or_init(Gallery::new);
            GALLERY.get_mut().unwrap().start_adder();
            res
        }
    }

    fn new() -> Self {
        Gallery {
            guests: RwLock::new(Vec::new()),
            paintings: [
                Painting::new("Mona Lisa"),
                Painting::new("Girl with a Pearl"),
                Painting::new("The Starry Night"),
                Painting::new("The Arnolfini Portrait"),
                Painting::new("Sandro Botticelli")
            ],
            watchman: None,
            thread_started: AtomicBool::new(false),
        }
    }

    /// Запускает поток, который будет время от времени добавлять новых гостей в галерею.
    fn start_adder(&'static mut self) {
        if self.thread_started.fetch_or(true, Ordering::SeqCst) {
            return;
        }

        std::thread::Builder::new()
            .name("Gallery thread".to_string())
            .spawn(|| loop {
                std::thread::sleep(Duration::from_millis(100));
                if rand::random::<f64>() > 0.15 {
                    continue;
                }

                let new_guest = Guest::start();
                let mut guests = self.guests.write().unwrap();
                guests.push(new_guest);
            })
            .unwrap();
    }
}
