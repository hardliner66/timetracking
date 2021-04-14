use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Default, Debug, Deserialize)]
pub struct Settings {
    pub data_file: String,
    pub auto_insert_stop: bool,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        // Start off by merging in the "default" configuration file
        #[cfg(feature = "binary")]
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

        s.merge(File::with_name(".timetracking.config").required(false))?;

        s.merge(Environment::with_prefix("tt"))?;

        // You can deserialize (and thus freeze) the entire configuration as
        s.try_into()
    }
}
