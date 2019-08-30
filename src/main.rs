use std::path::PathBuf;
use std::str::FromStr;

use failure::ResultExt;
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
            "L" | "Local" | "local" => Ok(Backend::Local),
            "GU" | "Unlimited" | "GolemUnlimited" => Ok(Backend::GolemUnlimited),
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
    /// Wasm App binary path to run. There should be an appropriate `.js` file within the same dir.
    #[structopt(long, short, parse(from_os_str))]
    wasm_app: PathBuf,
    /// Verbosity level. Add more v's to make app more verbose.
    #[structopt(short, parse(from_occurrences))]
    verbose: u8,
    /// List of volumes to bind mount
    #[structopt(long)]
    volume: Vec<String>,
    /// All other args that will be passed to the Wasm App
    wasm_app_args: Vec<String>,
}

pub fn run_wasm_app(
    volumes: Vec<String>,
    app: PathBuf,
    args: Vec<String>,
) -> failure::Fallible<()> {
    let mut sandbox = sp_wasm_engine::sandbox::Sandbox::new()?.set_exec_args(args)?;

    sandbox.init()?;
    for volume in volumes {
        let mut it = volume.split(":").fuse();
        match (it.next(), it.next(), it.next()) {
            (Some(src), Some(dst), None) => sandbox
                .mount(src, dst)
                .context(format!("on bind mount: {}:{}", src, dst))?,
            _ => return Err(failure::err_msg(format!("invalid volume: {}", volume))),
        }
    }

    let app_js = app.with_extension("js");
    let app_wasm = app.with_extension("wasm");

    sandbox.run(app_js.to_str().unwrap(), app_wasm.to_str().unwrap())?;

    Ok(())
}

fn main() -> failure::Fallible<()> {
    let opts = Opt::from_args();

    env_logger::init_from_env(
        env_logger::Env::default().default_filter_or(match opts.verbose {
            0 => "error",
            1 => "info",
            _ => "sp_wasm_engine=debug,info",
        }),
    );

    match opts.backend {
        Backend::Local => run_wasm_app(opts.volume, opts.wasm_app, opts.wasm_app_args),
        _ => unimplemented!(),
    }
}
