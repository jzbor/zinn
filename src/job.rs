use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::io::{BufRead, BufReader};
use std::sync::Arc;

use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

use crate::barkeeper::ThreadStateTracker;
use crate::error::*;
use crate::queue::JobState;
use crate::render_component;
use crate::Options;


/// Template for a job as described in the Zinnfile
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobDescription {
    /// The shell commands to run for this job
    #[serde(default)]
    run: String,

    /// Dependencies of the job
    ///
    /// See also [`JobDependency`].
    #[serde(default)]
    requires: Vec<JobDependency>,

    /// Argument declarations
    #[serde(default)]
    args: Vec<String>,

    /// Argument defaults
    #[serde(default)]
    defaults: HashMap<String, String>,

    /// Input files as space-separated list
    #[serde(default)]
    inputs: Option<String>,

    /// Input files as native list
    #[serde(default)]
    input_list: Vec<String>,

    /// Output files as space-separated list
    #[serde(default)]
    outputs: Option<String>,

    /// Output files as native list
    #[serde(default)]
    output_list: Vec<String>,

    /// Run job in interactive mode
    #[serde(default)]
    interactive: bool,
}

/// Executable job with dependencies resolved and all variables applied
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct InnerJobRealization {
    name: String,
    run: String,
    interactive: bool,
    param_values: Vec<String>,  // for info/debugging purposes
    dependencies: Vec<JobRealization>,
    inputs: Vec<String>,
    outputs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobDependency {
    /// Name of the dependency job
    job: String,

    /// Arguments to pass to the dependency job
    #[serde(default)]
    with: HashMap<String, String>,

    /// Feed an argument by iterating over a space-separated list
    #[serde(default)]
    foreach: Option<Foreach>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Foreach {
    /// Parameter name
    var: String,

    /// List of input values (space-separated)
    #[serde(default)]
    r#in: String,
}


pub type JobRealization = Arc<InnerJobRealization>;

impl JobDescription {
    /// Resolve templates and dependencies
    pub fn realize(&self, name: &str, job_descriptions: &HashMap<String, JobDescription>, handlebars: &mut Handlebars, constants: &HashMap<String, String>, parameters: &HashMap<String, String>) -> ZinnResult<JobRealization> {
        let mut dependencies = Vec::new();
        let mut param_values = Vec::new();
        let name = name.to_owned();

        let mut combined_vars = constants.clone();

        for arg in &self.args {
            match parameters.get(arg).or(self.defaults.get(arg)) {
                Some(val) => {
                    combined_vars.insert(arg.to_owned(), val.to_owned());
                    param_values.push(val.to_owned());
                },
                None => return Err(ZinnError::MissingArgument(arg.to_owned())),
            }
        }

        // render input files
        let mut inputs = Vec::new();
        if let Some(input_str) = &self.inputs {
            let template_path = ["jobs", &name, "inputs"];
            let rendered_input_str = render_component(&template_path, input_str, handlebars, &combined_vars)?;
            let additional_inputs = rendered_input_str.split(char::is_whitespace)
                .filter(|v| !v.is_empty())
                .map(|s| s.to_owned());
            inputs.extend(additional_inputs)
        }
        for (i, input) in self.input_list.iter().enumerate() {
            let template_path = ["jobs", &name, "input-list", &i.to_string()];
            let rendered = render_component(&template_path, input, handlebars, &combined_vars)?;
            inputs.push(rendered);
        }

        // render output files
        let mut outputs = Vec::new();
        if let Some(output_str) = &self.outputs {
            let template_path = ["jobs", &name, "outputs"];
            let rendered_output_str = render_component(&template_path, output_str, handlebars, &combined_vars)?;
            let additional_outputs = rendered_output_str .split(char::is_whitespace)
                .filter(|v| !v.is_empty())
                .map(|s| s.to_owned());
            outputs.extend(additional_outputs)
        }
        for (i, output) in self.output_list.iter().enumerate() {
            let template_path = ["jobs", &name, "output-list", &i.to_string()];
            let rendered = render_component(&template_path, output, handlebars, &combined_vars)?;
            outputs.push(rendered);
        }

        for (i, dep) in self.requires.iter().enumerate() {
            let mut realized_dep_params = dep.with.clone();
            for (key, val) in &mut realized_dep_params {
                let template_path = ["jobs", &name, "requires", &i.to_string(), key];
                *val = render_component(&template_path, val, handlebars, &combined_vars)?;
            }

            let dep_desc = match job_descriptions.get(&dep.job) {
                Some(desc) => desc,
                None => return Err(ZinnError::DependencyNotFound(dep.job.to_owned())),
            };

            if let Some(with_list) = &dep.foreach {
                let template_path = ["jobs", &name, "requires", &i.to_string(), "foreach"];
                let inputs = render_component(&template_path, &with_list.r#in, handlebars, &combined_vars)?;
                let val_list = inputs.split(char::is_whitespace)
                    .filter(|v| !v.is_empty());
                for val in val_list {
                    // mutating the environment is fine, as it will be overridden
                    // for every iteration with the proper value.
                    realized_dep_params.insert(with_list.var.to_owned(), val.to_owned());
                    let dep_realization = dep_desc.realize(&dep.job, job_descriptions, handlebars, constants, &realized_dep_params)?;
                    dependencies.push(dep_realization);
                }
            } else {
                let dep_realization = dep_desc.realize(&dep.job, job_descriptions, handlebars, constants, &realized_dep_params)?;
                dependencies.push(dep_realization);
            }
        }

        let template_path = ["jobs", &name, "run"];
        let run = render_component(&template_path, &self.run, handlebars, &combined_vars)?;
        let name = name.replace('\n', "");
        let interactive = self.interactive;

        Ok(Arc::new(InnerJobRealization {
            name, run, dependencies, inputs, outputs, param_values, interactive
        }))
    }

    pub fn args(&self) -> &Vec<String> {
        &self.args
    }

}

impl InnerJobRealization {
    pub fn run(&self, tracker: &mut impl ThreadStateTracker, options: &Options) -> ZinnResult<JobState> {
        // skip if dry run
        if options.dry_run {
            if options.trace {
                tracker.trace(self.cmd());
            }
            return Ok(JobState::Finished);
        }

        // check if all input files exist
        self.check_input_files()?;

        // check if any input file is newer than any output file
        if !options.force && !self.inputs.is_empty() && !self.outputs.is_empty() && self.check_file_skip()? {
            return Ok(JobState::Skipped);
        }

        // print out trace
        if options.trace {
            tracker.trace(self.cmd());
        }

        let cmd_with_exit_setting = format!("set -e; {}", self.run);
        let mut  process = if self.interactive {
            // run job interactively
            Command::new("sh")
                .arg("-c")
                .arg(&cmd_with_exit_setting)
                .spawn()?
        } else {
            // run job without user interaction and track output
            let (io_reader, io_writer) = os_pipe::pipe()?;
            let process = Command::new("sh")
                .arg("-c")
                .arg(&cmd_with_exit_setting)
                .stdout(io_writer.try_clone()?)
                .stderr(io_writer)
                .spawn()?;

            for line in BufReader::new(io_reader).lines().map_while(Result::ok) {
                tracker.cmd_output(&self.to_string(), &line, options.verbose);
            }
            tracker.flush_cmd_output(&self.to_string(), options.verbose);

            process
        };

        let status = process.wait()?;

        for file in &self.outputs {
            if !Path::new(file).exists() {
                return Err(ZinnError::OutputFileError(file.to_owned()));
            }
        }

        if !status.success() {
            match status.code() {
                Some(code) => Err(ZinnError::ChildFailed(code)),
                None => Err(ZinnError::ChildSignaled()),
            }
        } else {
            Ok(JobState::Finished)
        }
    }

    fn check_input_files(&self) -> ZinnResult<()> {
        for file in &self.inputs {
            if !Path::new(file).exists() {
                return Err(ZinnError::InputFileError(file.to_owned()));
            }
        }

        Ok(())
    }

    fn check_file_skip(&self) -> ZinnResult<bool> {
        self.check_input_files()?;
        for output in &self.outputs {
            if !Path::new(output).exists() {
                return Ok(false);
            }

            for input in &self.inputs {
                let out_time = fs::metadata(output)?.modified()?;
                let in_time = fs::metadata(input)?.modified()?;
                if in_time > out_time {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    pub fn dependencies(&self) -> Vec<JobRealization> {
        self.dependencies.clone()
    }

    pub fn transitive_dependencies(&self) -> Vec<JobRealization> {
        let mut trans: Vec<_> = self.dependencies.iter().flat_map(|d| d.transitive_dependencies()).collect();
        trans.extend(self.dependencies());
        trans
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn cmd(&self) -> &str {
        &self.run
    }

    #[cfg(feature = "progress")]
    pub fn is_interactive(&self) -> bool {
        self.interactive
    }
}


impl fmt::Display for InnerJobRealization {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.name())?;
        if !self.param_values.is_empty() {
            write!(f, " {}", self.param_values.join(" "))?
        }
        Ok(())
    }
}
