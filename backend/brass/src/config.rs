use app_dirs::{app_dir, AppDataType, AppInfo};
use std::path::Path;
use {
    gwasm_api::prelude::{Net, Timeout},
    serde::{Deserialize, Serialize},
    std::{fs::File, path::PathBuf, str::FromStr},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct GolemConfig {
    #[serde(default = "default_address")]
    pub address: String,
    #[serde(default = "default_bid")]
    pub bid: f64,
    #[serde(default = "default_budget")]
    pub budget: f64,
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(default = "default_net", with = "serde_with::rust::display_fromstr")]
    pub net: Net,
    #[serde(
        default = "default_task_timeout",
        with = "serde_with::rust::display_fromstr"
    )]
    pub task_timeout: Timeout,
    #[serde(
        default = "default_subtask_timeout",
        with = "serde_with::rust::display_fromstr"
    )]
    pub subtask_timeout: Timeout,
}

impl GolemConfig {
    pub fn from_file(config_path: impl AsRef<Path>) -> anyhow::Result<GolemConfig> {
        if config_path.as_ref().exists() {
            let user_config: GolemConfig = serde_json::from_reader(File::open(config_path)?)?;
            return Ok(user_config);
        }
        Ok(GolemConfig::default())
    }
}

impl Default for GolemConfig {
    fn default() -> GolemConfig {
        GolemConfig {
            address: default_address(),
            bid: default_bid(),
            budget: default_budget(),
            data_dir: default_data_dir(),
            name: default_name(),
            net: default_net(),
            task_timeout: default_task_timeout(),
            subtask_timeout: default_subtask_timeout(),
        }
    }
}

fn default_address() -> String {
    String::from("127.0.0.1:61000")
}

fn default_bid() -> f64 {
    1.0
}

fn default_budget() -> f64 {
    1.0
}

const GOLEM_APP_INFO: AppInfo = AppInfo {
    name: "golem",
    author: "golem",
};

fn default_data_dir() -> PathBuf {
    app_dir(AppDataType::UserData, &GOLEM_APP_INFO, "default").unwrap()
}

fn default_name() -> String {
    String::from("gwasm-task")
}

fn default_net() -> Net {
    Net::TestNet
}

fn default_task_timeout() -> Timeout {
    Timeout::from_str("00:30:00").unwrap()
}

fn default_subtask_timeout() -> Timeout {
    Timeout::from_str("00:10:00").unwrap()
}
