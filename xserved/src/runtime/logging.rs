#[derive(Default, Clone)]
pub struct LogsCollector {
    lines: Vec<String>
}

impl LogsCollector {
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    pub fn info(&mut self, msg: impl AsRef<str>) {
        let text = msg.as_ref();
        println!("{}", text);
        self.lines.push(text.to_string());
    }


    pub fn error(&mut self, msg: impl AsRef<str>) {
        let text = format!("ERROR: {}", msg.as_ref());
        println!("{}", text);
        self.lines.push(text.to_string());
    }

    pub fn flush(&self) -> String {
        self.lines.join("\n")
    }
}

