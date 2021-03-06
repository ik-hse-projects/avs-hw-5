//! Watchman — следит за тем, чтобы число активных пользователей не превышало 50.
//! Для этого он каждые 100мс просмотривает список гостей в галерее,
//! при необходимости убирая из них неактивных.
//! Если после этого число гостей всё равно слишком велико, то выбирает случайного гостя
//! и просит его уйти, отправляя ему SIGTERM.

use std::{sync::atomic::Ordering, time::Duration};

use rand::Rng;

use crate::{gallery::Gallery, tools::send_signal};

const MAX_GUESTS_RUNNING: usize = 50;

pub fn start_watchman() {
    std::thread::Builder::new()
        .name("Watchman".to_string())
        .spawn(|| {
            let gallery = Gallery::global();
            loop {
                std::thread::sleep(Duration::from_millis(100));

                let guests = gallery.guests.read().unwrap();
                if guests.len() <= MAX_GUESTS_RUNNING {
                    // Happy path
                    continue;
                }
                std::mem::drop(guests);
                let mut guests = gallery.guests.write().unwrap();

                // Remove all inactive guests
                guests.retain(|x| {
                    x.upgrade()
                        .map(|x| x.running.load(Ordering::SeqCst))
                        .unwrap_or_default()
                });

                if guests.len() <= MAX_GUESTS_RUNNING {
                    // Still no one killed, just removed inactive guests
                    continue;
                }

                let idx = rand::thread_rng().gen_range(0..guests.len());
                let guest = guests.remove(idx);
                std::mem::drop(guests); // release the lock sooner

                let guest = match guest.upgrade() {
                    Some(x) => x,
                    None => continue,
                };
                if !guest.running.load(Ordering::SeqCst) {
                    continue;
                }
                let &pid = guest.pid.get().unwrap();
                send_signal(pid, libc::SIGINT);
            }
        })
        .unwrap();
}
