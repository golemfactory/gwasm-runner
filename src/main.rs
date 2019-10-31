#![allow(clippy::unit_arg)]

use std::path::PathBuf;
use std::str::FromStr;
use structopt::*;

mod brass_config;
mod brass_runner;
mod brass_task;
#[cfg(feature = "with-gu-mode")]
mod gu_runner;
mod local_runner;

mod workdir;

use brass_runner::run_on_brass;
use local_runner::run_on_local;

#[derive(Debug, Clone)]
enum Backend {
    Local,
    GolemUnlimited(String),
    BrassGolem,
}

impl FromStr for Backend {
    type Err = String;

    fn from_str(s: &str) -> Result<Backend, String> {
        if s.starts_with("gu://") {
            let tail = &s[5..];
            if !tail.contains(':') {
                return Ok(Backend::GolemUnlimited(format!("{}:61622", tail)));
            } else {
                return Ok(Backend::GolemUnlimited(tail.to_string()));
            }
        }

        match s {
            "L" | "Local" | "local" => Ok(Backend::Local),
            "GU" | "Unlimited" | "GolemUnlimited" => Ok(Backend::GolemUnlimited(
                std::env::var("GU_HUB_ADDR").map_err(|e| e.to_string())?,
            )),
            "Golem" | "Brass" | "BrassGolem" | "GolemBrass" => Ok(Backend::BrassGolem),
            x => Err(format!("{} is not a valid Backend", x)),
        }
    }
}

#[derive(StructOpt, Debug, Clone)]
#[structopt(raw(setting = "structopt::clap::AppSettings::TrailingVarArg"))]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
struct Opt {
    /// Backend type to use
    #[structopt(long, short, default_value = "Local")]
    backend: Backend,
    /// Verbosity level. Add more v's to make app more verbose.
    #[structopt(short, parse(from_occurrences))]
    verbose: u8,
    /// Wasm App binary path to run. There should be an appropriate `.js` file within the same dir.
    #[structopt(parse(from_os_str))]
    wasm_app: PathBuf,
    /// All other args that will be passed to the Wasm App
    wasm_app_args: Vec<String>,
}

fn main() -> failure::Fallible<()> {
    let opts = Opt::from_args();

    env_logger::init_from_env(
        env_logger::Env::default().default_filter_or(match opts.verbose {
            0 => "info",
            1 => "debug",
            _ => "sp_wasm_engine=debug,info",
        }),
    );

    match opts.backend {
        Backend::BrassGolem => run_on_brass(&opts.wasm_app, &opts.wasm_app_args),
        Backend::Local => run_on_local(&opts.wasm_app, &opts.wasm_app_args),
        #[cfg(feature = "with-gu-mode")]
        Backend::GolemUnlimited(addr) => gu_runner::run(addr, &opts.wasm_app, &opts.wasm_app_args),
    }
}
