use actix::prelude::*;
use std::path::Path;
use failure::{Fallible, bail};
use sp_wasm_engine::prelude::*;
use crate::workdir::WorkDir;
use crate::local_runner::run_local_code;
use std::fs;
use zip::CompressionMethod;
use std::io::Cursor;

// 1. Image cache [TODO]
// 2.
//

fn push_image(wasm_path: &Path, js_path : &Path) -> Fallible<()> {
    let name_ws = wasm_path.file_name().unwrap().to_string_lossy();
    let name_js = js_path.file_name().unwrap().to_string_lossy();



    let mut zw = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zw.start_file(name_ws.as_ref(), zip::write::FileOptions::default().compression_method(CompressionMethod::Bzip2));
    std::io::copy(&mut fs::OpenOptions::new().read(true).open(wasm_path)?, &mut zw)?;
    zw.start_file(name_js.as_ref(), zip::write::FileOptions::default().compression_method(CompressionMethod::Bzip2));
    std::io::copy(&mut fs::OpenOptions::new().read(true).open(js_path)?, &mut zw)?;
    let data = zw.finish()?.into_inner();
    fs::write("/tmp/r.zip", data)?;
    Ok(())
}

pub fn run(wasm_path: &Path, args: &[String]) -> Fallible<()> {
    let sys = System::new("GU-wasm -runner");

    let engine_ref = Sandbox::init_ejs()?;
    let mut w = WorkDir::new("gu")?;

    let js_path = wasm_path.with_extension("js");

    if !js_path.exists() {
        bail!("file not found: {}", js_path.display())
    }

    push_image(&wasm_path, &js_path)?;

    let output_path = w.split_output()?;
    {
        let mut split_args = Vec::new();
        split_args.push("split".to_owned());
        split_args.push("/task_dir/".to_owned());
        split_args.extend(args.iter().cloned());
        run_local_code(
            engine_ref.clone(),
            wasm_path,
            &js_path,
            &output_path,
            split_args,
        )?;
    }


    unimplemented!()
}