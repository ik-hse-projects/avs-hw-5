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
    pub fn global() -> &'static Self {
        static mut GALLERY: SyncOnceCell<Gallery> = SyncOnceCell::new();
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
