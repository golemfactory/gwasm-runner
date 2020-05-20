use gwr_backend::rt::Engine;
use std::path::Path;
pub use ya_client::model::market::Demand;
mod demand;
mod negotiator;
mod runner;
mod storage;

pub trait YagnaEngine: Engine {
    fn build_image(wasm_path: &Path) -> anyhow::Result<Vec<u8>>;

    fn build_demand(
        node_name: &str,
        wasm_url: &str,
        timeout: std::time::Duration,
    ) -> anyhow::Result<Demand>;
}

#[derive(Debug, Clone)]
pub struct YagnaBackend {
    url: Option<String>,
    token: Option<String>,
}

impl YagnaBackend {
    pub fn parse_url(url: &str) -> anyhow::Result<Option<Self>> {
        Ok(match url {
            "yagna" | "lwg" => Some(YagnaBackend {
                url: None,
                token: None,
            }),
            _ => None,
        })
    }
}
