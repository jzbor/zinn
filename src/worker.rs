use crate::barkeeper::ThreadStateTracker;
use crate::queue::{JobState, Queue};
use crate::Options;


const MAX_PREFIX_LEN: usize = 60;


pub fn run_worker(queue: Queue, mut tracker: impl ThreadStateTracker, options: Options) {
    loop {
        tracker.set_prefix(String::from("waiting..."));
        tracker.clear_status();

        if let Some(job) = queue.fetch() {
            let prefix = console::style(truncate_ellipse(job.to_string(), MAX_PREFIX_LEN)).cyan().to_string();
            tracker.set_prefix(prefix);
            // tracker.set_prefix(job.to_string());
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

fn truncate_ellipse(mut string: String, max_size: usize) -> String {
    if string.len() > max_size {
        // UTF-8 is weird...
        let bytes = match string.char_indices().nth(max_size - 3) {
            None => string.len(),
            Some((idx, _)) => string[..idx].len(),
        };

        string.truncate(bytes);
        string.push_str("...");
    }

    string
}
