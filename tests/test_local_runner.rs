#[macro_use]
extern crate lazy_static;

use std::path::PathBuf;
use std::sync::Mutex;

mod utils;


lazy_static! {
    static ref LOCK: Mutex<u32> = Mutex::new(0);
}


#[test]
fn run_mandelbrot() {

    let _test_must_be_executed_in_single_thread = LOCK.lock().unwrap();

    // Note: old madelbrot version with all parameters as positional args.
    let mandelbrot_args = vec![ "0.2", "0.35", "0.6", "0.45", "100", "100", "2" ]
        .into_iter()
        .map(String::from)
        .collect();

    let execute_in = PathBuf::from("tests/outputs/mandelbrot-2/");
    let result = utils::run(&utils::get_wasm_mandelbrot_executable(), &execute_in, mandelbrot_args);

    assert!(result.is_ok(), "Running mandelbrot failed.");

    let out_file = execute_in.join("out.png");
    assert!(out_file.exists())
}


#[test]
fn run_mandelbrot_13subtasks() {

    let _test_must_be_executed_in_single_thread = LOCK.lock().unwrap();

    // Note: old madelbrot version with all parameters as positional args.
    let mandelbrot_args = vec![ "0.2", "0.35", "0.6", "0.45", "100", "100", "13" ]
        .into_iter()
        .map(String::from)
        .collect();

    let execute_in = PathBuf::from("tests/outputs/mandelbrot-13/");
    let result = utils::run(&utils::get_wasm_mandelbrot_executable(), &execute_in, mandelbrot_args);

    result.unwrap();
    //assert!(result.is_ok(), "Running mandelbrot failed.");

    let out_file = execute_in.join("out.png");
    assert!(out_file.exists())
}


