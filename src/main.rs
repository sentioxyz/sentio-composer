mod config;
mod converter;
mod helper;
mod storage;
mod table;
mod types;

extern crate core;
extern crate log;

use std::borrow::Borrow;
use std::fs;

use simplelog::*;

use std::fs::File;
use std::path::Path;
use std::str::FromStr;

use anyhow::Result;
use aptos_gas::{AbstractValueSizeGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_sdk::rest_client::aptos_api_types::MoveType;
use aptos_sdk::rest_client::Client;

use clap::Parser;

use log::{debug, LevelFilter};
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::{IdentStr, Identifier};
use move_core_types::language_storage::{ModuleId, TypeTag, CORE_CODE_ADDRESS};
use move_core_types::value::{MoveStruct, MoveValue};
use move_vm_runtime::move_vm::MoveVM;
use move_vm_test_utils::gas_schedule::{CostTable, Gas, GasStatus};
use uuid::Uuid;

use aptos_vm::natives;
use move_table_extension::NativeTableContext;
use move_vm_runtime::native_extensions::NativeContextExtensions;
use move_vm_runtime::native_functions::NativeFunctionTable;

use crate::config::{ConfigData, ToolConfig};
use crate::converter::move_value_to_json;
use crate::helper::{absolute_path, get_module, get_node_url, serialize_input_params};
use crate::storage::InMemoryLazyStorage;
use crate::types::{ExecutionResult, LogLevel, Network, ViewFunction};

const STD_ADDR: AccountAddress = AccountAddress::ONE;

fn main() {
    let command = ViewFunction::parse();

    let func: String = command.function_id;
    let type_args: Option<Vec<String>> = command.type_args;
    let args: Option<Vec<String>> = command.args;
    let ledger_version: u64 = command.ledger_version;
    let network: Network = command.network;
    let config: Option<String> = command.config;
    let log_level: LogLevel = command.log_level;

    let mut tool_config = ToolConfig::default();
    if let Some(config_file) = config {
        tool_config = load_config(config_file.as_str());
        debug!("Use config file: {}", config_file);
    } else {
        tool_config = load_config("config.toml");
        debug!("Use default config file: config.toml");
    }

    let log_path = set_up_log(&tool_config, format!("{}", log_level));

    debug!("Value for func: {}", func);
    if let Some(val) = type_args.clone() {
        debug!("Value for type arguments: {:?}", val);
    }
    if let Some(val) = args.clone() {
        debug!("Value for arguments: {:?}", val);
    }
    debug!("Value for ledger version: {}", ledger_version);
    debug!("Value for network: {}", network);
    debug!("Value for log level: {}", log_level);

    let mut execution_result = ExecutionResult {
        log_path,
        return_values: vec![],
    };
    exec_func(
        func,
        type_args,
        args,
        ledger_version,
        &network,
        &tool_config,
        &mut execution_result,
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&execution_result).unwrap()
    )
}

fn load_config(file_path: &str) -> ToolConfig {
    if Path::new(file_path).exists() {
        return ConfigData::from_file(file_path).config;
    }
    ConfigData::default().config
}

fn set_up_log(config: &ToolConfig, log_level: String) -> String {
    if log_level.as_str().eq_ignore_ascii_case("off") {
        return String::new();
    }
    let dir = Path::new(config.log_folder.as_ref().unwrap().as_str());
    fs::create_dir_all(dir.clone()).unwrap();
    let id = Uuid::new_v4();
    let file = Path::new(&dir).join(format!("aptos_tool_bin_{}.log", id.to_string()));
    let file_path = absolute_path(file)
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();
    WriteLogger::init(
        LevelFilter::from_str(log_level.as_str()).unwrap(),
        Config::default(),
        File::create(file_path.clone()).unwrap(),
    )
    .unwrap();
    file_path
}

