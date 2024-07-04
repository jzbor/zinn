use clap::Parser;
use handlebars::Handlebars;
use queue::Queue;
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
mod queue;


#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, default_value_t = String::from("zinn.yml"))]
    zinnfile: String,

    #[clap(default_values_t = [String::from("default")])]
    targets: Vec<String>,

    #[clap(short, long, default_value_t = 4)]
    jobs: usize,

    #[clap(short, long)]
    verbose: bool,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
struct Zinnfile {
    #[serde(default)]
    constants: HashMap<String, String>,

    jobs: HashMap<String, JobDescription>,
}


fn main() {
    let args = Args::parse();

    let contents = resolve(fs::read_to_string(args.zinnfile));
    let zinnfile: Zinnfile = resolve(serde_yaml::from_str(&contents));
    let queue = Queue::new();

    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);
    let mp = MultiProgress::new();
    let main_bar_style = ProgressStyle::with_template("[{elapsed}] {wide_bar} {pos}/{len}").unwrap();
    let main_bar = ProgressBar::new(zinnfile.jobs.len() as u64);
    main_bar.set_style(main_bar_style);

    for name in args.targets {
        let job = match zinnfile.jobs.get(&name) {
            Some(job) => resolve(job.realize(&name, &zinnfile.jobs, &handlebars, &zinnfile.constants, &HashMap::new())),
            None => resolve(Err(ZinnError::JobNotFound(name))),
        };
        for dep in job.transitive_dependencies() {
            queue.enqueue(dep);
        }
        queue.enqueue(job);
    }

    let mut bars: Vec<_> = (0..args.jobs).map(|_| {
        let bar = ProgressBar::new(10000000);
        bar.set_style(ProgressStyle::with_template("{spinner} {prefix:.cyan} {wide_msg}").unwrap());
        bar.enable_steady_tick(Duration::from_millis(75));
        mp.add(bar.clone());
        bar
    }).collect();

    main_bar.set_length(queue.len() as u64);

    let threads: Vec<_> = (0..args.jobs).map(|_| {
        let main_bar = main_bar.clone();
        let verbose = args.verbose;
        let queue = queue.clone();
        let bar = bars.pop().unwrap();

        thread::spawn(move || {
            worker::run_worker(queue, bar, main_bar, verbose)
        })
    }).collect();

    mp.add(main_bar.clone());
    main_bar.enable_steady_tick(Duration::new(0, 200000));


    queue.done();
    for thread in threads {
        let _ = thread.join();
    }
}
