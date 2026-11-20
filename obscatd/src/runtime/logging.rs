use std::{os::fd::FromRawFd, sync::OnceLock};
use std::fs::File;
use std::io::{BufRead, BufReader};
use tokio::{sync::broadcast};

// #[derive(Default, Clone)]
// pub struct LogsCollector {
//     lines: Vec<String>,
// }

// impl LogsCollector {
//     pub fn new() -> Self {
//         Self { lines: Vec::new() }
//     }

//     pub fn info(&mut self, msg: impl AsRef<str>) {
//         let text = msg.as_ref();
//         println!("{}", text);
//         self.lines.push(text.to_string());
//     }

//     pub fn error(&mut self, msg: impl AsRef<str>) {
//         let text = format!("ERROR: {}", msg.as_ref());
//         println!("{}", text);
//         self.lines.push(text.to_string());
//     }

//     pub fn flush(&self) -> String {
//         self.lines.join("\n")
//     }
// }

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
