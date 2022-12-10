use crate::types::Network;
use anyhow::{anyhow, Result};
use aptos_sdk::rest_client::aptos_api_types::MoveModule;
use aptos_sdk::rest_client::{Client, MoveModuleBytecode};
use log::{debug, warn};
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::ModuleId;
use parking_lot::RwLock;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use tokio::runtime::Runtime;

pub struct CacheModuleResolver {
    network: Network,
    client: Client,
    cache_folder: String,
    module_cache: RwLock<HashMap<ModuleId, (Option<Vec<u8>>, Option<MoveModule>)>>,
    enable_module_caching: bool,
}

impl Clone for CacheModuleResolver {
    fn clone(&self) -> Self {
        Self {
            network: self.network,
            client: self.client.clone(),
            cache_folder: self.cache_folder.clone(),
            module_cache: RwLock::new(self.module_cache.read().clone()),
            enable_module_caching: self.enable_module_caching,
        }
    }
}

impl CacheModuleResolver {
    pub fn new(
        network: &Network,
        client: Client,
        cache_folder: String,
        enable_module_caching: bool,
    ) -> Self {
        Self {
            network: *network,
            client,
            cache_folder,
            module_cache: RwLock::new(HashMap::new()),
            enable_module_caching,
        }
    }

    pub fn get_module(
        &self,
        module_id: &ModuleId,
    ) -> Result<(Option<Vec<u8>>, Option<MoveModule>)> {
        let locked_cache = self.module_cache.read();
        if let Some(res) = locked_cache.get(module_id) {
            debug!("loading module {} from memory cache", module_id);
            return match res {
                (Some(bytecode), Some(abi)) => Ok((Some(bytecode.clone()), Some(abi.clone()))),
                _ => Ok((None, None)),
            };
        }
        drop(locked_cache);
        let addr = module_id.address();
        // Get module from the local cache if:
        // 1. enable the caching 2. it belongs to standard module
        if self.is_cached_module(addr) {
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
                    abi = mod_.abi.clone();
                    let module_address = abi.as_ref().unwrap().address.to_string();
                    let module_name = abi.as_ref().unwrap().name.as_str();
                    let inner_module_id = &ModuleId::new(
                        AccountAddress::from_str(module_address.as_str()).unwrap(),
                        Identifier::from_str(module_name).unwrap(),
                    );
                    // caching the module into memory before head
                    let mut locked_cache = self.module_cache.write();
                    locked_cache.insert(
                        inner_module_id.clone(),
                        (Some(module.bytecode.0.clone()), abi.clone()),
                    );
                    drop(locked_cache);

                    // caching the standard module to disk
                    if self.is_cached_module(addr) {
                        self.write_module_cache_to_disk(inner_module_id, module.bytecode.0.clone());
                    }
                    return module_name == module_id.name().as_str();
                }
                false
            });
        if let Some(module) = matched_module {
            debug!("load module: {}::{}", addr, module_id.name().as_str());
            return Ok((Option::from(module.bytecode.0.clone()), abi));
        }
        Ok((None, abi))
    }

    fn is_cached_module(&self, addr: &AccountAddress) -> bool {
        self.enable_module_caching
            || addr.to_hex_literal() == "0x1"
            || addr.to_hex_literal() == "0x3"
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
                debug!("loaded module from disk cache: {}", module_id);
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

    fn write_module_cache_to_disk(&self, module_id: &ModuleId, bytecode: Vec<u8>) {
        debug!("Caching {} to disk", module_id);
        cacache::write_sync(
            self.get_cache_path(self.cache_folder.clone()),
            self.get_cache_key(module_id),
            bytecode,
        )
        .expect("Failed to cache the module");
    }

    fn get_cache_key(&self, module_id: &ModuleId) -> String {
        let mut module_cache_key = module_id.to_string();
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
