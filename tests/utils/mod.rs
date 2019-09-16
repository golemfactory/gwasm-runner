use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::io;


fn get_runner_binary_path() -> PathBuf {
    return PathBuf::from("target/release/gwasm-runner");
}

pub fn get_wasm_executables_dir() -> PathBuf {
    return PathBuf::from("tests/wasm-binaries/");
}

pub fn get_wasm_mandelbrot_executable() -> PathBuf {
    return get_wasm_executables_dir().join("mandelbrot.wasm");
}

pub fn run(binary: &Path, args: Vec<String>) -> io::Result<Output> {
    let runner = get_runner_binary_path().to_str().unwrap().to_owned();

    let mut cmd_args = vec![String::from("--"), binary.to_str().unwrap().to_owned()];
    cmd_args.append(&mut args.clone());

    Command::new(runner)
        .args(&cmd_args)
        .output()
}
