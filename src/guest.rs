//! Гость — самый сложный поток в этом задании.
//! Во-первых, он устаналивает обработчик сигнала SIGTERM, который устаналивает флаг остановки
//! Во-вторых, он добавлят свой поток в список гостей галерии.
//! В-третьих, он внутри потока запускает цикл (пока флаг не сбросится), в котором
//! либо переходит к другой картине, либо выходит из галереи, либо (чаще всего) ничего не делает.
//! 
//! Каждый гость имеет свой уникальный идентификатор, что позволяет их с лёгкостью отличать.

use rand::Rng;
use std::{
    lazy::{OnceCell, SyncOnceCell},
    os::unix::prelude::JoinHandleExt,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Weak,
    },
    time::Duration,
};

use crate::{atomic_ref::AtomicRef, gallery::Gallery, painting::Painting};

const MAX_GUESTS_PER_PAINTING: usize = 10;

/// Счётчик гостей в программе. Используется для генерации уникальных id.
static GUEST_COUNTER: AtomicUsize = AtomicUsize::new(1);

/// Thread-local переменная, содержащую умный указатель на гостя,
/// исполняющегося в этом потоке.
#[thread_local]
static CURRENT_GUEST: OnceCell<Arc<Guest>> = OnceCell::new();

#[derive(Debug)]
pub struct Guest {
    pub guest_id: usize,
    /// pthread_t этого потока. Инициализируется сразу после запуска.
    pub pid: SyncOnceCell<u64>,
    /// Флаг того, исполняется ли поток на данный момент
    pub running: AtomicBool,
    /// Указатель на текущую картину, если таковая есть.
    pub painting: AtomicRef<'static, Painting>,
}

/// Устанавливает обработчик сигнала SIGTERM для текщуего потока
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
            pid: SyncOnceCell::new(),
        };
        let guest = Arc::new(guest);
        let result = Arc::clone(&guest);
        let handle = std::thread::Builder::new()
            .name(format!("Guest #{}", guest_id))
            .spawn(|| {
                // Устаналиваем thread_local:
                CURRENT_GUEST.set(guest).unwrap();

                // Настраиваем SIGTERM
                setup_signal();

                // Наконец, запускаем main_loop.
                CURRENT_GUEST.get().unwrap().main_loop();
            })
            .unwrap();

        result.pid.get_or_init(|| handle.as_pthread_t());

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
        // Если мы сейчас находимся в какой-то картине, то обязательно от неё уходим
        // и не забываем уменьшить счётчик гостей перед этой картиной.
        if let Some(painting) = self.painting.swap(None, Ordering::SeqCst) {
            painting.watching.fetch_sub(1, Ordering::SeqCst);
        }

        // Затем начинаем пытаться подойти к выбранной картине
        while self.running.load(Ordering::Acquire) {
            // fetch_update позволяет атомарно провести нетривиальное изменение количества гостей
            // перед этой картиной: увеличить число, но только если оно строго меньше максимального
            let upd = painting
                .watching
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |val| {
                    if val < MAX_GUESTS_PER_PAINTING {
                        Some(val + 1)
                    } else {
                        None
                    }
                });
            // Если всё прошло успешно: место было и мы смогли его занять, то завершаем переход.
            if upd.is_ok() {
                self.painting.store(Some(painting), Ordering::SeqCst);
                break;
            }
            // Иначе чуть-чуть ждём и пробуем ещё раз.
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
