use log::error;
use std::collections::HashMap;
use std::fs;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct ConfigData {
    pub config: ToolConfig,
}

#[derive(Deserialize)]
pub struct ToolConfig {
    pub log_folder: Option<String>,
    pub cache_folder: Option<String>,
    pub network_configs: HashMap<String, String>,
}

impl ConfigData {
    pub fn from_file(file_path: &str) -> Self {
        let mut default_config = Self::default();

        let contents = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => {
                error!("Could not read the config file `{}`", file_path);
                String::new()
            }
        };

        let data: ConfigData = match toml::from_str(&contents) {
            Ok(d) => d,
            Err(_) => {
                error!("Unable to load data from the config file `{}`", file_path);
                Self::new()
            }
        };
        if let Some(folder) = data.config.log_folder {
            default_config.config.log_folder = Some(folder)
        }
        if let Some(cache_folder) = data.config.cache_folder {
            default_config.config.cache_folder = Some(cache_folder)
        }
        default_config
            .config
            .network_configs
            .extend(data.config.network_configs);
        default_config
    }

    pub fn new() -> Self {
        Self {
            config: ToolConfig {
                log_folder: None,
                cache_folder: None,
                network_configs: HashMap::new(),
            },
        }
    }

    pub fn default() -> Self {
        let mut network_configs = HashMap::new();
        network_configs.insert(
            String::from("mainnet"),
            String::from("https://fullnode.mainnet.aptoslabs.com"),
        );
        network_configs.insert(
            String::from("testnet"),
            String::from("https://fullnode.testnet.aptoslabs.com"),
        );
        network_configs.insert(
            String::from("devnet"),
            String::from("https://fullnode.devnet.aptoslabs.com"),
        );
        let home_path = match home::home_dir() {
            Some(path) => path.into_os_string().into_string().unwrap(),
            None => String::from("."),
        };
        Self {
            config: ToolConfig {
                log_folder: Some(String::from(".log")),
                cache_folder: Some(home_path),
                network_configs,
            },
        }
    }
}
