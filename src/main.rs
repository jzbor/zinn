#![doc = include_str!("../README.md")]

use barkeeper::{StateTracker, ThreadStateTracker};
use clap::Parser;
use handlebars::Handlebars;
use queue::Queue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::{fs, thread};

use error::*;
use job::*;


mod barkeeper;
mod constants;
mod error;
mod hbextensions;
mod job;
mod nix;
mod queue;
mod worker;


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
    param: Vec<(String, String)>,

    /// Set or overwrite globals
    #[clap(short, long, value_parser = parse_key_val::<String, String>)]
    override_const: Vec<(String, String)>,

    /// Disable progress bars
    #[cfg(feature = "progress")]
    #[clap(short, long)]
    no_progress: bool,

    /// Open an interactive shell containing the specified Nix packages
    #[clap(long)]
    nix_shell: bool,

    /// Don't run jobs in a nix shell
    #[clap(long)]
    no_nix: bool,

    /// Run a command in a shell environment containing the specified Nix packages
    #[clap(long)]
    nix_run: Option<String>,
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

    /// Nix configuration
    ///
    /// See also [`NixConfig`]
    nix: Option<NixConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NixConfig {
    /// Flake reference to a flake providing the required packages
    #[serde(default = "nix::default_nixpkgs")]
    nixpkgs: String,

    /// Nix packages to add to the execution environment
    packages: Vec<String>,
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


fn run<T: StateTracker>(barkeeper: T, zinnfile: Zinnfile, nthreads: usize, constants: &HashMap<String, String>,
                        handlebars: Handlebars, args: Args)
where
    <T as StateTracker>::ThreadStateTracker: 'static
{

    // setup bars
    let mut thread_barkeepers = barkeeper.for_threads(nthreads);

    // feed the queue
    let queue = Queue::new();
    let parameters = args.param.iter().cloned().collect();
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

    barkeeper.set_njobs(queue.len());

    // start worker bars
    for tb in &thread_barkeepers {
        tb.start();
    }

    // start the threads
    let threads: Vec<_> = (0..nthreads).map(|_| {
        let queue = queue.clone();
        let tb: T::ThreadStateTracker = thread_barkeepers.pop().unwrap();
        let options = args.options();

        thread::spawn(move || {
            worker::run_worker(queue, tb, options)
        })
    }).collect();

    // enable the main bar
    barkeeper.start();

    // wait for the work to be completed
    queue.done();
    for thread in threads {
        let _ = thread.join();
    }
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
    let mut zinnfile: Zinnfile = resolve(serde_yaml::from_str(&contents));
    zinnfile.constants.extend(args.override_const.iter().cloned());

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

    // Nix features
    if let Some(nix_config) = &zinnfile.nix {
        // --nix-shell
        if args.nix_shell && !nix::inside_wrap() && nix::check_flakes() {
            resolve(nix::enter_shell(nix_config));
            return;
        }

        // --nix-run
        if let Some(cmd) = &args.nix_run {
            if !nix::inside_wrap() && nix::check_flakes() {
                resolve(nix::run(nix_config, cmd));
                return;
            }
        }

        // enter nix wrap if desired
        if !args.no_nix && !nix::inside_wrap() && nix::check_flakes() {
            resolve(nix::wrap(nix_config));
            return;
        }
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

    #[cfg(feature = "progress")]
    let has_interactive_jobs = |zf: &Zinnfile| zf.jobs.values().find(|j| j.is_interactive()).is_some();

    #[cfg(feature = "progress")]
    if args.no_progress || has_interactive_jobs(&zinnfile) {
        run(barkeeper::DummyBarkeeper::new(), zinnfile, nthreads, &constants, handlebars, args);
    } else {
        run(barkeeper::Barkeeper::new(), zinnfile, nthreads, &constants, handlebars, args);
    }

    #[cfg(not(feature = "progress"))]
    run(barkeeper::DummyBarkeeper::new(), zinnfile, nthreads, &constants, handlebars, args);
}
