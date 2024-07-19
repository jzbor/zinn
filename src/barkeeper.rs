use crate::{queue::JobState, JobRealization, ZinnError};


pub trait StateTracker {
    type ThreadStateTracker: ThreadStateTracker;
    fn set_njobs(&self, njobs: usize);
    fn start(&self);
    fn for_threads(&self, nthreads: usize) -> Vec<Self::ThreadStateTracker>;
}

pub trait ThreadStateTracker: Send {
    fn job_completed(&self, job: JobRealization, state: JobState, error: Option<ZinnError>);
    fn start(&self);
    fn set_prefix(&mut self, prefix: String);
    fn clear_status(&mut self);
    fn cmd_output(&mut self, job: &str, out: &str, verbose: bool);
    fn flush_cmd_output(&mut self, job: &str, verbose: bool);
    fn trace(&mut self, cmd: &str);
}

#[cfg(feature = "progress")]
pub struct Barkeeper {
    mp: indicatif::MultiProgress,
    bar: indicatif::ProgressBar,
}

#[cfg(feature = "progress")]
pub struct ThreadBarkeeper {
    mp: indicatif::MultiProgress,
    bar: indicatif::ProgressBar,
    main_bar: indicatif::ProgressBar,
    last_line: Option<String>,
}

pub struct DummyBarkeeper {}
pub struct DummyThreadBarkeeper {}


#[cfg(feature = "progress")]
impl Barkeeper {
    pub fn new() -> Self {
        let mp = indicatif::MultiProgress::new();
        let bar_style = indicatif::ProgressStyle::with_template("[{elapsed}] {wide_bar} {pos}/{len}").unwrap();
        let bar = indicatif::ProgressBar::new(1);
        bar.set_style(bar_style);

        Barkeeper { mp, bar }
    }
}

impl DummyBarkeeper {
    pub fn new() -> Self {
        DummyBarkeeper {}
    }
}

#[cfg(feature = "progress")]
impl StateTracker for Barkeeper {
    type ThreadStateTracker = ThreadBarkeeper;

    fn set_njobs(&self, njobs: usize) {
        self.bar.set_length(njobs as u64)
    }

    fn start(&self) {
        self.mp.add(self.bar.clone());
        self.bar.tick();
        self.bar.enable_steady_tick(std::time::Duration::from_millis(75));
    }

    fn for_threads(&self, nthreads: usize) -> Vec<ThreadBarkeeper> {
        (0..nthreads).map(|_| {
            let bar = indicatif::ProgressBar::new(1);
            bar.set_style(indicatif::ProgressStyle::with_template("{spinner} {prefix:.cyan} {wide_msg}").unwrap());

            ThreadBarkeeper {
                mp: self.mp.clone(),
                main_bar: self.bar.clone(),
                bar,
                last_line: None,
            }

        }).collect()
    }
}

impl StateTracker for DummyBarkeeper {
    type ThreadStateTracker = DummyThreadBarkeeper;

    fn set_njobs(&self, _njobs: usize) {}

    fn start(&self) {}

    fn for_threads(&self, nthreads: usize) -> Vec<DummyThreadBarkeeper> {
        (0..nthreads).map(|_| {
            DummyThreadBarkeeper {}
        }).collect()
    }
}

impl ThreadStateTracker for DummyThreadBarkeeper {
    fn job_completed(&self, job: JobRealization, state: JobState, error: Option<ZinnError>) {
        println!("{}", job_finished_msg(job, state));
        if let Some(e) = error {
            println!("{}", e);
        }
    }

    fn start(&self) {}

    fn set_prefix(&mut self, _prefix: String) {}

    fn clear_status(&mut self) {}

    fn cmd_output(&mut self, job: &str, out: &str, _verbose: bool) {
        println!("{}: {}", job, out);
    }

    fn flush_cmd_output(&mut self, _job: &str, _verbose: bool) {}

    fn trace(&mut self, cmd: &str) {
        println!("{}", cmd);
    }
}

#[cfg(feature = "progress")]
impl ThreadStateTracker for ThreadBarkeeper {
    fn start(&self) {
        self.mp.add(self.bar.clone());
        self.bar.tick();
        self.bar.enable_steady_tick(std::time::Duration::from_millis(75));
    }

    fn job_completed(&self, job: JobRealization, state: JobState, error: Option<ZinnError>) {
        self.bar.println(job_finished_msg(job, state));
        if let Some(e) = error {
            self.bar.println(e.to_string());
        }
        self.main_bar.inc(1)
    }

    fn set_prefix(&mut self, prefix: String) {
        self.bar.set_prefix(prefix)
    }

    fn clear_status(&mut self) {
        self.bar.set_message("");
    }

    fn cmd_output(&mut self, job: &str, out: &str, verbose: bool) {
        self.bar.set_message(out.to_owned());

        if verbose {
            if let Some(line) = self.last_line.take() {
                self.bar.println(format!("{}: {}", job, line));
            }
            self.last_line = Some(out.to_owned());
        }
    }

    fn flush_cmd_output(&mut self, job: &str, verbose: bool) {
        if verbose {
            if let Some(line) = self.last_line.take() {
                self.bar.println(format!("{}: {}", job, line));
            }
        }
    }

    fn trace(&mut self, cmd: &str) {
        self.bar.println(cmd);
    }
}


fn job_finished_msg(job: JobRealization, state: JobState ) -> String {
    match state {
        JobState::Finished => console::style(format!("=> DONE {}", job)).green().to_string(),
        JobState::Skipped => console::style(format!("=> SKIPPED {}", job)).yellow().to_string(),
        JobState::Failed => console::style(format!("=> FAILED {}", job)).red().to_string(),
        _ => panic!("Invalid job state after run: {:?}", state),
    }
}
