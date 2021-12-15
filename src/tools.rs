/// Поскольку в стандартную библиотеку языка не входит возможность отправки сигналов потокам
/// (т.к. Windows, например, не имеет такого понятия), то приходится вызывать libc самому.
pub fn send_signal(thread: libc::pthread_t, signal: i32) {
    unsafe {
        libc::pthread_kill(thread, signal);
    }
}
