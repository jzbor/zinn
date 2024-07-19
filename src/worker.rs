use crate::barkeeper::ThreadStateTracker;
use crate::queue::{JobState, Queue};
use crate::Options;

pub fn run_worker(queue: Queue, mut tracker: impl ThreadStateTracker, options: Options) {
    loop {
        tracker.set_prefix(String::from("waiting..."));
        tracker.clear_status();

        if let Some(job) = queue.fetch() {
            tracker.set_prefix(job.to_string());
            let result = job.run(&mut tracker, &options);
            let state = match &result {
                Ok(state) => *state,
                Err(_) => JobState::Failed,
            };
            tracker.job_completed(job.clone(), state, result.err());
            queue.finished(job, state);
        } else {
            break;
        }
    }
}
