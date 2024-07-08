use std::io;
use std::process;
use thiserror::Error;

pub type ZinnResult<T> = Result<T, ZinnError>;

#[derive(Error, Debug)]
pub enum ZinnError {
    #[error("File Error - {0}")]
    File(#[from] io::Error),

    #[error("{0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Child exited with error {0}")]
    ChildFailed(i32),

    #[error("Child terminated by signal")]
    ChildSignaled(),

    #[error("Dependency not found ({0})")]
    DependencyNotFound(String),

    #[error("Job not found ({0})")]
    JobNotFound(String),

    #[error("Missing argument \"{0}\"")]
    MissingArgument(String),

    #[error("Template rendering failed - ({0})")]
    TemplateError(#[from] handlebars::RenderError),

    #[error("Missing input file \"{0}\"")]
    InputFileError(String),

    #[error("Missing output file \"{0}\"")]
    OutputFileError(String),

    #[cfg(feature = "regex")]
    #[error("Unable to parse regex - {0}")]
    RegexError(#[from] regex::Error),
}

pub fn die(e: impl Into<ZinnError>) -> ! {
    eprintln!("{}", e.into());
    process::exit(1);
}

pub fn resolve<T>(res: Result<T, impl Into<ZinnError>>) -> T {
    match res {
        Ok(val) => val,
        Err(e) => die(e),
    }
}

