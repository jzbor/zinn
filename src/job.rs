use std::collections::HashMap;
use std::fmt::Write;
use std::process::{Command, Stdio};
use std::io::{BufWriter, BufRead, BufReader};
use std::io::Write as _;
use std::sync::Arc;

use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

use crate::error::*;


/// Template for a job as described in the Zinnfile
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobDescription {
    #[serde(default)]
    run: String,

    #[serde(default)]
    requires: Vec<JobDependency>,

    #[serde(default)]
    args: Vec<String>,
}

/// Executable job with dependencies resolved and all variables applied
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct InnerJobRealization {
    name: String,
    run: String,
    dependencies: Vec<JobRealization>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobDependency {
    job: String,

    #[serde(default)]
    with: HashMap<String, String>,

    #[serde(default)]
    with_list: Option<WithList>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WithList {
    param: String,

    #[serde(default)]
    inputs: String,
}


pub type JobRealization = Arc<InnerJobRealization>;

impl JobDescription {
    /// Resolve templates and dependencies
    pub fn realize(&self, name: &str, job_descriptions: &HashMap<String, JobDescription>, handlebars: &Handlebars, constants: &HashMap<String, String>, parameters: &HashMap<String, String>) -> ZinnResult<JobRealization> {
        let mut dependencies = Vec::new();
        let mut name = name.to_owned();

        let mut combined_vars = HashMap::new();
        for (name, value) in constants {
            combined_vars.insert(name.clone(), handlebars.render_template(value, &())?);
        }

        for arg in &self.args {
            match parameters.get(arg) {
                Some(val) => {
                    combined_vars.insert(arg.to_owned(), val.to_owned());
                    name.push(' ');
                    name.push_str(val);
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

            if let Some(with_list) = &dep.with_list {
                let inputs = handlebars.render_template(&with_list.inputs, &combined_vars)?;
                let val_list = inputs.split(char::is_whitespace);
                for val in val_list {
                    // mutating the environment is fine, as it will be overridden
                    // for every iteration with the proper value.
                    realized_dep_desc.insert(with_list.param.to_owned(), val.to_owned());
                    let dep_realization = dep_desc.realize(&dep.job, job_descriptions, handlebars, constants, &realized_dep_desc)?;
                    dependencies.push(dep_realization);
                }
            } else {
                let dep_realization = dep_desc.realize(&dep.job, job_descriptions, handlebars, constants, &realized_dep_desc)?;
                dependencies.push(dep_realization);
            }
        }

        let run = handlebars.render_template(&self.run, &combined_vars)?;
        let name = name.replace('\n', "");

        Ok(Arc::new(InnerJobRealization {
            name, run, dependencies,
        }))
    }
}

impl InnerJobRealization {
    pub fn run(&self, status_writer: &mut impl Write, log_writer: &mut impl Write, verbose: bool) -> ZinnResult<String> {
        let (io_reader, io_writer) = os_pipe::pipe()?;

        let mut process = Command::new("sh")
            .stdin(Stdio::piped())
            .stdout(io_writer.try_clone()?)
            .stderr(io_writer)
            .spawn()?;


        let stdin = process.stdin.take()
            .ok_or_else(ZinnError::ShellStdin)?;
        let mut writer = BufWriter::new(&stdin);
        write!(writer, "{}", self.run)?;
        writer.flush()?;
        drop(writer);
        drop(stdin);

        let output = String::new();
        let mut last_line: Option<String> = None;

        for line in BufReader::new(io_reader).lines().map_while(Result::ok) {
            let _ = write!(status_writer, "{}", line);

            if verbose {
                if let Some(line) = last_line.take() {
                    let _ = write!(log_writer, "{}", format!("{}: {}", self.name, line));
                }
                last_line = Some(line);
            }
        }
        if let Some(line) = last_line.take() {
            let _ = write!(log_writer, "{}", format!("{}: {}", self.name, line));
        }

        assert!(!self.name.contains('\n'));

        let status = process.wait()?;
        if !status.success() {
            Err(ZinnError::Child())
        } else {
            Ok(output)
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
}

