use app_dirs::{app_dir, AppDataType::UserCache, AppInfo};
use failure::Fallible;
use std::fs;
use std::path::PathBuf;

const APP_INFO: AppInfo = AppInfo {
    name: "g-wasm-runner",
    author: "Golem Factory",
};

pub struct WorkDir {
    task_type: &'static str,
    base: PathBuf,
}

impl WorkDir {
    pub fn new(task_type: &'static str) -> Fallible<Self> {
        let uuid = uuid::Uuid::new_v4();
        let base =
            app_dir(UserCache, &APP_INFO, task_type)?.join(uuid.to_hyphenated_ref().to_string());
        Ok(WorkDir { base, task_type })
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
            app_dir(UserCache, &APP_INFO, "/local").unwrap().display()
        )
    }

}
