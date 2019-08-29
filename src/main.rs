use std::path::PathBuf;
use std::str::FromStr;
use structopt::*;

#[derive(Debug, Clone)]
enum Backend {
    Local,
    GolemUnlimited,
    BrassGolem,
}

impl FromStr for Backend {
    type Err = ();

    fn from_str(s: &str) -> Result<Backend, ()> {
        match s {
            "Local" => Ok(Backend::Local),
            "GolemUnlimited" => Ok(Backend::GolemUnlimited),
            "GU" => Ok(Backend::GolemUnlimited),
            "Brass" => Ok(Backend::BrassGolem),
            "Golem" => Ok(Backend::BrassGolem),
            _ => Err(()),
        }
    }
}

#[derive(Debug, StructOpt, Clone)]
struct Opt {
    /// Backend type to use
    #[structopt(long, short)]
    backend: Backend,
    /// Wasm binary file to run
    #[structopt(long, short, parse(from_os_str))]
    wasm_app: PathBuf,
    /// Wasm app arguments
    wasm_args: String,
}

fn main() {
    let opt = Opt::from_args();
    println!("{:?}", opt);
}
