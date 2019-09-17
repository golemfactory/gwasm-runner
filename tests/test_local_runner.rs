use std::path::PathBuf;

mod utils;




#[test]
fn run_madelbrot() {

    let mandelbrot_args = vec![ "0.2", "0.35", "0.6", "0.45", "100", "100", "2" ]
        .into_iter()
        .map(String::from)
        .collect();

    let execute_in = PathBuf::from("tests/outputs/mandelbrot/");
    let result = utils::run(&utils::get_wasm_mandelbrot_executable(), &execute_in, mandelbrot_args);

    assert!(result.is_ok(), "Running mandelbrot failed.");

    let out_file = execute_in.join("out.png");
    assert!(out_file.exists())
}

