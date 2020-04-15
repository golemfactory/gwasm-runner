#![allow(clippy::unit_arg)]

use std::path::PathBuf;
use std::str::FromStr;
use structopt::*;

#[cfg(feature = "with-brass-mode")]
mod brass_config;

#[cfg(feature = "with-brass-mode")]
mod brass_runner;
#[cfg(feature = "with-brass-mode")]
mod brass_task;
#[cfg(feature = "with-gu-mode")]
mod gu_runner;
mod local_runner;
mod lwg;
mod wasm_engine;
mod workdir;

#[cfg(feature = "with-brass-mode")]
use brass_runner::run_on_brass;
use local_runner::run_on_local;

#[derive(Debug, Clone)]
enum Backend {
    Local,
    GolemUnlimited(String),
    BrassGolem,
    Lwg {
        url: Option<String>,
        token: Option<String>,
    },
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
            "yagna" | "lwg" => Ok(Backend::Lwg {
                url: None,
                token: None,
            }),
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
    /// Skip confirmation dialogs
    #[structopt(short = "y", long = "assume-yes")]
    skip_confirmation: bool,
}

fn main() -> anyhow::Result<()> {
    let opts = Opt::from_args();

    env_logger::init_from_env(
        env_logger::Env::default().default_filter_or(match opts.verbose {
            0 => "cranelift_wasm=warn,info",
            1 => "debug",
            _ => "sp_wasm_engine=debug,info",
        }),
    );

    let engine = wasm_engine::engine();

    match opts.backend {
        #[cfg(feature = "with-brass-mode")]
        Backend::BrassGolem => {
            run_on_brass(&opts.wasm_app, opts.skip_confirmation, &opts.wasm_app_args)
        }

        #[cfg(not(feature = "with-brass-mode"))]
        Backend::BrassGolem => Ok(eprintln!("golem brass mode is unsupported in this runner")),

        Backend::Local => run_on_local(engine, &opts.wasm_app, &opts.wasm_app_args),
        Backend::Lwg { url, token } => {
            lwg::run(url, token, engine, &opts.wasm_app, &opts.wasm_app_args)
        }
        #[cfg(feature = "with-gu-mode")]
        Backend::GolemUnlimited(addr) => gu_runner::run(addr, &opts.wasm_app, &opts.wasm_app_args),
        #[cfg(not(feature = "with-gu-mode"))]
        Backend::GolemUnlimited(_) => {
            Ok(eprintln!("golem unlimited is unsupported in this runner"))
        }
    }
}
