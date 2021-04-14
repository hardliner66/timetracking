use config::{Config, ConfigError, Environment, File, FileFormat};
use serde::Deserialize;

use std::path::Path;

#[derive(Default, Debug, Deserialize)]
pub struct Settings {
    pub data_file: String,
    pub auto_insert_stop: bool,
    pub enable_project_settings: bool,
}

fn add_file_if_exists(s: &mut Config, file: &str) -> Result<bool, ConfigError> {
    let result = if Path::new(file).exists() {
        s.merge(File::new(file, FileFormat::Toml).required(false))?;
        true
    } else {
        false
    };
    Ok(result)
}

fn path_to_string_lossy<P: AsRef<Path>>(path: P) -> String {
    path.as_ref().to_string_lossy().to_string()
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        // Start off by merging in the "default" configuration file
        s.merge(File::from_str(
            include_str!("../default_config.toml"),
            config::FileFormat::Toml,
        ))?;

        #[cfg(not(feature = "binary"))]
        s.merge(File::from_str(
            include_str!("../default_config_development.toml"),
            config::FileFormat::Toml,
        ))?;

        let config_path = shellexpand::full("~/.config/timetracking/config.toml")
            .expect("could not expand path")
            .to_string();
        s.merge(File::with_name(config_path.as_str()).required(false))?;

        if s.get_bool("enable_project_settings")? {
            let current_dir = std::env::current_dir().expect("Could not get current directory");
            let mut path = current_dir.as_path();
            if !add_file_if_exists(
                &mut s,
                &format!("{}/timetracking.project.toml", path_to_string_lossy(&path)),
            )? {
                while let Some(parent) = path.parent() {
                    if add_file_if_exists(
                        &mut s,
                        &format!("{}/timetracking.project.toml", path_to_string_lossy(&path)),
                    )? {
                        break;
                    }
                    path = parent;
                }
            }
        }

        s.merge(File::with_name(".timetracking.config").required(false))?;

        s.merge(Environment::with_prefix("tt"))?;

        // You can deserialize (and thus freeze) the entire configuration as
        s.try_into()
    }
}
