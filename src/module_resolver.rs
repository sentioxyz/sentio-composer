use crate::types::Network;
use anyhow::{anyhow, Result};
use aptos_sdk::rest_client::aptos_api_types::MoveModule;
use aptos_sdk::rest_client::{Client, MoveModuleBytecode};
use log::{debug, warn};
use move_core_types::language_storage::ModuleId;
use std::collections::HashMap;
use std::path::Path;
use tokio::runtime::Runtime;

#[derive(Clone, Debug)]
pub struct CacheModuleResolver {
    network: Network,
    client: Client,
    cache_folder: String,
    module_cache: HashMap<ModuleId, (Option<Vec<u8>>, Option<MoveModule>)>,
}

impl CacheModuleResolver {
    pub fn new(network: &Network, client: Client, cache_folder: String) -> Self {
        Self {
            network: *network,
            client,
            cache_folder,
            module_cache: HashMap::new(),
        }
    }

    pub fn get_module(
        &mut self,
        module_id: &ModuleId,
    ) -> Result<(Option<Vec<u8>>, Option<MoveModule>)> {
        if let Some(res) = self.module_cache.get(module_id) {
            match res {
                (Some(bytecode), Some(abi)) => {
                    return Ok((Some(bytecode.clone()), Some(abi.clone())));
                }
                _ => Ok((None, None)),
            }
        } else {
            let addr = module_id.address();
            // Get modules from the local cache if it starts from 0x1 or 0x3
            if addr.to_hex_literal() == "0x1" || addr.to_hex_literal() == "0x3" {
                if let Some(res) = self.try_load_module_from_disk_cache(module_id) {
                    return Ok(res);
                }
            }
            use aptos_sdk::move_types::account_address::AccountAddress as AptosAccountAddress;
            let aptos_account = AptosAccountAddress::from_bytes(addr.into_bytes()).unwrap();
            let mut abi: Option<MoveModule> = None;
            let matched_module = Runtime::new()
                .unwrap()
                .block_on(self.client.get_account_modules(aptos_account))
                .unwrap()
                .into_inner()
                .into_iter()
                .find(|module| {
                    if let Ok(mod_) =
                        MoveModuleBytecode::new(module.bytecode.0.to_vec()).try_parse_abi()
                    {
                        abi = mod_.abi;
                        // caching the module into memory before head
                        self.module_cache.insert(
                            module_id.clone(),
                            (Some(module.bytecode.0.clone()), abi.clone()),
                        );
                        return abi.as_ref().unwrap().name.as_str() == module_id.name().as_str();
                    }
                    false
                });
            if let Some(module) = matched_module {
                debug!("load module: {}::{}", addr, module_id.name().as_str());
                if addr.to_hex_literal() == "0x1" || addr.to_hex_literal() == "0x3" {
                    self.cache_module_to_disk(module_id, module.bytecode.0.clone());
                }
                return Ok((Option::from(module.bytecode.0.clone()), abi));
            }
            Ok((None, abi))
        }
    }

    fn try_load_module_from_disk_cache(
        &self,
        module_id: &ModuleId,
    ) -> Option<(Option<Vec<u8>>, Option<MoveModule>)> {
        let module_cache_key = self.get_cache_key(module_id);
        let cache_path = self.get_cache_path(self.cache_folder.clone());
        let cached_module = cacache::read_sync(cache_path, module_cache_key);
        match cached_module {
            Ok(m) => {
                debug!(
                    "loaded module from cache: {}::{}",
                    module_id.address(),
                    module_id.name().as_str()
                );
                let _abi = MoveModuleBytecode::new(m.clone())
                    .try_parse_abi()
                    .unwrap()
                    .abi;
                return Some((Some(m), _abi));
            }
            Err(e) => {
                warn!("{}", e);
            }
        }
        None
    }

    fn cache_module_to_disk(&self, module_id: &ModuleId, bytecode: Vec<u8>) {
        cacache::write_sync(
            self.get_cache_path(self.cache_folder.clone()),
            self.get_cache_key(module_id),
            bytecode,
        )
        .expect("Failed to cache the module");
    }

    fn get_cache_key(&self, module_id: &ModuleId) -> String {
        let mut module_cache_key = module_id.address().to_string().to_owned();
        let module_name = module_id.name().as_str();
        module_cache_key.push_str(module_name);
        module_cache_key.push_str(self.network.as_str());
        module_cache_key
    }

    fn get_cache_path(&self, cache_folder: String) -> String {
        return Path::new(&cache_folder)
            .join(".move-modules-cache")
            .into_os_string()
            .into_string()
            .unwrap();
    }
}
