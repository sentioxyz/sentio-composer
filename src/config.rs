use log::error;
use std::collections::HashMap;
use std::fs;

use crate::types::Network;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ConfigData {
    pub config: ToolConfig,
}

#[derive(Deserialize)]
pub struct ToolConfig {
    pub log_folder: Option<String>,
    pub cache_folder: Option<String>,
    pub network_configs: HashMap<Network, String>,
    #[serde(default)]
    pub enable_module_caching: bool
}

impl ToolConfig {
    pub fn default() -> Self {
        let mut network_configs = HashMap::new();
        network_configs.insert(
            Network::Mainnet,
            String::from("https://fullnode.mainnet.aptoslabs.com"),
        );
        network_configs.insert(
            Network::Testnet,
            String::from("https://fullnode.testnet.aptoslabs.com"),
        );
        network_configs.insert(
            Network::Devnet,
            String::from("https://fullnode.devnet.aptoslabs.com"),
        );
        let home_path = match home::home_dir() {
            Some(path) => path.into_os_string().into_string().unwrap(),
            None => String::from("."),
        };
        Self {
            log_folder: Some(String::from(".log")),
            cache_folder: Some(home_path),
            network_configs,
            enable_module_caching: false
        }
    }
}

impl ConfigData {
    pub fn from_file(file_path: &str) -> Self {
        let mut default_config = Self::default();

        let contents = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => {
                panic!("Could not read the config file `{}`", file_path);
            }
        };

        let data: ConfigData = match toml::from_str(&contents) {
            Ok(d) => d,
            Err(e) => {
                panic!("Unable to load data from the config file `{}`, error: {}",
                    file_path,
                    e);
            }
        };
        if let Some(folder) = data.config.log_folder {
            default_config.config.log_folder = Some(folder)
        }
        if let Some(cache_folder) = data.config.cache_folder {
            default_config.config.cache_folder = Some(cache_folder)
        }
        default_config.config.enable_module_caching = data.config.enable_module_caching;
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
                enable_module_caching: false
            },
        }
    }

    pub fn default() -> Self {
        Self {
            config: ToolConfig::default(),
        }
    }
}
