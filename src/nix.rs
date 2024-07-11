use std::env;
use std::process::{Command, Stdio};

use crate::{NixConfig, ZinnResult};


const NIX_ENV_MARKER: &str = "ZINN_NIX_ENV";


fn to_flake_parameters<'a>(nix_config: &'a NixConfig, packages: &'a[String]) -> impl Iterator<Item = String> + 'a {
    packages.iter()
        .map(|p| {
            if p.contains('#') {
                p.clone()
            } else {
                format!("{}#{}", nix_config.nixpkgs, p)
            }
        })
}

pub fn check_flakes() -> bool {
    Command::new("nix")
        .arg("shell")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

pub fn default_nixpkgs() -> String {
    String::from("nixpkgs")
}

pub fn enter_shell(nix_config: &NixConfig) -> ZinnResult<()>{
    Command::new("nix")
        .arg("shell")
        .args(to_flake_parameters(nix_config, &nix_config.packages))
        .arg("--command")
        .arg(env::var("SHELL").unwrap_or(String::from("sh")))
        .env("name", "zinn")
        .env(NIX_ENV_MARKER, "1")
        .status()?;
    Ok(())
}

pub fn inside_wrap() -> bool {
    env::var(NIX_ENV_MARKER).is_ok()
}

pub fn run(nix_config: &NixConfig, cmd: &str) -> ZinnResult<()>{
    Command::new("nix")
        .arg("shell")
        .args(to_flake_parameters(nix_config, &nix_config.packages))
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
    Command::new("nix")
        .arg("shell")
        .args(to_flake_parameters(nix_config, &nix_config.packages))
        .arg("--command")
        .args(env::args())
        .env(NIX_ENV_MARKER, "1")
        .status()?;

    Ok(())
}
