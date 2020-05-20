use crate::{Demand, YagnaEngine};
use chrono::{Datelike, Timelike, Utc};
use std::convert::TryInto;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::time::Duration;
use ya_emscripten_meta as em;
use zip::CompressionMethod;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Manifest {
    /// Deployment id in url like form.
    pub id: String,
    pub name: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entry_points: Vec<EntryPoint>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mount_points: Vec<MountPoint>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct EntryPoint {
    pub id: String,
    pub wasm_path: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum MountPoint {
    Ro(String),
    Rw(String),
    Wo(String),
}

impl MountPoint {
    pub fn path(&self) -> &str {
        match self {
            MountPoint::Ro(path) => path,
            MountPoint::Rw(path) => path,
            MountPoint::Wo(path) => path,
        }
    }
}

fn zip_time_from_path(p: &Path) -> anyhow::Result<zip::DateTime> {
    let mtime: chrono::DateTime<Utc> = wasm_path.metadata()?.modified()?.into();

    let mtime = zip::DateTime::from_date_and_time(
        mtime.year().try_into()?,
        mtime.month().try_into()?,
        mtime.day().try_into()?,
        mtime.hour().try_into()?,
        mtime.minute().try_into()?,
        mtime.second().try_into()?,
    )?;

    Ok(mtime)
}

impl YagnaEngine for gwr_backend::WtEngine {
    fn build_image(wasm_path: &Path) -> anyhow::Result<Vec<u8>> {
        let name_ws = wasm_path.file_name().unwrap().to_string_lossy();

        let m = Manifest {
            id: "wasm-runner/-/todo".to_string(),
            name: name_ws.to_string(),
            entry_points: vec![EntryPoint {
                id: "main".to_string(),
                wasm_path: name_ws.to_string(),
            }],
            mount_points: vec![MountPoint::Ro("in".into()), MountPoint::Rw("out".into())],
        };
        let mtime = zip_time_from_path(wasm_path)?;

        let mut zw = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zw.start_file(
            "manifest.json",
            zip::write::FileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .last_modified_time(mtime.clone()),
        )?;
        serde_json::to_writer_pretty(&mut zw, &m)?;
        zw.start_file(
            name_ws.as_ref(),
            zip::write::FileOptions::default()
                .compression_method(CompressionMethod::Bzip2)
                .last_modified_time(mtime.clone()),
        )?;
        std::io::copy(
            &mut fs::OpenOptions::new().read(true).open(wasm_path)?,
            &mut zw,
        )?;
        let data = zw.finish()?.into_inner();
        Ok(data)
    }

    fn build_demand(node_name: &str, wasm_url: &str, timeout: Duration) -> anyhow::Result<Demand> {
        let expiration = Utc::now()
            + chrono::Duration::from_std(timeout).unwrap_or(chrono::Duration::max_value());

        let mut properties = serde_json::json!({
            "golem": {
                "node.id.name": node_name,
                "srv.comp.wasm.task_package": wasm_url,
                "srv.comp.expiration": expiration.timestamp_millis(),
            },
        });

        Ok(Demand {
            properties,
            constraints: r#"(&
                (golem.inf.mem.gib>0.5)
                (golem.inf.storage.gib>1)
                (golem.com.pricing.model=linear)
            )"#
            .to_string(),
            demand_id: Default::default(),
            requestor_id: Default::default(),
        })
    }
}
