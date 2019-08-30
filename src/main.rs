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
    type Err = String;

    fn from_str(s: &str) -> Result<Backend, String> {
        match s {
            "L" => Ok(Backend::Local),
            "Local" => Ok(Backend::Local),
            "GU" => Ok(Backend::GolemUnlimited),
            "Unlimited" => Ok(Backend::GolemUnlimited),
            "GolemUnlimited" => Ok(Backend::GolemUnlimited),
            "Golem" => Ok(Backend::BrassGolem),
            "Brass" => Ok(Backend::BrassGolem),
            "BrassGolem" => Ok(Backend::BrassGolem),
            "GolemBrass" => Ok(Backend::BrassGolem),
            x => Err(format!("{} is not a valid Backend", x)),
        }
    }
}

#[derive(StructOpt, Debug, Clone)]
#[structopt(raw(setting = "structopt::clap::AppSettings::TrailingVarArg"))]
struct Opt {
    /// Backend type to use
    #[structopt(long, short, default_value = "Local")]
    backend: Backend,
    /// Wasm App binary file to run
    #[structopt(long, short, parse(from_os_str))]
    wasm_app: PathBuf,
    /// All other args that will be passed to the Wasm App
    wasm_app_args: Vec<String>,
}

fn main() {
    let opt = Opt::from_args();
    println!("{:?}", opt);
}
