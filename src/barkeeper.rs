use std::{fmt::Write, time::Duration};


struct BarMessageWriter(String, indicatif::ProgressBar);
struct BarPrintWriter(String, indicatif::ProgressBar);
struct DummyOutWriter();
struct DummyStatusWriter(String, String);


pub trait StateTracker {
    type ThreadStateTracker: ThreadStateTracker;
    fn set_njobs(&self, njobs: usize);
    fn start(&self);
    fn for_threads(&self, nthreads: usize) -> Vec<Self::ThreadStateTracker>;
}

pub trait ThreadStateTracker: Send {
    fn out(&mut self) -> &mut impl Write;
    fn status(&mut self) -> &mut impl Write;
    fn job_completed(&self);
    fn start(&self);
    fn set_prefix(&mut self, prefix: String);
}

pub struct Barkeeper {
    mp: indicatif::MultiProgress,
    bar: indicatif::ProgressBar,
}

pub struct ThreadBarkeeper {
    message_writer: BarMessageWriter,
    print_writer: BarPrintWriter,
    mp: indicatif::MultiProgress,
    bar: indicatif::ProgressBar,
    main_bar: indicatif::ProgressBar,
}

pub struct DummyBarkeeper {}
pub struct DummyThreadBarkeeper {
    out_writer: DummyOutWriter,
    status_writer: DummyStatusWriter,
}


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

impl StateTracker for Barkeeper {
    type ThreadStateTracker = ThreadBarkeeper;

    fn set_njobs(&self, njobs: usize) {
        self.bar.set_length(njobs as u64)
    }

    fn start(&self) {
        self.mp.add(self.bar.clone());
        self.bar.tick();
        self.bar.enable_steady_tick(Duration::from_millis(75));
    }

    fn for_threads(&self, nthreads: usize) -> Vec<ThreadBarkeeper> {
        (0..nthreads).map(|_| {
            let bar = indicatif::ProgressBar::new(1);
            bar.set_style(indicatif::ProgressStyle::with_template("{spinner} {prefix:.cyan} {wide_msg}").unwrap());

            ThreadBarkeeper {
                message_writer: BarMessageWriter(String::new(), bar.clone()),
                print_writer: BarPrintWriter(String::new(), bar.clone()),
                mp: self.mp.clone(),
                main_bar: self.bar.clone(),
                bar,
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
            let out_writer = DummyOutWriter();
            let status_writer = DummyStatusWriter(String::new(), String::new());
            DummyThreadBarkeeper { out_writer, status_writer }
        }).collect()
    }
}

impl ThreadStateTracker for DummyThreadBarkeeper {
    fn out(&mut self) -> &mut impl Write {
        &mut self.out_writer
    }

    fn status(&mut self) -> &mut impl Write {
        &mut self.status_writer
    }

    fn job_completed(&self) {}

    fn start(&self) {}

    fn set_prefix(&mut self, prefix: String) {
        self.status_writer.0 = prefix;
    }
}

impl ThreadStateTracker for ThreadBarkeeper {
    fn out(&mut self) -> &mut impl Write {
        &mut self.print_writer
    }

    fn status(&mut self) -> &mut impl Write {
        &mut self.message_writer
    }

    fn start(&self) {
        self.mp.add(self.bar.clone());
        self.bar.tick();
        self.bar.enable_steady_tick(Duration::from_millis(75));
    }

    fn job_completed(&self) {
        self.main_bar.inc(1)
    }

    fn set_prefix(&mut self, prefix: String) {
        self.bar.set_prefix(prefix)
    }
}


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

impl Write for DummyOutWriter {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        print!("{}", s);
        Ok(())
    }
}

impl Write for DummyStatusWriter {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for c in s.chars() {
            self.write_char(c)?
        }
        Ok(())
    }

    fn write_char(&mut self, c: char) -> std::fmt::Result {
        if c == '\n' {
            println!("{} {}", self.0, self.1);
            self.1.clear();
        } else {
            self.1.push(c);
        }
        Ok(())
    }
}
