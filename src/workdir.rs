use app_dirs::{app_dir, AppDataType::UserCache, AppInfo};
use failure::Fallible;
use std::fs;
use std::path::PathBuf;

pub const GWASM_APP_INFO: AppInfo = AppInfo {
    name: "g-wasm-runner",
    author: "Golem Factory",
};

#[derive(Debug, Clone)]
pub struct WorkDir {
    base: PathBuf,
}

impl WorkDir {
    pub fn new(task_type: &'static str) -> Fallible<Self> {
        let uuid = uuid::Uuid::new_v4();
        let base =
            app_dir(UserCache, &GWASM_APP_INFO, task_type)?.join(uuid.to_hyphenated_ref().to_string());
        Ok(WorkDir { base })
    }

    pub fn base_dir(&self) -> &PathBuf {
        &self.base
    }

    pub fn split_output(&mut self) -> Fallible<PathBuf> {
        let output = self.base.join("split");

        fs::create_dir_all(&output)?;
        Ok(output)
    }

    pub fn merge_path(&mut self) -> Fallible<PathBuf> {
        let output = self.base.join("merge");

        fs::create_dir_all(&output)?;
        Ok(output)
    }

    pub fn new_task(&mut self) -> Fallible<PathBuf> {
        let uuid = format!("tsk-{}", uuid::Uuid::new_v4().to_hyphenated());
        let task_path = self.base.join(uuid);
        fs::create_dir(&task_path)?;
        Ok(task_path)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_app_dir() {
        eprintln!(
            "dir={}",
            app_dir(UserCache, &GWASM_APP_INFO, "/local").unwrap().display()
        )
    }

}
