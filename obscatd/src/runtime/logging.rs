use std::fs::File;
use std::io::{BufRead, BufReader};
use std::{os::fd::FromRawFd, sync::OnceLock};
use tokio::sync::broadcast;

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";

pub const CYAN: &str = "\x1b[36m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const RED: &str = "\x1b[31m";

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        println!(
            "{}{}[OBSCAT]{} {}",
            $crate::runtime::logging::BOLD,
            $crate::runtime::logging::CYAN,
            $crate::runtime::logging::RESET,
            format!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! log_success {
    ($($arg:tt)*) => {
        println!(
            "{}{}[OBSCAT]{} {}✔{} {}",
            $crate::runtime::logging::BOLD,
            $crate::runtime::logging::GREEN,
            $crate::runtime::logging::RESET,
            $crate::runtime::logging::GREEN,
            $crate::runtime::logging::RESET,
            format!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        println!(
            "{}{}[OBSCAT]{} {}⚠ {}{}",
            $crate::runtime::logging::BOLD,
            $crate::runtime::logging::CYAN,
            $crate::runtime::logging::RESET,
            $crate::runtime::logging::YELLOW,
            format!($($arg)*),
            $crate::runtime::logging::RESET
        )
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        eprintln!(
            "{}{}[OBSCAT]{} {}✖ {}{}",
            $crate::runtime::logging::BOLD,
            $crate::runtime::logging::RED,
            $crate::runtime::logging::RESET,
            $crate::runtime::logging::RED,
            msg,
            $crate::runtime::logging::RESET
        );
        msg
    }};
}

#[macro_export]
macro_rules! log_step {
    ($id:expr, $($arg:tt)*) => {
        println!(
            "{}{}[OBSCAT]{} {}[{}]{} {}",
            $crate::runtime::logging::BOLD,
            $crate::runtime::logging::CYAN,
            $crate::runtime::logging::RESET,
            $crate::runtime::logging::DIM,
            $id,
            $crate::runtime::logging::RESET,
            format!($($arg)*)
        )
    };
}

// debug capture
static LOG_TX: OnceLock<broadcast::Sender<String>> = OnceLock::new();
pub fn subscribe_logs() -> broadcast::Receiver<String> {
    LOG_TX
        .get()
        .expect("Global log capture belum diinisialisasi")
        .subscribe()
}
pub fn init_global_capture() {
    let (tx, _) = broadcast::channel::<String>(1024);
    let _ = LOG_TX.set(tx.clone());

    unsafe {
        let mut fds = [0i32; 2];
        if libc::pipe(fds.as_mut_ptr()) != 0 {
            return;
        }
        let read_fd = fds[0];
        let write_fd = fds[1];

        let orig_stderr = libc::dup(libc::STDERR_FILENO);

        libc::dup2(write_fd, libc::STDOUT_FILENO);
        libc::dup2(write_fd, libc::STDERR_FILENO);
        libc::close(write_fd);

        std::thread::spawn(move || {
            let file = File::from_raw_fd(read_fd);
            let reader = BufReader::new(file);

            for line in reader.lines().map_while(Result::ok) {
                if orig_stderr >= 0 {
                    let msg = format!("{}\n", line);
                    libc::write(orig_stderr, msg.as_ptr() as *const _, msg.len());
                }
                let _ = tx.send(line);
            }
        });
    }
}
