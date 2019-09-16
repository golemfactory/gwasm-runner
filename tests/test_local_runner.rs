mod utils;




#[test]
fn run_madelbrot() {

    let madelbrot_args = vec![ "0.2", "0.35", "0.6", "0.45", "100", "100", "2" ]
        .into_iter()
        .map(String::from)
        .collect();

    let result = utils::run(&utils::get_wasm_mandelbrot_executable(), madelbrot_args);
    result.unwrap();
}


#[test]
fn run_madelbrot_lacking_params() {

    // No subtasks number
    let madelbrot_args = vec![ "0.2", "0.35", "0.6", "0.45", "100", "100" ]
        .into_iter()
        .map(String::from)
        .collect();

    let result = utils::run(&utils::get_wasm_mandelbrot_executable(), madelbrot_args);
    result.unwrap();
}