fn exec_func(
    func: String,
    type_args_input: Option<Vec<String>>,
    args_input: Option<Vec<String>>,
    ledger_version: u64,
    network: &Network,
    config: &ToolConfig,
    execution_res: &mut ExecutionResult,
) {
    let mut splitted_func = func.split("::");
    let account = AccountAddress::from_hex_literal(splitted_func.next().unwrap()).unwrap();
    let module = ModuleId::new(
        account,
        Identifier::new(splitted_func.next().unwrap()).unwrap(),
    );
    let func_id = IdentStr::new(splitted_func.next().unwrap()).unwrap();

    let client = Client::new(get_node_url(network, config));

    let cache_folder = config.cache_folder.clone().unwrap();
    let (_, abi) = get_module(
        client.clone(),
        &module,
        format!("{}", network),
        cache_folder.clone(),
    )
    .unwrap();

    let matched_func = abi
        .unwrap()
        .exposed_functions
        .into_iter()
        .find(|f| f.name.to_string() == func_id.to_string());

    let (param_types, ret_types) = if let Some(f) = matched_func {
        (f.params, f.return_)
    } else {
        panic!("No matched function found!");
    };

    let ser_args: Vec<Vec<u8>> = serialize_input_params(args_input, param_types);

    let type_args: Vec<TypeTag> = type_args_input
        .unwrap_or(vec![])
        .into_iter()
        .map(|tp| TypeTag::from_str(tp.as_str()).unwrap())
        .collect();

    let storage = InMemoryLazyStorage::new(
        ledger_version,
        format!("{}", network),
        client.clone(),
        cache_folder.clone(),
    );
    let res = exec_func_internal(storage, module, func_id, type_args, ser_args);
    match res {
        None => execution_res.return_values = vec![],
        Some(vals) => {
            let mut value_iter = vals.into_iter();
            let mut type_iter = ret_types.into_iter();
            let mut json_ret_vals = vec![];
            loop {
                let tpe = type_iter.next();
                if let Some(t) = tpe {
                    let mut val = value_iter.next().unwrap();
                    match t {
                        MoveType::Struct(struct_tag) => {
                            let module = ModuleId::new(
                                AccountAddress::from_bytes(struct_tag.address.inner().into_bytes())
                                    .unwrap(),
                                Identifier::from_str(struct_tag.module.as_str()).unwrap(),
                            );
                            let (_, abi) = get_module(
                                client.clone(),
                                &module,
                                format!("{}", network),
                                cache_folder.clone(),
                            )
                            .unwrap();

                            let fields_found = if let Some(ms) = abi
                                .unwrap()
                                .structs
                                .into_iter()
                                .find(|s| s.name.to_string() == struct_tag.name.to_string())
                            {
                                Some(ms.fields)
                            } else {
                                None
                            };

                            val = match val {
                                MoveValue::Struct(MoveStruct::Runtime(struct_vals)) => {
                                    MoveValue::Struct(MoveStruct::WithFields(
                                        struct_vals
                                            .into_iter()
                                            .map(|v| (Identifier::from_str("dummy").unwrap(), v))
                                            .collect(),
                                    ))
                                }
                                _ => val,
                            }
                        }
                        MoveType::Vector { items } => match items.borrow() {
                            MoveType::Struct(struct_tag) => {
                                let module = ModuleId::new(
                                    AccountAddress::from_bytes(
                                        struct_tag.address.inner().into_bytes(),
                                    )
                                    .unwrap(),
                                    Identifier::from_str(struct_tag.module.as_str()).unwrap(),
                                );
                                let (_, abi) = get_module(
                                    client.clone(),
                                    &module,
                                    format!("{}", network),
                                    cache_folder.clone(),
                                )
                                .unwrap();

                                let fields_found = if let Some(ms) = abi
                                    .unwrap()
                                    .structs
                                    .into_iter()
                                    .find(|s| s.name.to_string() == struct_tag.name.to_string())
                                {
                                    Some(ms.fields)
                                } else {
                                    None
                                };

                                val = match val {
                                    MoveValue::Struct(MoveStruct::Runtime(struct_vals)) => {
                                        MoveValue::Struct(MoveStruct::WithFields(
                                            struct_vals
                                                .into_iter()
                                                .map(|v| {
                                                    (Identifier::from_str("dummy").unwrap(), v)
                                                })
                                                .collect(),
                                        ))
                                    }
                                    MoveValue::Vector(inner_vals) => MoveValue::Vector(
                                        inner_vals
                                            .into_iter()
                                            .map(|v| match v {
                                                MoveValue::Struct(MoveStruct::Runtime(
                                                    struct_vals,
                                                )) => MoveValue::Struct(MoveStruct::WithFields(
                                                    struct_vals
                                                        .into_iter()
                                                        .map(|v| {
                                                            (
                                                                Identifier::from_str("dummy")
                                                                    .unwrap(),
                                                                v,
                                                            )
                                                        })
                                                        .collect(),
                                                )),
                                                _ => panic!(""),
                                            })
                                            .collect(),
                                    ),
                                    _ => val,
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                    json_ret_vals.push(move_value_to_json(val));
                } else {
                    break;
                }
            }
            execution_res.return_values = json_ret_vals;
        }
    }
}

fn exec_func_internal(
    storage: InMemoryLazyStorage,
    module: ModuleId,
    function: &IdentStr,
    type_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
) -> Option<Vec<MoveValue>> {
    let natives = natives::aptos_natives(
        NativeGasParameters::zeros(),
        AbstractValueSizeGasParameters::zeros(),
        LATEST_GAS_FEATURE_VERSION,
    );

    let vm = MoveVM::new(natives).unwrap();

    let mut extensions = NativeContextExtensions::default();
    extensions.add(NativeTableContext::new([0u8; 32], &storage));
    let (mut session, mut gas_status) = {
        let gas_status = get_gas_status(
            &move_vm_test_utils::gas_schedule::INITIAL_COST_SCHEDULE,
            Some(1000000),
        )
        .unwrap();
        let session = vm.new_session_with_extensions(&storage, extensions);
        (session, gas_status)
    };
    let res = session.execute_function_bypass_visibility(
        &module,
        function,
        type_args,
        args,
        &mut gas_status,
    );
    match res {
        Ok(success_result) => {
            let move_values: Vec<MoveValue> = success_result
                .return_values
                .clone()
                .into_iter()
                .map(|v| {
                    let deserialized_value = MoveValue::simple_deserialize(&*v.0, &v.1).unwrap();
                    deserialized_value
                })
                .collect();
            return Some(move_values);
        }
        Err(err) => {
            panic!("Error while executing the function! {}", err.to_string())
        }
    }
}

fn get_gas_status(cost_table: &CostTable, gas_budget: Option<u64>) -> Result<GasStatus> {
    let gas_status = if let Some(gas_budget) = gas_budget {
        // TODO(Gas): This should not be hardcoded.
        let max_gas_budget = u64::MAX.checked_div(1000).unwrap();
        if gas_budget >= max_gas_budget {
            panic!("Gas budget set too high; maximum is {}", max_gas_budget)
        }
        GasStatus::new(cost_table, Gas::new(gas_budget))
    } else {
        // no budget specified. Disable gas metering
        GasStatus::new_unmetered()
    };
    Ok(gas_status)
}

#[cfg(test)]
mod tests {
    use crate::converter::move_value_to_json;
    use crate::{
        exec_func, exec_func_internal, get_node_url, ConfigData, ExecutionResult,
        InMemoryLazyStorage, Network, ToolConfig,
    };
    use aptos_sdk::rest_client::Client;
    use log::{debug, LevelFilter};
    use move_core_types::account_address::AccountAddress;
    use move_core_types::identifier::{IdentStr, Identifier};
    use move_core_types::language_storage::{ModuleId, TypeTag};
    use move_core_types::value::MoveValue;
    use once_cell::sync::Lazy;
    use serde_json::Value;
    use simplelog::{Config, SimpleLogger};

    static CONFIG: Lazy<ToolConfig> = Lazy::new(|| ConfigData::default().config);

    #[cfg(test)]
    #[ctor::ctor]
    fn init() {
        SimpleLogger::init(LevelFilter::Debug, Config::default()).unwrap();
    }

    #[test]
    fn test_call_aptos_function() {
        let client = Client::new(get_node_url(&Network::Mainnet, &CONFIG));
        let storage = InMemoryLazyStorage::new(
            0,
            format!("{}", Network::Mainnet),
            client,
            String::from("."),
        );
        let addr = AccountAddress::from_hex_literal(
            "0x54ad3d30af77b60d939ae356e6606de9a4da67583f02b962d2d3f2e481484e90",
        )
        .unwrap();
        let module = ModuleId::new(addr, Identifier::new("packet").unwrap());
        let func = IdentStr::new("hash_sha3_packet_bytes").unwrap();
        let type_args: Vec<TypeTag> = vec![];
        let mut args: Vec<Vec<u8>> = Vec::new();
        args.push(
            MoveValue::vector_u8("bar".as_bytes().to_vec())
                .simple_serialize()
                .unwrap(),
        );
        let res = exec_func_internal(storage, module, func, type_args, args);
        match res {
            None => {}
            Some(val) => {
                assert_eq!(val.len(), 1);
                debug!("[{}]", val[0]);
            }
        }
    }

    #[test]
    fn test_call_aptos_function_vault_e2e() {
        let mut execution_result = ExecutionResult {
            log_path: String::new(),
            return_values: vec![],
        };
        exec_func(
            String::from("0xeaa6ac31312d55907f6c9d7a66432d92d4da3aeef7ceb4e6242a8414ac67fa82::vault::account_collateral_and_debt"),
            Some(vec![String::from("0x1::aptos_coin::AptosCoin")]),
            Some(vec![String::from("0xf485fdf431d489c7bd0b83efa2413a6701fe4985d3e64a299a1a2e9fb46bcb82")]),
        0,
            &Network::Testnet,
            &CONFIG,
            &mut execution_result);
        assert_eq!(execution_result.return_values.len(), 2);
        debug!("{}", execution_result.return_values[0]);
        debug!("{}", execution_result.return_values[1]);
    }

    #[test]
    fn test_get_current_block_height() {
        let mut execution_result = ExecutionResult {
            log_path: String::new(),
            return_values: vec![],
        };
        exec_func(
            String::from("0x1::block::get_current_block_height"),
            None,
            None,
            0,
            &Network::Mainnet,
            &CONFIG,
            &mut execution_result,
        );
        assert_eq!(execution_result.return_values.len(), 1);
        debug!("{}", execution_result.return_values[0]);
    }

    #[test]
    fn test_aptos_native_function() {
        let mut execution_result = ExecutionResult {
            log_path: String::new(),
            return_values: vec![],
        };
        exec_func(
            String::from("0x193fbac5485237942de26fe360764e812b71a6b4f5ce8f374d41e3f55dcf01df::order::get_user_orders_history"),
            None,
            Some(vec![String::from("0x193fbac5485237942de26fe360764e812b71a6b4f5ce8f374d41e3f55dcf01df")]),
            0,
            &Network::Testnet,
            &CONFIG,
            &mut execution_result,
        );
        assert_eq!(execution_result.return_values.len(), 1);
        debug!("{}", execution_result.return_values[0]);
    }

    #[test]
    fn test_account_deposit() {
        let mut execution_result = ExecutionResult {
            log_path: String::new(),
            return_values: vec![],
        };
        exec_func(
            String::from("0xa46f37ead5670b6862709a0f17f7464a767877cba7c3c18196bc8e1e0f3c3a89::stability_pool::account_deposit"),
            None,
            Some(vec![String::from("0xa0fc6038965061835c42e8b8b0528841d492d3fb8f6d9e2105c613652ba9f5ce")]),
            375991494,
            &Network::Testnet,
            &CONFIG,
            &mut execution_result,
        );
        assert_eq!(execution_result.return_values.len(), 1);
        assert_eq!(
            execution_result.return_values[0],
            serde_json::to_value(0).unwrap()
        );
        debug!("{}", execution_result.return_values[0]);
    }
}
