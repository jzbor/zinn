use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use std::{fs, thread};
use indicatif::MultiProgress;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;

use error::*;
use job::*;


mod error;
mod job;
mod worker;


#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, default_value_t = String::from("zinn.yml"))]
    zinnfile: String,

    #[clap(short, long, default_value_t = 4)]
    jobs: usize,

    #[clap(short, long)]
    verbose: bool,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
struct Zinnfile {
    jobs: HashMap<String, JobDescription>,
}


fn main() {
    let args = Args::parse();

    let contents = resolve(fs::read_to_string(args.zinnfile));
    let zinnfile: Zinnfile = resolve(serde_yaml::from_str(&contents));

    let mp = MultiProgress::new();
    let main_bar = ProgressBar::new(zinnfile.jobs.len() as u64);

    let (job_tx, job_rx) = crossbeam::channel::unbounded();

    let threads: Vec<_> = (0..args.jobs).map(|_| {
        let bar = ProgressBar::new(10000000);
        bar.set_style(ProgressStyle::with_template("{spinner} {prefix:.cyan} {msg}").unwrap());
        bar.enable_steady_tick(Duration::from_millis(75));
        mp.add(bar.clone());
        let job_rx = job_rx.clone();
        let main_bar = main_bar.clone();
        let verbose = args.verbose;

        thread::spawn(move || {
            worker::run_worker(job_rx, bar, main_bar, verbose)
        })
    }).collect();

    mp.add(main_bar.clone());

    for (name, job) in &zinnfile.jobs {
        let job = job.realize(name);
        job_tx.send(job).unwrap();
    }

    drop(job_tx);
    for thread in threads {
        let _ = thread.join();
    }
}
