use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

use shared::utility::is_running_under_cargo;
use tokio::sync::Mutex;

use crate::config::detail_config::GlobalConfig;

#[derive(Clone)]
pub struct Config {
    pub working_dir: PathBuf,
    pub workspace_dir: PathBuf,
    pub public_dir: PathBuf,
    pub index_file: PathBuf,
    pub config_file: PathBuf,

    pub config: Arc<Mutex<GlobalConfig>>,
}

impl Config {
    pub fn load() -> (Self, bool) {
        let mut working_dir = std::env::current_dir().unwrap();
        
        if is_running_under_cargo() {
            working_dir = working_dir.join("test");
        }

        let workspace_dir = working_dir.join("workspace");
        let public_dir = working_dir.join("public");
        let index_file = working_dir.join("public/index.json");
        let config_file = working_dir.join("config.toml");

        std::fs::create_dir_all(&workspace_dir).unwrap();
        std::fs::create_dir_all(&public_dir).unwrap();

        let first_run = !config_file.exists();

        if first_run {
            let default_content = toml::to_string_pretty(&GlobalConfig::default()).unwrap();

            std::fs::write(&config_file, default_content).unwrap();
        }

        let config_content = std::fs::read_to_string(&config_file).unwrap();
        let config = toml::from_str::<GlobalConfig>(&config_content).unwrap();

        let config = Config {
            working_dir,
            workspace_dir,
            public_dir,
            index_file,
            config_file,
            config: Arc::new(Mutex::new(config)),
        };

        (config, first_run)
    }

    pub fn save(&self) {
        let mut config = self.config.blocking_lock();

        self.do_save(&mut config);
    }

    pub async fn save_async(&self) {
        let mut config = self.config.lock().await;

        self.do_save(&mut config);
    }

    fn do_save(&self, config: &mut GlobalConfig) {
        let default_content = toml::to_string_pretty(&config.deref()).unwrap();

        std::fs::write(&self.config_file, default_content).unwrap();
    }
}

