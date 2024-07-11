use std::env;
use std::process::{Command, Stdio};

use crate::{NixConfig, ZinnResult};


const NIX_ENV_MARKER: &str = "ZINN_NIX_ENV";


pub fn check_flakes() -> bool {
    Command::new("nix")
        .arg("shell")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

pub fn enter_shell(nix_config: &NixConfig) -> ZinnResult<()>{
    let packages = nix_config.packages.iter()
        .map(|p| format!("nixpkgs#{}", p));
    Command::new("nix")
        .arg("shell")
        .args(packages)
        .arg("--command")
        .arg(env::var("SHELL").unwrap_or(String::from("sh")))
        .env("name", "zinn")
        .env(NIX_ENV_MARKER, "1")
        .status()?;
    Ok(())
}

pub fn run(nix_config: &NixConfig, cmd: &str) -> ZinnResult<()>{
    let packages = nix_config.packages.iter()
        .map(|p| format!("nixpkgs#{}", p));
    Command::new("nix")
        .arg("shell")
        .args(packages)
        .arg("--command")
        .arg("sh")
        .arg("-c")
        .arg(cmd)
        .env("name", "zinn")
        .env(NIX_ENV_MARKER, "1")
        .status()?;
    Ok(())
}

pub fn wrap(nix_config: &NixConfig) -> ZinnResult<()>{
    let packages = nix_config.packages.iter()
        .map(|p| format!("nixpkgs#{}", p));
    Command::new("nix")
        .arg("shell")
        .args(packages)
        .arg("--command")
        .args(env::args())
        .env(NIX_ENV_MARKER, "1")
        .status()?;

    Ok(())
}

pub fn inside_wrap() -> bool {
    env::var(NIX_ENV_MARKER).is_ok()
}
