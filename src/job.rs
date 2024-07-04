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
    run: String,

    #[serde(default)]
    requires: Vec<String>,
}

/// Executable job with dependencies resolved and all variables applied
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct InnerJobRealization {
    name: String,
    run: String,
    dependencies: Vec<JobRealization>,
}

pub type JobRealization = Arc<InnerJobRealization>;

impl JobDescription {
    /// Resolve templates and dependencies
    pub fn realize(&self, name: &str, job_descriptions: &HashMap<String, JobDescription>, handlebars: &Handlebars, constants: &HashMap<String, String>) -> ZinnResult<JobRealization> {
        let mut dependencies = Vec::new();
        for dep in &self.requires {
            match job_descriptions.get(dep) {
                Some(desc) => dependencies.push(desc.realize(&dep, job_descriptions, handlebars, constants)?),
                None => return Err(ZinnError::DependencyNotFound(dep.to_owned())),
            }
        }

        let run = handlebars.render_template(&self.run, constants)?;

        Ok(Arc::new(InnerJobRealization {
            name: name.to_owned(),
            run,
            dependencies,
        }))
    }
}

impl InnerJobRealization {
    pub fn run(&self, status_writer: &mut impl Write, log_writer: &mut impl Write, verbose: bool) -> ZinnResult<String> {
        let mut process = Command::new("sh")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;


        let stdin = process.stdin.take()
            .ok_or_else(ZinnError::ShellStdin)?;
        let mut writer = BufWriter::new(&stdin);
        write!(writer, "{}", self.run)?;
        writer.flush()?;
        drop(writer);
        drop(stdin);

        let stdout = process.stdout.take()
            .ok_or_else(ZinnError::ShellStdout)?;
        let reader = BufReader::new(stdout);
        let output = String::new();

        let mut last_line: Option<String> = None;

        for line in reader.lines().flatten() {
            let _ = write!(status_writer, "{}", line);

            if verbose {
                if let Some(line) = last_line.take() {
                    let _ = write!(log_writer, "{}: {}", self.name, line);
                }
                last_line = Some(line);
            }
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
        self.dependencies().iter().flat_map(|d| d.transitive_dependencies()).collect()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

