use std::io;
use std::process;
use thiserror::Error;

pub type ZinnResult<T> = Result<T, ZinnError>;

#[derive(Error, Debug)]
pub enum ZinnError {
    #[error("File Error: {0}")]
    File(#[from] io::Error),

    #[error("YAML Parsing Error: {0}")]
    YAML(#[from] serde_yaml::Error),

    #[error("Shell Error: unable to open stdin")]
    ShellStdin(),

    #[error("Shell Error: unable to open stdout")]
    ShellStdout(),

    #[error("Child Error: Child exited unsuccessfully")]
    Child(),
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

