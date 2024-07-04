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

    #[error("Unable to open stdin")]
    ShellStdin(),

    #[error("Unable to open stdout")]
    ShellStdout(),

    #[error("Child exited unsuccessfully")]
    Child(),

    #[error("Dependency not found ({0})")]
    DependencyNotFound(String),

    #[error("Job not found ({0})")]
    JobNotFound(String),

    #[error("Template rendering failed - ({0})")]
    TemplateError(#[from] handlebars::RenderError),
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

