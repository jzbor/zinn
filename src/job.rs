use std::collections::HashMap;
use std::fmt::{self, Write};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::io::{BufRead, BufReader};
use std::sync::Arc;

use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

use crate::error::*;
use crate::queue::JobState;
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
}

/// Executable job with dependencies resolved and all variables applied
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct InnerJobRealization {
    name: String,
    run: String,
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
    pub fn realize(&self, name: &str, job_descriptions: &HashMap<String, JobDescription>, handlebars: &Handlebars, constants: &HashMap<String, String>, parameters: &HashMap<String, String>) -> ZinnResult<JobRealization> {
        let mut dependencies = Vec::new();
        let mut param_values = Vec::new();
        let name = name.to_owned();

        let mut combined_vars = HashMap::new();
        for (name, value) in constants {
            combined_vars.insert(name.clone(), handlebars.render_template(value, &())?);
        }

        for arg in &self.args {
            match parameters.get(arg).or(self.defaults.get(arg)) {
                Some(val) => {
                    combined_vars.insert(arg.to_owned(), val.to_owned());
                    param_values.push(val.to_owned());
                },
                None => return Err(ZinnError::MissingArgument(arg.to_owned())),
            }
        }

        for dep in &self.requires {
            let mut realized_dep_desc = dep.with.clone();
            for val in realized_dep_desc.values_mut() {
                *val = handlebars.render_template(val, &combined_vars)?;
            }

            let dep_desc = match job_descriptions.get(&dep.job) {
                Some(desc) => desc,
                None => return Err(ZinnError::DependencyNotFound(dep.job.to_owned())),
            };

            if let Some(with_list) = &dep.foreach {
                let inputs = handlebars.render_template(&with_list.r#in, &combined_vars)?;
                let val_list = inputs.split(char::is_whitespace)
                    .filter(|v| !v.is_empty());
                for val in val_list {
                    // mutating the environment is fine, as it will be overridden
                    // for every iteration with the proper value.
                    realized_dep_desc.insert(with_list.var.to_owned(), val.to_owned());
                    let dep_realization = dep_desc.realize(&dep.job, job_descriptions, handlebars, constants, &realized_dep_desc)?;
                    dependencies.push(dep_realization);
                }
            } else {
                let dep_realization = dep_desc.realize(&dep.job, job_descriptions, handlebars, constants, &realized_dep_desc)?;
                dependencies.push(dep_realization);
            }
        }

        let mut inputs = Vec::new();
        if let Some(input_str) = &self.inputs {
            let rendered_input_str = handlebars.render_template(input_str, &combined_vars)?;
            let additional_inputs = rendered_input_str.split(char::is_whitespace)
                .filter(|v| !v.is_empty())
                .map(|s| s.to_owned());
            inputs.extend(additional_inputs)
        }
        for input in &self.input_list {
            inputs.push(handlebars.render_template(input, &combined_vars)?);
        }
        let mut outputs = Vec::new();
        if let Some(output_str) = &self.outputs {
            let rendered_output_str = handlebars.render_template(output_str, &combined_vars)?;
            let additional_outputs = rendered_output_str .split(char::is_whitespace)
                .filter(|v| !v.is_empty())
                .map(|s| s.to_owned());
            outputs.extend(additional_outputs)
        }
        for output in &self.output_list {
            outputs.push(handlebars.render_template(output, &combined_vars)?);
        }

        let run = handlebars.render_template(&self.run, &combined_vars)?;
        let name = name.replace('\n', "");

        Ok(Arc::new(InnerJobRealization {
            name, run, dependencies, inputs, outputs, param_values,
        }))
    }

    pub fn args(&self) -> &Vec<String> {
        &self.args
    }
}

impl InnerJobRealization {
    pub fn run(&self, status_writer: &mut impl Write, log_writer: &mut impl Write, options: &Options) -> ZinnResult<JobState> {
        // skip if dry run
        if options.dry_run {
            if options.trace {
                let _ = writeln!(log_writer, "{}", self.cmd());
            }
            return Ok(JobState::Finished);
        }

        // check if all input files exist
        for file in &self.inputs {
            if !Path::new(file).exists() {
                return Err(ZinnError::InputFileError(file.to_owned()));
            }
        }

        // check if any input file is newer than any output file
        if !options.force && !self.inputs.is_empty() && !self.outputs.is_empty() {
            let mut dirty = false;
            for output in &self.outputs {
                if !Path::new(output).exists() {
                    dirty = true;
                    break;
                }

                for input in &self.inputs {
                    let out_time = fs::metadata(output)?.modified()?;
                    let in_time = fs::metadata(input)?.modified()?;
                    if in_time > out_time {
                        dirty = true;
                        break;
                    }
                }
            }
            if !dirty {
                return Ok(JobState::Skipped);
            }
        }

        // print out trace
        if options.trace {
            let _ = writeln!(log_writer, "{}", self.cmd());
        }

        let (io_reader, io_writer) = os_pipe::pipe()?;
        let cmd_with_exit_setting = format!("set -e; {}", self.run);
        let mut process = Command::new("sh")
            .arg("-c")
            .arg(cmd_with_exit_setting)
            .stdout(io_writer.try_clone()?)
            .stderr(io_writer)
            .spawn()?;


        let mut last_line: Option<String> = None;

        for line in BufReader::new(io_reader).lines().map_while(Result::ok) {
            let _ = writeln!(status_writer, "{}", line);

            if options.verbose {
                if let Some(line) = last_line.take() {
                    let _ = writeln!(log_writer, "{}: {}", self, line);
                }
                last_line = Some(line);
            }
        }
        if let Some(line) = last_line.take() {
            let _ = writeln!(log_writer, "{}: {}", self, line);
        }

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
