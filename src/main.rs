#![allow(clippy::unit_arg)]
use gwr_backend::{rt, rt::Engine, Flags};
use std::path::PathBuf;
use std::str::FromStr;
use structopt::*;

#[derive(StructOpt, Clone)]
#[structopt(raw(setting = "structopt::clap::AppSettings::TrailingVarArg"))]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
struct Opt {
    #[structopt(flatten)]
    flags: Flags,
    /// Backend type to use
    #[structopt(long, short, default_value = "Local")]
    backend: Backend,
    #[structopt(long, short)]
    runtime: Option<RuntimeName>,
    /// Wasm App binary path to run.
    #[structopt(parse(from_os_str))]
    pub wasm_app: PathBuf,
    /// All other args that will be passed to the Wasm App
    pub wasm_app_args: Vec<String>,
}

macro_rules! gen {
    {
       dolar $dolar:tt;

       enum Runtime {
            $(
                $(#[feature($feature:expr)])?
                $id:ident($engine:ty)
            ),*
       }

       enum Backend {
           $(
                $(#[feature($b_feature:expr)])?
                $b_id:ident($backend:ty)
            ),*
       }
     } => {
        #[derive(Clone)]
        enum Runtime {
            $(
            $(#[cfg(feature=$feature)])?
            $id($engine)
            ),*
        }

        #[derive(Clone)]
        enum RuntimeName {
            $(
                $(#[cfg(feature=$feature)])?
                $id
            ),*
        }

        #[derive(Clone)]
        enum Backend {
            $(
            $(#[cfg(feature=$b_feature)])?
            $b_id($backend)
            ),*
        }

         impl FromStr for RuntimeName {
            type Err = anyhow::Error;

            fn from_str(s: &str) -> anyhow::Result<RuntimeName> {
                $(
                $(#[cfg(feature=$feature)])?
                if s.eq_ignore_ascii_case(stringify!($id)) {
                    return Ok(RuntimeName::$id)
                })*
                anyhow::bail!("{} is not a valid Backend", s)
            }
        }

        impl RuntimeName {

            fn into_runtime(self) -> anyhow::Result<Runtime> {
               Ok(match self { $(
                $(#[cfg(feature=$feature)])?
                RuntimeName::$id => Runtime::$id(<$engine>::new()?)
                ),*
               })
            }
        }

        impl FromStr for Backend {
            type Err = anyhow::Error;

            fn from_str(s: &str) -> anyhow::Result<Backend> {
                $(
                $(#[cfg(feature=$b_feature)])?
                if let Some(b) = <$backend>::parse_url(s)? {
                    return Ok(Backend::$b_id(b))
                })*

                anyhow::bail!("{} is not a valid Backend", s)
            }
        }

        macro_rules! internal_gen_run {
            {on($dolar runtime:expr) $dolar rt:ident => $dolar e:expr} => {{
                match $dolar runtime {
                        $(
                            $(#[cfg(feature=$feature)])?
                            Runtime::$id($dolar rt) => ($dolar e)
                        ),*
                }
            }}
        }

        impl Opt {

            fn run(self) -> anyhow::Result<()> {
                Ok(match self.backend {
                $(
                    $(#[cfg(feature=$b_feature)])?
                    Backend::$b_id(backend) => internal_gen_run! {
                        on(self.runtime.unwrap().into_runtime()?)
                        engine => backend.run(engine, &self.flags, &self.wasm_app, self.wasm_app_args.as_ref())?
                    }
                ),*
                })
            }

        }
     }
}

gen! {
    dolar $;

    enum Runtime {
        #[feature("spwasm")]
        SpWasm(gwr_backend::SpEngine),
        #[feature("wasmtime")]
        Wasmtime(gwr_backend::WtEngine)
    }
    enum Backend {
        Local(gwr_backend::LocalBackend),
        #[feature("with-brass")]
        Brass(gwr_backend_brass::BrassBackend),
        #[feature("with-gu")]
        Unlimited(gwr_backend_unlimited::GuBackend),
        #[feature("with-yagna")]
        Yagna(gwr_backend_yagna::YagnaBackend)
    }
}

fn main() -> anyhow::Result<()> {
    let opts = Opt::from_args();
    env_logger::init_from_env(env_logger::Env::default().default_filter_or(
        match opts.flags.verbose {
            0 => "cranelift_wasm=warn,info",
            1 => "debug",
            _ => "sp_wasm_engine=debug,info",
        },
    ));
    opts.run()?;
    Ok(())
}
