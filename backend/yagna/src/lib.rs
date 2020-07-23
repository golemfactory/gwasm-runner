#![allow(clippy::unit_arg)]
use gwr_backend::rt::Engine;
use gwr_backend::Flags;
use std::path::Path;
use url::Url;
pub use ya_client::model::market::Demand;

mod demand;
mod negotiator;
mod runner;
mod storage;

pub trait YagnaEngine: Engine {
    fn build_image(&self, wasm_path: &Path) -> anyhow::Result<Vec<u8>>;

    fn build_demand(
        &self,
        node_name: &str,
        wasm_url: &str,
        timeout: std::time::Duration,
        subnet: Option<&String>,
    ) -> anyhow::Result<Demand>;
}

#[derive(Debug, Clone)]
pub struct YagnaBackend {
    url: Option<String>,
    token: Option<String>,
    subnet: Option<String>,
}

impl YagnaBackend {
    pub fn parse_url(url: &str) -> anyhow::Result<Option<Self>> {
        if let Ok(url) = Url::parse(url) {
            let scheme = url.scheme();
            if scheme != "yagna" && scheme != "lwg" {
                return Ok(None);
            }
            let mut token = None;
            let mut subnet = None;
            for (param, value) in url.query_pairs() {
                match param.as_ref() {
                    "token" | "appkey" => token = Some(value.into()),
                    "subnet" => subnet = Some(value.into()),
                    _ => log::warn!("unknown url key: {}", param),
                }
            }
            return Ok(Some(YagnaBackend {
                url: None,
                token,
                subnet,
            }));
        }

        Ok(match url {
            "yagna" | "lwg" => Some(YagnaBackend {
                url: None,
                token: None,
                subnet: None,
            }),
            _ => None,
        })
    }

    pub fn run<E: YagnaEngine + 'static>(
        &self,
        engine: E,
        flags: &Flags,
        wasm_path: &Path,
        args: &[String],
    ) -> anyhow::Result<()> {
        runner::run(
            self.url.clone(),
            self.token.clone(),
            self.subnet.clone(),
            engine,
            wasm_path,
            flags.timeout.into(),
            args,
        )
    }
}
