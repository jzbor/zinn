use std::fmt::Write;

use indicatif::ProgressBar;

use crate::queue::Queue;
use crate::Options;

struct BarMessageWriter(String, ProgressBar);
struct BarPrintWriter(String, ProgressBar);

impl Write for BarMessageWriter {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for c in s.chars() {
            self.write_char(c)?
        }
        Ok(())
    }

    fn write_char(&mut self, c: char) -> std::fmt::Result {
        if c == '\n' {
            let msg = self.0.clone();
            self.1.set_message(msg);
            self.0 = String::new();
        } else {
            self.0.push(c);
        }
        Ok(())
    }
}

impl Write for BarPrintWriter {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for c in s.chars() {
            self.write_char(c)?
        }
        Ok(())
    }

    fn write_char(&mut self, c: char) -> std::fmt::Result {
        if c == '\n' {
            self.1.println(&self.0);
            self.0.clear();
        } else {
            self.0.push(c)
        }
        Ok(())
    }
}

pub fn run_worker(queue: Queue, bar: ProgressBar, main_bar: ProgressBar, options: Options) {
    let mut status_writer = BarMessageWriter(String::new(), bar.clone());
    let mut log_writer = BarPrintWriter(String::new(), bar.clone());

    loop {
        bar.set_prefix("waiting...");
        bar.set_message("");
        if let Some(job) = queue.fetch() {
            bar.set_prefix(job.to_string());
            if let Err(e) = job.run(&mut status_writer, &mut log_writer, &options) {
                bar.println(console::style(format!("=> FAILED {}: {}", job, e)).red().to_string());
                queue.failed(job);
            } else {
                bar.println(console::style(format!("=> DONE {}", job)).cyan().to_string());
                queue.finished(job);
            }
            main_bar.inc(1);
        } else {
            break;
        }
    }
}
