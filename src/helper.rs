use crate::config::ToolConfig;
use aptos_sdk::rest_client::aptos_api_types::{MoveModule, MoveType};
use aptos_sdk::rest_client::{Client, MoveModuleBytecode};
use home;
use log::{error, info};
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::{IdentStr, Identifier};
use move_core_types::language_storage::{ModuleId, StructTag, TypeTag};
use move_core_types::value::MoveValue;
use path_clean::PathClean;
use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tokio::runtime::Runtime;
use url::Url;

pub fn absolute_path(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let path = path.as_ref();

    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()?.join(path)
    }
    .clean();

    Ok(absolute_path)
}

pub fn get_node_url(network: String, config: &ToolConfig) -> Url {
    if let Some(url) = config.network_configs.get(&*network) {
        info!("Use client url: {}", url);
        return Url::from_str(url.as_str()).unwrap();
    }
    panic!("Cannot find the network URL")
}

type Error = ();

pub fn cache_folder() -> String {
    return match home::home_dir() {
        Some(path) => path
            .join(".move-modules-cache")
            .into_os_string()
            .into_string()
            .unwrap(),
        None => String::from(".move-modules-cache"),
    };
}

pub fn get_function_module(
    client: Client,
    module_id: &ModuleId,
    network: String,
) -> Result<(Option<Vec<u8>>, Option<MoveModule>), Error> {
    // Get modules from the local cache
    let mut module_cache_key = module_id.address().to_string().to_owned();
    let module_name = module_id.name().as_str();
    module_cache_key.push_str(module_name);
    module_cache_key.push_str(network.as_str());
    let cached_module = cacache::read_sync(cache_folder(), module_cache_key.clone());
    match cached_module {
        Ok(m) => {
            info!(
                "loaded module from cache: {}::{}",
                module_id.address(),
                module_id.name().as_str()
            );
            let _abi = MoveModuleBytecode::new(m.clone())
                .try_parse_abi()
                .unwrap()
                .abi;
            return Ok((Some(m), _abi));
        }
        Err(e) => {
            error!("{}", e);
        }
    }

    use aptos_sdk::move_types::account_address::AccountAddress as AptosAccountAddress;
    let aptos_account = AptosAccountAddress::from_bytes(module_id.address().into_bytes()).unwrap();
    let mut abi: Option<MoveModule> = None;
    let matched_module = Runtime::new()
        .unwrap()
        .block_on(client.get_account_modules(aptos_account))
        .unwrap()
        .into_inner()
        .into_iter()
        .find(|module| {
            if let Ok(mod_) = MoveModuleBytecode::new(module.bytecode.0.to_vec()).try_parse_abi() {
                abi = mod_.abi;
                return abi.as_ref().unwrap().name.as_str() == module_id.name().as_str();
            }
            false
        });
    if let Some(module) = matched_module {
        // module.try_parse_abi()
        info!(
            "load module: {}::{}",
            module_id.address(),
            module_id.name().as_str()
        );
        // caching the module
        cacache::write_sync(
            cache_folder(),
            module_cache_key.clone(),
            module.bytecode.0.clone(),
        )
        .expect("Failed to cache the module");
        return Ok((Option::from(module.bytecode.0.clone()), abi));
    }
    Ok((None, abi))
}

pub fn construct_struct_type_tag_from_str(raw: &str) -> TypeTag {
    let mut splitted = raw.split("::");
    let address = AccountAddress::from_hex_literal(splitted.next().unwrap()).unwrap();
    let module = Identifier::from(IdentStr::new(splitted.next().unwrap()).unwrap());
    let name = Identifier::from(IdentStr::new(splitted.next().unwrap()).unwrap());
    return TypeTag::Struct(Box::from(StructTag {
        address,
        module,
        name,
        type_params: vec![],
    }));
}

pub fn serialize_input_params(input_params: Vec<&str>, param_types: Vec<MoveType>) -> Vec<Vec<u8>> {
    assert_eq!(input_params.len(), param_types.len());
    let mut args: Vec<Vec<u8>> = Vec::new();
    let mut param_types_iter = param_types.into_iter();
    input_params.into_iter().for_each(|p| {
        if p.trim().len() > 0 {
            let current_param_type = param_types_iter.next().unwrap();
            match current_param_type {
                MoveType::Bool => {
                    args.push(
                        MoveValue::Bool(matches!(p.trim(), "true" | "t" | "1"))
                            .simple_serialize()
                            .unwrap(),
                    );
                }
                MoveType::U8 => {
                    let num = p.trim().parse::<u8>();
                    args.push(MoveValue::U8(num.unwrap()).simple_serialize().unwrap());
                }
                MoveType::U64 => {
                    let num = p.trim().parse::<u64>();
                    args.push(MoveValue::U64(num.unwrap()).simple_serialize().unwrap());
                }
                MoveType::U128 => {
                    let num = p.trim().parse::<u128>();
                    args.push(MoveValue::U128(num.unwrap()).simple_serialize().unwrap());
                }
                MoveType::Address => {
                    // Suppose it's an account parameter
                    args.push(
                        MoveValue::Address(AccountAddress::from_hex_literal(p.trim()).unwrap())
                            .simple_serialize()
                            .unwrap(),
                    );
                }
                MoveType::Signer => {
                    // Suppose it's an account parameter
                    args.push(
                        MoveValue::Signer(AccountAddress::from_hex_literal(p.trim()).unwrap())
                            .simple_serialize()
                            .unwrap(),
                    );
                }
                MoveType::Vector { items } => match items.as_ref() {
                    MoveType::Bool => {}
                    MoveType::U8 => args.push(
                        MoveValue::vector_u8(String::from(p.trim()).into_bytes())
                            .simple_serialize()
                            .unwrap(),
                    ),
                    MoveType::U64 => {}
                    MoveType::U128 => {}
                    MoveType::Address => {}
                    MoveType::Signer => {}
                    MoveType::Vector { .. } => {}
                    MoveType::Struct(_) => {}
                    MoveType::GenericTypeParam { .. } => {}
                    MoveType::Reference { .. } => {}
                    MoveType::Unparsable(_) => {}
                },
                MoveType::Struct(_tag) => {
                    panic!("Struct type is not supported yet")
                }
                MoveType::GenericTypeParam { .. } => {
                    panic!("Struct type is not supported yet")
                }
                MoveType::Reference { .. } => {
                    panic!("Struct type is not supported yet")
                }
                MoveType::Unparsable(_) => {
                    panic!("Unparsable paramter")
                }
            }
        }
    });
    return args;
}
