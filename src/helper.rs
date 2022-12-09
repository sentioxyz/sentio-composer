use crate::config::ToolConfig;
use crate::types::Network;
use aptos_sdk::rest_client::aptos_api_types::{MoveModule, MoveType};
use aptos_sdk::rest_client::{Client, MoveModuleBytecode};
use log::{debug, info, warn};
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::ModuleId;
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

pub fn get_node_url(network: &Network, config: &ToolConfig) -> Url {
    if let Some(url) = config.network_configs.get(network) {
        info!("Use client url: {}", url);
        return Url::from_str(url.as_str()).unwrap();
    }
    panic!("Cannot find the network URL")
}

pub fn serialize_input_params(
    raw_args: Option<Vec<String>>,
    param_types: Vec<MoveType>,
) -> Vec<Vec<u8>> {
    let mut args: Vec<Vec<u8>> = Vec::new();
    if let Some(input_params) = raw_args {
        assert_eq!(
            input_params.len(),
            param_types.len(),
            "The length of provided input params is not equal to expected one."
        );
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
    }
    return args;
}
