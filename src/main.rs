#![doc = include_str!("../README.md")]

use clap::Parser;
use handlebars::Handlebars;
use queue::Queue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
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
mod hbextensions;
mod constants;


const DOCS_URL: &str = "https://jzbor.de/zinn/zinn";


#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Zinnfile to run
    #[clap(short, long, default_value_t = String::from("zinn.yaml"))]
    file: String,

    /// Target jobs to execute as entry points
    #[clap(default_values_t = [String::from("default")])]
    targets: Vec<String>,

    /// Number of jobs to run in parallel
    #[clap(short, long)]
    jobs: Option<usize>,

    /// Print output of jobs
    #[clap(short, long)]
    verbose: bool,

    /// Force rebuild all files
    #[clap(short = 'B', long)]
    force_rebuild: bool,

    /// Open documentation in the browser
    #[clap(long)]
    docs: bool,

    /// List all jobs with their parameters
    #[clap(long)]
    list: bool,

    /// Output commands before executing
    #[clap(short, long)]
    trace: bool,

    /// Don't actually execute the commands
    #[clap(long)]
    dry_run: bool,

    /// Set parameters for the initial job
    #[clap(short, long, value_parser = parse_key_val::<String, String>)]
    parameters: Vec<(String, String)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Options {
    verbose: bool,
    force: bool,
    trace: bool,
    dry_run: bool,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
struct Zinnfile {
    /// Constants to pass to the jobs
    ///
    /// All constants are rendered, with all previous constants being available.
    #[serde(default)]
    #[serde(deserialize_with = "constants::parse")]
    constants: Vec<(String, String)>,

    /// Descriptions of the jobs
    ///
    /// See also [`JobDescription`].
    jobs: HashMap<String, JobDescription>,
}

impl Args {
    fn options(&self) -> Options {
        Options {
            verbose: self.verbose,
            force: self.force_rebuild,
            trace: self.trace,
            dry_run: self.dry_run,
        }
    }
}


/// Parse a single key-value pair
fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

fn main() {
    let args = Args::parse();

    // process arguments
    if args.docs {
        let result = std::process::Command::new("xdg-open")
            .arg(DOCS_URL)
            .spawn();
        match result {
            Ok(_) => (),
            Err(e) => { eprintln!("Error: {}", e); std::process::exit(1); }
        }
        return;
    }
    let nthreads = if let Some(nthreads) = args.jobs {
        nthreads
    } else if let Ok(nthreads) = thread::available_parallelism() {
        nthreads.into()
    } else { 4 };

    // read zinnfile
    let contents = resolve(fs::read_to_string(&args.file));
    let zinnfile: Zinnfile = resolve(serde_yaml::from_str(&contents));

    // --list
    if args.list {
        for (name, job) in zinnfile.jobs {
            print!("{}", name);
            if !job.args().is_empty() {
                print!(" ({})", job.args().join(", "));
            }
            println!();
        }
        return;
    }


    // init template engine
    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);
    handlebars.register_escape_fn(handlebars::no_escape);
    hbextensions::register_helpers(&mut handlebars);

    // parse constants
    let mut constants = HashMap::new();
    for (name, value) in &zinnfile.constants {
        let realized = resolve(handlebars.render_template(value, &constants));
        constants.insert(name.to_owned(), realized);
    }

    // setup bars
    let mp = MultiProgress::new();
    let main_bar_style = ProgressStyle::with_template("[{elapsed}] {wide_bar} {pos}/{len}").unwrap();
    let main_bar = ProgressBar::new(zinnfile.jobs.len() as u64);
    main_bar.set_style(main_bar_style);
    let mut bars: Vec<_> = (0..nthreads).map(|_| {
        let bar = ProgressBar::new(10000000);
        bar.set_style(ProgressStyle::with_template("{spinner} {prefix:.cyan} {wide_msg}").unwrap());
        mp.add(bar.clone());
        bar
    }).collect();

    // feed the queue
    let queue = Queue::new();
    let parameters = args.parameters.iter().cloned().collect();
    for name in &args.targets {
        let job = match zinnfile.jobs.get(name) {
            Some(job) => resolve(job.realize(name, &zinnfile.jobs, &handlebars, &constants, &parameters)),
            None => resolve(Err(ZinnError::JobNotFound(name.to_owned()))),
        };
        for dep in job.transitive_dependencies() {
            queue.enqueue(dep);
        }
        queue.enqueue(job);
    }

    main_bar.set_length(queue.len() as u64);

    // start worker bars
    for bar in &mut bars {
        bar.tick();
        bar.enable_steady_tick(Duration::from_millis(75));
    }

    // start the threads
    let threads: Vec<_> = (0..nthreads).map(|_| {
        let main_bar = main_bar.clone();
        let queue = queue.clone();
        let bar = bars.pop().unwrap();
        let options = args.options();

        thread::spawn(move || {
            worker::run_worker(queue, bar, main_bar, options)
        })
    }).collect();

    // enable the main bar
    mp.add(main_bar.clone());
    main_bar.enable_steady_tick(Duration::from_millis(75));

    // wait for the work to be completed
    queue.done();
    for thread in threads {
        let _ = thread.join();
    }
}
