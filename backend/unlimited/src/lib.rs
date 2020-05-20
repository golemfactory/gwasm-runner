use gwr_backend::{rt, Flags};
use std::path::Path;

mod runner;

#[derive(Debug, Clone)]
pub struct GuBackend {
    hub_url: String,
}

impl GuBackend {
    pub fn parse_url(url: &str) -> anyhow::Result<Option<Self>> {
        if url.starts_with("gu://") {
            let tail = &url[5..];
            let hub_url = if !tail.contains(':') {
                format!("{}:61622", tail)
            } else {
                tail.to_string()
            };
            return Ok(Some(GuBackend { hub_url }));
        }
        Ok(match url {
            "GU" | "Unlimited" | "GolemUnlimited" => Some(GuBackend {
                hub_url: std::env::var("GU_HUB_ADDR")?,
            }),
            _ => None,
        })
    }

    pub fn run<E: rt::Engine>(
        &self,engine: E, flags: &Flags, wasm_path: &Path, args: &[String],) -> anyhow::Result<()> {
        runner::run(engine, self.hub_url.clone(), wasm_path,  args)
    }
}
