use aptos_sdk::move_types::account_address::AccountAddress as AptosAccountAddress;
use aptos_sdk::move_types::language_storage::StructTag as AptosStructTag;

use move_core_types::resolver::{ModuleResolver, MoveResolver, ResourceResolver};
use move_core_types::account_address::AccountAddress;
use std::{
    collections::{btree_map, BTreeMap},
    fmt::Debug,
};
use std::str::FromStr;
use anyhow::anyhow;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::{ModuleId, StructTag, TypeTag};
use aptos_sdk::rest_client::{Client, MoveModuleBytecode};
use url::Url;
use once_cell::sync::Lazy;
use move_binary_format::CompiledModule;
use move_bytecode_utils::layout::TypeLayoutBuilder;
use move_bytecode_utils::module_cache::{GetModule, ModuleCache};
use tokio::runtime::Runtime;
use move_vm_types::values::Value;

/// Simple in-memory storage for modules and resources under an account.
#[derive(Debug, Clone)]
struct InMemoryAccountStorage {
    resources: BTreeMap<StructTag, Vec<u8>>,
    modules: BTreeMap<Identifier, Vec<u8>>,
}

/// Simple in-memory lazy storage that can be used as a Move VM storage backend for testing purposes. It restores resources from the Aptos chain
#[derive(Debug, Clone)]
pub struct InMemoryLazyStorage {
    accounts: BTreeMap<AccountAddress, InMemoryAccountStorage>,
    #[cfg(feature = "table-extension")]
    tables: BTreeMap<TableHandle, BTreeMap<Vec<u8>, Vec<u8>>>,
}

static NODE_URL: Lazy<Url> = Lazy::new(|| {
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
        if let Some(account_storage) = self.accounts.get(module_id.address()) {
            return Ok(account_storage.modules.get(module_id.name()).cloned());
        }
        // Get account's modules from the chain
        let rest_client = Client::new(NODE_URL.clone());
        let aptos_account = AptosAccountAddress::from_bytes(module_id.address().into_bytes());
        match aptos_account {
            Ok(account_address) => {
                // Runtime::new().unwrap().block_on(rest_client.get_account_modules(account_address))
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
                    println!("load {}::{}", module_id.address(), module_id.name().as_str());
                    return Ok(Option::from(module.bytecode.0));
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
            return Ok(account_storage.resources.get(tag).cloned());
        }
        // Get account's resources from the chain
        let rest_client = Client::new(NODE_URL.clone());
        let aptos_account = AptosAccountAddress::from_bytes(address.into_bytes());
        match aptos_account {
            Ok(account_address) => {
                let matched_resource = Runtime::new().unwrap().block_on(rest_client.get_account_resources_bcs(account_address))
                    .unwrap()
                    .into_inner();
                    // .into_iter()
                    // .find(|resource| {
                    //     resource.resource_type.to_string() == tag.to_string()
                    // });
                    // .get(&AptosStructTag::from_str(tag.to_string().as_str()).unwrap());
                // let storage = InMemoryLazyStorage::new();
                // let layout = TypeLayoutBuilder::build_runtime(&TypeTag::Struct(Box::from(tag.clone())), &storage)
                //     .map_err(|_| anyhow!("Failed to resolve type: {:?}", access_path.root.type_))?;
                if let Some(resource) = matched_resource.get(&AptosStructTag::from_str(tag.to_string().as_str()).unwrap()) {
                    println!("load resource from address{} to get {}", address.to_string(), tag.to_string());
                    return Ok(Option::from(resource.clone()));
                }
                return Ok(None)
            },
            Err(err) => print!("{}", err),
        }
        Ok(None)
    }
}

// impl MoveResolver for InMemoryLazyStorage {
//     type Err = ();
// }
//
// impl GetModule for &InMemoryLazyStorage {
//     type Error = anyhow::Error;
//     type Item = CompiledModule;
//
//     fn get_module_by_id(&self, id: &ModuleId) -> Result<Option<CompiledModule>, Self::Error> {
//         if let Some(bytes) = self.get_module_bytes(id)? {
//             let module = CompiledModule::deserialize(&bytes)
//                 .map_err(|e| anyhow!("Failure deserializing module {:?}: {:?}", id, e))?;
//             Ok(Some(module))
//         } else {
//             Ok(None)
//         }
//     }
// }

#[cfg(feature = "table-extension")]
impl TableResolver for InMemoryLazyStorage {
    fn resolve_table_entry(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> std::result::Result<Option<Vec<u8>>, Error> {
        Ok(self.tables.get(handle).and_then(|t| t.get(key).cloned()))
    }
}


impl InMemoryLazyStorage {
    // pub fn apply_extended(
    //     &mut self,
    //     changeset: ChangeSet,
    //     #[cfg(feature = "table-extension")] table_changes: TableChangeSet,
    // ) -> Result<()> {
    //     for (addr, account_changeset) in changeset.into_inner() {
    //         match self.accounts.entry(addr) {
    //             btree_map::Entry::Occupied(entry) => {
    //                 entry.into_mut().apply(account_changeset)?;
    //             }
    //             btree_map::Entry::Vacant(entry) => {
    //                 let mut account_storage = InMemoryAccountStorage::new();
    //                 account_storage.apply(account_changeset)?;
    //                 entry.insert(account_storage);
    //             }
    //         }
    //     }
    //
    //     #[cfg(feature = "table-extension")]
    //     self.apply_table(table_changes)?;
    //
    //     Ok(())
    // }
    //
    // pub fn apply(&mut self, changeset: ChangeSet) -> Result<()> {
    //     self.apply_extended(
    //         changeset,
    //         #[cfg(feature = "table-extension")]
    //             TableChangeSet::default(),
    //     )
    // }
    //
    // #[cfg(feature = "table-extension")]
    // fn apply_table(&mut self, changes: TableChangeSet) -> Result<()> {
    //     let TableChangeSet {
    //         new_tables,
    //         removed_tables,
    //         changes,
    //     } = changes;
    //     self.tables.retain(|h, _| !removed_tables.contains(h));
    //     self.tables.extend(
    //         new_tables
    //             .keys()
    //             .into_iter()
    //             .map(|h| (*h, BTreeMap::default())),
    //     );
    //     for (h, c) in changes {
    //         assert!(
    //             self.tables.contains_key(&h),
    //             "inconsistent table change set: stale table handle"
    //         );
    //         let table = self.tables.get_mut(&h).unwrap();
    //         apply_changes(table, c.entries)?;
    //     }
    //     Ok(())
    // }

    pub fn new() -> Self {
        Self {
            accounts: BTreeMap::new(),
            #[cfg(feature = "table-extension")]
            tables: BTreeMap::new(),
        }
    }

//     pub fn publish_or_overwrite_module(&mut self, module_id: ModuleId, blob: Vec<u8>) {
//         let account = get_or_insert(&mut self.accounts, *module_id.address(), || {
//             InMemoryAccountStorage::new()
//         });
//         account.modules.insert(module_id.name().to_owned(), blob);
//     }
//
//     pub fn publish_or_overwrite_resource(
//         &mut self,
//         addr: AccountAddress,
//         struct_tag: StructTag,
//         blob: Vec<u8>,
//     ) {
//         let account = get_or_insert(&mut self.accounts, addr, InMemoryAccountStorage::new);
//         account.resources.insert(struct_tag, blob);
//     }
}
