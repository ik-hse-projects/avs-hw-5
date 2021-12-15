#![allow(dead_code)]
#![feature(inline_const, once_cell, thread_local, exclusive_range_pattern)]

use std::{
    io::Write,
    os::unix::prelude::FromRawFd, time::Duration,
};

mod atomic_ref;
mod guest;
mod painting;
mod watchman;
mod gallery;
mod tools;

fn main() {
    let mut stdout = unsafe { std::fs::File::from_raw_fd(1) };
    let gallery = gallery::Gallery::global();
    watchman::start_watchman();
    loop {
        write!(stdout, "\x1b[2J").unwrap();

        writeln!(stdout, "Paintings:").unwrap();
        for painting in &gallery.paintings {
            writeln!(stdout, "    {:?}", painting).unwrap();
        }

        let guests = gallery.guests.read().unwrap();
        writeln!(stdout, "Guests ({}):", guests.len()).unwrap();
        for guest in guests.iter() {
            if let Some(existing) = guest.upgrade() {
                writeln!(stdout, "    {:?}", existing).unwrap();
            }
        }

        stdout.flush().unwrap();
        std::thread::sleep(Duration::from_millis(250));
    }
}
