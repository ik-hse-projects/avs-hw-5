pub fn send_signal(thread: libc::pthread_t, signal: i32) {
    unsafe {
        libc::pthread_kill(thread, signal);
    }
}
