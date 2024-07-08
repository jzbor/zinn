use std::{fmt::Write, time::Duration};


struct BarMessageWriter(String, indicatif::ProgressBar);
struct BarPrintWriter(String, indicatif::ProgressBar);


pub trait StateTracker {
    fn set_njobs(&self, njobs: usize);
    fn start(&self);
    fn for_threads(&self, nthreads: usize) -> Vec<impl ThreadStateTracker>;
}

pub trait ThreadStateTracker {
    fn out(&mut self) -> &mut impl Write;
    fn status(&mut self) -> &mut impl Write;
    fn job_completed(&self);
    fn start(&self);
    fn set_prefix(&self, prefix: String);
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


impl Barkeeper {
    pub fn new() -> Self {
        let mp = indicatif::MultiProgress::new();
        let bar_style = indicatif::ProgressStyle::with_template("[{elapsed}] {wide_bar} {pos}/{len}").unwrap();
        let bar = indicatif::ProgressBar::new(1);
        bar.set_style(bar_style);

        Barkeeper { mp, bar }
    }
}

impl StateTracker for Barkeeper {
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

    fn set_prefix(&self, prefix: String) {
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
