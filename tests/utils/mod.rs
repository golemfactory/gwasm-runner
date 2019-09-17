use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::fs;
use failure::{Error};

use std::env;


fn get_runner_binary_path() -> PathBuf {
    return PathBuf::from("target/release/gwasm-runner");
}

pub fn get_wasm_executables_dir() -> PathBuf {
    return PathBuf::from("tests/wasm-binaries/");
}

pub fn get_wasm_mandelbrot_executable() -> PathBuf {
    return get_wasm_executables_dir().join("mandelbrot.wasm");
}

fn prepare_test_dir(execute_dir: &Path) -> Result<(), Error> {
    if execute_dir.exists() {
        fs::remove_dir_all(execute_dir)?;
    }
    fs::create_dir_all(execute_dir)?;
    Ok(())
}

pub fn run(binary: &Path, execute_dir: &Path, args: Vec<String>) -> Result<Output, Error> {
    let runner = get_runner_binary_path().to_str().unwrap().to_owned();

    let working_dir = env::current_dir()?;

    let runner_abs = working_dir.join(runner);
    let binary_abs = working_dir.join(binary);

    prepare_test_dir(execute_dir)?;

    env::set_current_dir(execute_dir)?;

    // Build args.
    let mut cmd_args = vec![String::from("--"), binary_abs.to_str().unwrap().to_owned()];
    cmd_args.append(&mut args.clone());

    let result = Command::new(runner_abs)
        .args(&cmd_args)
        .output();

    env::set_current_dir(&working_dir)?;

    return Ok(result?);
}
