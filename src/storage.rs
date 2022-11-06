use aptos_sdk::move_types::account_address::AccountAddress as AptosAccountAddress;
use aptos_sdk::move_types::language_storage::StructTag as AptosStructTag;

use move_core_types::resolver::{ModuleResolver, ResourceResolver};
use move_core_types::account_address::AccountAddress;
use std::{collections::{btree_map, BTreeMap}, fmt::Debug};
use std::str::FromStr;
use anyhow::{bail, Result};
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::{ModuleId, StructTag};
use aptos_sdk::rest_client::{Client, MoveModuleBytecode};
use log::{error, info};
use url::Url;
use once_cell::sync::Lazy;
use move_core_types::effects::{AccountChangeSet, ChangeSet, Op};
use tokio::runtime::Runtime;
use cacache;

/// Simple in-memory storage for modules and resources under an account.
#[derive(Debug, Clone)]
struct InMemoryAccountStorage {
    resources: BTreeMap<StructTag, Vec<u8>>,
    modules: BTreeMap<Identifier, Vec<u8>>
}

fn apply_changes<K, V>(
    map: &mut BTreeMap<K, V>,
    changes: impl IntoIterator<Item = (K, Op<V>)>,
) -> Result<()>
    where
        K: Ord + Debug,
{
    use btree_map::Entry::*;
    use Op::*;

    for (k, op) in changes.into_iter() {
        match (map.entry(k), op) {
            (Occupied(entry), New(_)) => {
                bail!(
                    "Failed to apply changes -- key {:?} already exists",
                    entry.key()
                )
            }
            (Occupied(entry), Delete) => {
                entry.remove();
            }
            (Occupied(entry), Modify(val)) => {
                *entry.into_mut() = val;
            }
            (Vacant(entry), New(val)) => {
                entry.insert(val);
            }
            (Vacant(entry), Delete | Modify(_)) => bail!(
                "Failed to apply changes -- key {:?} does not exist",
                entry.key()
            ),
        }
    }
    Ok(())
}

impl InMemoryAccountStorage {
    fn apply(&mut self, account_changeset: AccountChangeSet) -> Result<()> {
        let (modules, resources) = account_changeset.into_inner();
        apply_changes(&mut self.modules, modules)?;
        apply_changes(&mut self.resources, resources)?;
        Ok(())
    }

    fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
            resources: BTreeMap::new()
        }
    }
}

/// Simple in-memory lazy storage that can be used as a Move VM storage backend for testing purposes. It restores resources from the Aptos chain
#[derive(Clone)]
pub struct InMemoryLazyStorage {
    accounts: BTreeMap<AccountAddress, InMemoryAccountStorage>,
    ledger_version: u64
}

impl InMemoryLazyStorage {
    pub fn apply_extended(
        &mut self,
        changeset: ChangeSet,
    ) -> Result<()> {
        for (addr, account_changeset) in changeset.into_inner() {
            match self.accounts.entry(addr) {
                btree_map::Entry::Occupied(entry) => {
                    entry.into_mut().apply(account_changeset)?;
                }
                btree_map::Entry::Vacant(entry) => {
                    let mut account_storage = InMemoryAccountStorage::new();
                    account_storage.apply(account_changeset)?;
                    entry.insert(account_storage);
                }
            }
        }

        Ok(())
    }

    pub fn new(ledger_version: u64) -> Self {
        Self {
            accounts: BTreeMap::new(),
            ledger_version
        }
    }
}

pub static NODE_URL: Lazy<Url> = Lazy::new(|| {
    Url::from_str(
        std::env::var("APTOS_NODE_URL")
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("https://fullnode.mainnet.aptoslabs.com"),
    )
        .unwrap()
});

impl ModuleResolver for InMemoryLazyStorage {
    type Error = ();

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Vec<u8>>, Self::Error> {
        // We should never hit this as there's no in-memory cache
        if let Some(account_storage) = self.accounts.get(module_id.address()) {
            let cached_module =  account_storage.modules.get(module_id.name()).cloned();
            match cached_module {
                None => {}
                Some(m) => {
                    return Ok(Some(m));
                }
            }
        }
        // Get modules from the local cache
        let mut module_cache_key = module_id.address().to_string().to_owned();
        let module_name = module_id.name().as_str();
        module_cache_key.push_str(module_name);
        // if self.ledger_version > 0 {
        //     module_cache_key.push_str(self.ledger_version.to_string().as_str())
        // }
        let cached_module = cacache::read_sync("./modules-cache", module_cache_key.clone());
        match cached_module {
            Ok(m) => {
                info!("loaded module from cache: {}::{}", module_id.address(), module_id.name().as_str());
                return Ok(Some(m));
            }
            Err(e) => {
                error!("{}", e);
            }
        }

        // Get account's modules from the chain
        let rest_client = Client::new(NODE_URL.clone());
        let aptos_account = AptosAccountAddress::from_bytes(module_id.address().into_bytes());
        match aptos_account {
            Ok(account_address) => {
                let matched_module = Runtime::new().unwrap().block_on(rest_client.get_account_modules(account_address))
                    .unwrap()
                    .into_inner()
                    .into_iter()
                    .find(|module| {
                        // MoveModuleBytecode::new(Vec::from(module.bytecode.0.as_bytes())).try_parse_abi()
                        if let Ok(mod_) = MoveModuleBytecode::new(module.bytecode.0.to_vec()).try_parse_abi() {
                            return mod_.abi.unwrap().name.as_str() == module_id.name().as_str()
                        }
                        false
                    });
                if let Some(module) = matched_module {
                    info!("load module: {}::{}", module_id.address(), module_id.name().as_str());
                    // caching the module
                    cacache::write_sync("./modules-cache", module_cache_key.clone(), module.bytecode.0.clone()).expect("Failed to cache the module");
                    return Ok(Option::from(module.bytecode.0.clone()));
                }
            },
            Err(err) => print!("{}", err),
        }
        Ok(None)
    }
}

impl ResourceResolver for InMemoryLazyStorage {
    type Error = ();

    fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        if let Some(account_storage) = self.accounts.get(address) {
            let cached_resource = account_storage.resources.get(tag).cloned();
            match cached_resource {
                None => {}
                Some(r) => {
                    return Ok(Some(r));
                }
            }
        }
        // Get account's resources from the chain
        let rest_client = Client::new(NODE_URL.clone());
        let aptos_account = AptosAccountAddress::from_bytes(address.into_bytes());
        match aptos_account {
            Ok(account_address) => {
                let matched_resource;
                if self.ledger_version > 0 {
                    matched_resource = Runtime::new().unwrap().block_on(rest_client.get_account_resources_at_version_bcs(account_address, self.ledger_version))
                        .unwrap()
                        .into_inner();
                } else {
                    matched_resource = Runtime::new().unwrap().block_on(rest_client.get_account_resources_bcs(account_address))
                        .unwrap()
                        .into_inner();
                }
                if let Some(resource) = matched_resource.get(&AptosStructTag::from_str(tag.to_string().as_str()).unwrap()) {
                    info!("load resource from address{} to get {}", address.to_string(), tag.to_string());
                    return Ok(Option::from(resource.clone()));
                }
                return Ok(None)
            },
            Err(err) => print!("{}", err),
        }
        Ok(None)
    }
}
