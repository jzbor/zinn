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
        let (mut io_reader, io_writer) = os_pipe::pipe()?;

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

        for line in BufReader::new(io_reader).lines().flatten() {
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

