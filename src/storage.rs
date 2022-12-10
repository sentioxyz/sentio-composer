use aptos_sdk::move_types::account_address::AccountAddress as AptosAccountAddress;
use aptos_sdk::move_types::language_storage::StructTag as AptosStructTag;

use crate::module_resolver::CacheModuleResolver;
use crate::types::Network;
use anyhow::{bail, Error, Result};
use aptos_sdk::rest_client::aptos_api_types::mime_types::BCS;
use aptos_sdk::rest_client::Client;
use log::{debug, error};
use move_core_types::account_address::AccountAddress;
use move_core_types::effects::{AccountChangeSet, ChangeSet, Op};
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::{ModuleId, StructTag, TypeTag};
use move_core_types::resolver::{ModuleResolver, ResourceResolver};
use move_table_extension::{TableHandle, TableResolver};
use reqwest::header::ACCEPT;
use reqwest::StatusCode;
use std::borrow::BorrowMut;
use std::cell::Cell;
use std::collections::HashMap;
use std::str::FromStr;
use std::{
    collections::{btree_map, BTreeMap},
    fmt::Debug,
};
use std::sync::RwLock;
use tokio::runtime::Runtime;

/// Simple in-memory storage for modules and resources under an account.
#[derive(Debug, Clone)]
struct InMemoryAccountStorage {
    resources: BTreeMap<StructTag, Vec<u8>>,
    modules: BTreeMap<Identifier, Vec<u8>>,
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
            resources: BTreeMap::new(),
        }
    }
}

/// Simple in-memory lazy storage that can be used as a Move VM storage backend. It restores resources from the Aptos chain
// #[derive(Clone)]
pub struct InMemoryLazyStorage {
    accounts: BTreeMap<AccountAddress, InMemoryAccountStorage>,
    ledger_version: u64,
    network: Network,
    client: Client,
    module_resolver: CacheModuleResolver,
}

impl InMemoryLazyStorage {
    pub fn apply_extended(&mut self, changeset: ChangeSet) -> Result<()> {
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

    pub fn new(
        ledger_version: u64,
        network: Network,
        client: Client,
        module_resolver: CacheModuleResolver
    ) -> Self {
        Self {
            accounts: BTreeMap::new(),
            ledger_version,
            network,
            client,
            module_resolver
        }
    }
}

impl ModuleResolver for InMemoryLazyStorage {
    type Error = ();

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Vec<u8>>, Self::Error> {
        let (mod_, _) = self.module_resolver.get_module(module_id).unwrap();

        Ok(mod_)
    }
}

impl ResourceResolver for InMemoryLazyStorage {
    type Error = ();

    fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        // Get account's resources from the chain
        let rest_client = self.client.clone();
        let aptos_account = AptosAccountAddress::from_bytes(address.into_bytes());
        match aptos_account {
            Ok(account_address) => {
                let matched_resource;
                if self.ledger_version > 0 {
                    matched_resource = Runtime::new()
                        .unwrap()
                        .block_on(rest_client.get_account_resources_at_version_bcs(
                            account_address,
                            self.ledger_version,
                        ))
                        .unwrap()
                        .into_inner();
                } else {
                    matched_resource = Runtime::new()
                        .unwrap()
                        .block_on(rest_client.get_account_resources_bcs(account_address))
                        .unwrap()
                        .into_inner();
                }
                if let Some(resource) = matched_resource
                    .get(&AptosStructTag::from_str(tag.to_string().as_str()).unwrap())
                {
                    debug!(
                        "load resource from address {} to get {}",
                        address.to_string(),
                        tag.to_string()
                    );
                    return Ok(Option::from(resource.clone()));
                }
                return Ok(None);
            }
            Err(err) => error!("{}", err),
        }
        Ok(None)
    }
}

impl TableResolver for InMemoryLazyStorage {
    fn resolve_table_entry(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> std::result::Result<Option<Vec<u8>>, Error> {
        let url_string = if self.ledger_version > 0 {
            format!(
                "https://fullnode.{}.aptoslabs.com/v1/tables/0x{}/raw_item?ledger_version={}",
                self.network, handle.0, self.ledger_version
            )
        } else {
            format!(
                "https://fullnode.{}.aptoslabs.com/v1/tables/0x{}/raw_item",
                self.network, handle.0
            )
        };
        let c = reqwest::blocking::Client::new();
        let mut map = HashMap::new();
        map.insert("key", hex::encode(key));
        let resp = c
            .post(url_string)
            .header(ACCEPT, BCS)
            .json(&map)
            .send()
            .unwrap();
        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let bytes = resp.bytes().unwrap();
        Ok(Some(bytes.to_vec()))
    }
}
