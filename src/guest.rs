use rand::Rng;
use std::{
    lazy::OnceCell,
    os::unix::prelude::JoinHandleExt,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Weak,
    },
    time::Duration,
};

use crate::{atomic_ref::AtomicRef, gallery::Gallery, painting::Painting};

const MAX_GUESTS_PER_PAINTING: usize = 10;

static GUEST_COUNTER: AtomicUsize = AtomicUsize::new(1);

#[thread_local]
static CURRENT_GUEST: OnceCell<Arc<Guest>> = OnceCell::new();

#[derive(Debug)]
pub struct Guest {
    pub guest_id: usize,
    pub pid: AtomicU64,
    pub running: AtomicBool,
    pub painting: AtomicRef<'static, Painting>,
}

fn setup_signal() {
    extern "C" fn handler(_signum: i32) {
        if let Some(current) = CURRENT_GUEST.get() {
            current.running.store(false, Ordering::Release);
        }
        setup_signal();
    }

    unsafe {
        libc::signal(libc::SIGINT, handler as usize);
    }
}

/*
 * Вопрос: кто ответственнен за освобождение памяти за Guest?
 * Если это поток гостя, то он может заершиться раньше, чем основной поток.
 * Если это основной поток, то он освободить гостя раньше, чем его завершится его поток.
 * Поэтому гость должен быть освобождён только тогда, когда он не нужен ни одному из потоков.
 * Эту задачу решает atomic reference counting (Arc).
 */

impl Guest {
    pub fn start() -> Weak<Guest> {
        let guest_id = GUEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let guest = Guest {
            guest_id,
            running: AtomicBool::new(true),
            painting: AtomicRef::new(None),
            pid: AtomicU64::new(0),
        };
        let guest = Arc::new(guest);
        let result = Arc::clone(&guest);
        let handle = std::thread::Builder::new()
            .name(format!("Guest #{}", guest_id))
            .spawn(|| {
                CURRENT_GUEST.set(guest).unwrap();
                setup_signal();
                CURRENT_GUEST.get().unwrap().main_loop();
            })
            .unwrap();

        let pid = handle.as_pthread_t();
        result.pid.store(pid, Ordering::Relaxed);

        Arc::downgrade(&result)
    }

    fn main_loop(&self) {
        while self.running.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(100));
            let rnd: f64 = rand::random();
            if rnd < 0.10 {
                // 10% chance to change painting
                let paintings = &Gallery::global().paintings;
                let index = rand::thread_rng().gen_range(0..paintings.len());
                self.go_to_painting(&paintings[index])
            } else if rnd < 0.10 + 0.0009 {
                // 0.09% chance to leave
                self.running.store(false, Ordering::Release);
            } else {
                // ≈90% to do nothing
            }
        }
        if let Some(painting) = self.painting.swap(None, Ordering::SeqCst) {
            painting.watching.fetch_sub(1, Ordering::SeqCst);
        }   
    }

    fn go_to_painting(&self, painting: &'static Painting) {
        if let Some(painting) = self.painting.swap(None, Ordering::SeqCst) {
            painting.watching.fetch_sub(1, Ordering::SeqCst);
        }
        while self.running.load(Ordering::Acquire) {
            let upd = painting
                .watching
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |val| {
                    if val < MAX_GUESTS_PER_PAINTING {
                        Some(val + 1)
                    } else {
                        None
                    }
                });
            if upd.is_ok() {
                self.painting.store(Some(painting), Ordering::SeqCst);
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
