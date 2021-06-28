use std::sync::atomic::{AtomicBool, Ordering};

// TODO(#23): ctrlc module is not implemented for windows
// It's not that important right now, since ncurses crate already prevents it from working properly
// on windows anyway.
#[cfg(not(unix))]
compile_error! {"Windows is not supported right now"}

// We are just trying to flip a bunch of bits in a single-threaded environment with no plans of
// making it multi-threaded. No need to make it overcomplicated. Just a single atomic bool with
// relaxed ordering should be enough.
static CTRLC: AtomicBool = AtomicBool::new(false);

extern "C" fn callback(_signum: i32) {
    CTRLC.store(true, Ordering::Relaxed);
}

pub fn init() {
    unsafe {
        // TODO(#24): Use sigaction(2) instead of signal(2) for better potability
        // See signal(2) Portability section. Though for our specific case of flipping some bits on
        // SIGINT this might not be that important.
        if libc::signal(libc::SIGINT, callback as libc::sighandler_t) == libc::SIG_ERR {
            // signal(2) usually fails when the first argument is invalid. This means we are
            // on a really weird UNIX or there is a bug in libc crate.
            unreachable!()
        }
    }
}

pub fn poll() -> bool {
    CTRLC.swap(false, Ordering::Relaxed)
}
