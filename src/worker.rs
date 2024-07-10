use std::fmt::Write;

use crate::barkeeper::ThreadStateTracker;
use crate::queue::{JobState, Queue};
use crate::Options;

pub fn run_worker(queue: Queue, mut tracker: impl ThreadStateTracker, options: Options) {
    loop {
        tracker.set_prefix(String::from("waiting..."));
        tracker.clear_status();

        if let Some(job) = queue.fetch() {
            tracker.set_prefix(job.to_string());
            if let Ok(state) = job.run(&mut tracker, &options) {
                let msg = match state {
                    JobState::Finished => console::style(format!("=> DONE {}", job)).green().to_string(),
                    JobState::Skipped => console::style(format!("=> SKIPPED {}", job)).yellow().to_string(),
                    JobState::Failed => console::style(format!("=> FAILED {}", job)).red().to_string(),
                    _ => panic!("Invalid job state after run: {:?}", state),
                };
                let _ = writeln!(tracker.out(), "{}", msg);
                queue.finished(job);
            } else if let Err(e) = job.run(&mut tracker, &options) {
                let msg = console::style(format!("=> FAILED {}: {}", job, e)).red().to_string();
                let _ = writeln!(tracker.out(), "{}", msg);
                queue.failed(job);
            }
            tracker.job_completed();
        } else {
            break;
        }
    }
}
