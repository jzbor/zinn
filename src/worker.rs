use std::fmt::Write;

use indicatif::ProgressBar;

use crate::queue::Queue;
use crate::Options;

struct BarMessageWriter(ProgressBar);
struct BarPrintWriter(ProgressBar);

impl Write for BarMessageWriter {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.set_message(s.to_owned());
        Ok(())
    }
}

impl Write for BarPrintWriter {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.println(s);
        Ok(())
    }
}


pub fn run_worker(queue: Queue, bar: ProgressBar, main_bar: ProgressBar, options: Options) {
    let mut status_writer = BarMessageWriter(bar.clone());
    let mut log_writer = BarPrintWriter(bar.clone());

    loop {
        bar.set_prefix("waiting...");
        bar.set_message("");
        if let Some(job) = queue.fetch() {
            bar.set_prefix(job.name().to_owned());
            if let Err(e) = job.run(&mut status_writer, &mut log_writer, &options) {
                bar.println(format!("[FAILED] {}: {}", job.name(), e));
                queue.failed(job);
            } else {
                bar.println(format!("[DONE] {}", job.name()));
                queue.finished(job);
            }
            main_bar.inc(1);
        } else {
            break;
        }
    }
}
