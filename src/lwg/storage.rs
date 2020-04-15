use actix_http::HttpMessage;
use futures::prelude::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub struct DistStorage {
    url: Arc<str>,
}

pub struct DistSlot {
    upload_url: String,
    download_url: String,
}

impl DistSlot {
    pub fn url(&self) -> &str {
        self.upload_url.as_str()
    }

    pub async fn download(&self, out_path: &Path) -> anyhow::Result<()> {
        let c = awc::Client::new();

        let mut response = c
            .get(&self.download_url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("download json: {}", e))?;

        let payload = response.take_payload();
        let mut fs = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(out_path)?;
        Ok(payload
            .for_each(|b| {
                let bytes = b.unwrap();
                fs.write_all(bytes.as_ref()).unwrap();
                future::ready(())
            })
            .await)
    }

    pub async fn download_json<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
        let c = awc::Client::new();
        let b = c
            .get(&self.download_url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("download json: {}", e))?
            .body()
            .await
            .map_err(|e| anyhow::anyhow!("download json: {}", e))?;

        Ok(serde_json::from_slice(b.as_ref())?)
    }
}

impl DistStorage {
    pub fn new(storage_url: Arc<str>) -> Self {
        let url = storage_url;
        Self { url }
    }

    async fn upload_bytes(&self, prefix: &str, bytes: Vec<u8>) -> anyhow::Result<String> {
        let c = awc::Client::new();
        let id = uuid::Uuid::new_v4();
        let upload_url = format!("{}upload/{}-{}", self.url, prefix, id);

        let response = c
            .put(&upload_url)
            .content_length(bytes.len() as u64)
            .content_type("application/octet-stream")
            .send_body(bytes)
            .await
            .map_err(|e| anyhow::anyhow!("upload bytes: {}", e))?;

        Ok(format!("{}{}-{}", self.url, prefix, id))
    }

    pub async fn upload_file(&self, path: &Path) -> anyhow::Result<String> {
        self.upload_bytes("blob", std::fs::read(path)?).await
    }

    pub async fn upload_json<T: Serialize>(&self, obj: &T) -> anyhow::Result<String> {
        let bytes = serde_json::to_vec_pretty(obj)?;
        self.upload_bytes("json", bytes).await
    }

    pub async fn download_slot(&self) -> anyhow::Result<DistSlot> {
        let id = uuid::Uuid::new_v4();
        let upload_url = format!("{}upload/out-{}", self.url, id);
        let download_url = format!("{}out-{}", self.url, id);
        Ok(DistSlot {
            upload_url,
            download_url,
        })
    }
}
