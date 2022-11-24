mod config;
mod helper;
mod storage;
mod table;

extern crate core;
extern crate log;

use std::fs;

use simplelog::*;

use std::fs::File;
use std::path::Path;
use std::str::FromStr;

use anyhow::Result;
use aptos_gas::{AbstractValueSizeGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};

use aptos_sdk::rest_client::Client;
use clap::{arg, command};
use log::{debug, error, LevelFilter};
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::{IdentStr, Identifier};
use move_core_types::language_storage::{CORE_CODE_ADDRESS, ModuleId, TypeTag};
use move_core_types::value::MoveValue;
use move_stdlib;
use move_vm_runtime::move_vm::MoveVM;
use move_vm_test_utils::gas_schedule::{CostTable, Gas, GasStatus};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use aptos_vm::natives;
use move_vm_runtime::native_extensions::NativeContextExtensions;
use move_vm_runtime::native_functions::NativeFunctionTable;
use toml::value::Table;

use crate::config::{ConfigData, ToolConfig};
use crate::helper::{
    absolute_path, construct_struct_type_tag_from_str, get_function_module, get_node_url,
    serialize_input_params,
};
use crate::storage::InMemoryLazyStorage;
use crate::table::NativeTableContext;

const STD_ADDR: AccountAddress = AccountAddress::ONE;

#[derive(Serialize, Deserialize, Debug)]
struct ExecutionResult {
    log_path: String,
    return_values: Vec<String>,
}

fn main() {
    let matches =
        command!() // requires `cargo` feature
            .arg(
                arg!(-F --func <FUNCTION> "Function name to call, e.g. 0x1::foo::bar.")
                    .required(true),
            )
            .arg(
                arg!(
                    -T --type_args <TYPE_ARGS> "Type parameters, seperated by ',' e.g. 0x1::aptos_coin::AptosCoin."
                )
                .required(false),
            )
            .arg(
                arg!(
                    -A --args <ARGS> "Parameters, seperated by ',' e.g. foo, bar."
                )
                .required(false),
            )
            .arg(
                arg!(
                    -L --ledger_version <LEDGER_VERSION> "Ledger version, if not apply or 0, use the latest ledger version."
                )
                .required(false)
                    .default_value("0")
                .value_parser(clap::value_parser!(u64)),
            )
            .arg(
                arg!(
                    -N --network <NETWORK> "Network to use, e.g. mainnet."
                )
                .default_value("mainnet")
                .required(false),
            )
            .arg(
                arg!(
                    -C --config <CONFIG_FILE> "Config file to use."
                ).default_value("config.toml").required(false)
            )
            .arg(
                arg!(
                    --log_level <LOG_LEVEL> "log level, one of 'Off', 'Error', 'Warn', 'Info', 'Debug', 'Trace'."
                ).default_value("Off")
            )
            .get_matches();

    let func = matches.get_one::<String>("func").unwrap().clone();
    let type_args = matches.get_one::<String>("type_args");
    let args = matches.get_one::<String>("args");
    let ledger_version: u64 = matches.get_one::<u64>("ledger_version").unwrap().clone();
    let network: String = matches.get_one::<String>("network").unwrap().clone();
    let config_file: String = matches.get_one::<String>("config").unwrap().clone();
    let log_level: String = matches.get_one::<String>("log_level").unwrap().clone();

    let config = load_config(config_file.as_str());
    let log_path = set_up_log(&config, log_level.clone());

    debug!("Value for func: {}", func);
    if let Some(val) = type_args {
        debug!("Value for type arguments: {}", val);
    }
    if let Some(val) = args {
        debug!("Value for arguments: {}", val);
    }
    debug!("Value for ledger version: {}", ledger_version);
    debug!("Value for network: {}", network);
    debug!("Value for config file: {}", config_file);
    debug!("Value for log_level: {}", log_level);

    let mut execution_result = ExecutionResult {
        log_path,
        return_values: vec![],
    };
    exec_func(
        func,
        type_args,
        args,
        ledger_version,
        network,
        &config,
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
    type_args_input: Option<&String>,
    args_input: Option<&String>,
    ledger_version: u64,
    network: String,
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

    let client = Client::new(get_node_url(network.clone(), config));

    let cache_folder = config.cache_folder.clone().unwrap();
    let (_, abi) = get_function_module(
        client.clone(),
        &module,
        network.clone(),
        cache_folder.clone(),
    )
    .unwrap();

    let matched_func = abi
        .unwrap()
        .exposed_functions
        .into_iter()
        .find(|f| f.name.to_string() == func_id.to_string());

    let param_types = if let Some(f) = matched_func {
        f.params
    } else {
        panic!("No matched function found!");
    };

    let ser_args: Vec<Vec<u8>> = serialize_input_params(args_input, param_types);

    // For now, we only support struct type arg
    let mut type_args: Vec<TypeTag> = vec![];
    if let Some(tp_args_val) = type_args_input {
        let splitted_type_args = tp_args_val.split(",");
        splitted_type_args.into_iter().for_each(|tp| {
            if tp.trim().len() > 0 {
                if tp.contains("::") {
                    type_args.push(construct_struct_type_tag_from_str(tp));
                } else {
                    panic!("only support struct type parameters now!");
                }
            }
        });
    }

    let storage = InMemoryLazyStorage::new(ledger_version, network, client.clone(), cache_folder);
    let res = exec_func_internal(storage, module, func_id, type_args, ser_args);
    match res {
        None => execution_res.return_values = vec![],
        Some(vals) => execution_res.return_values = vals,
    }
}

fn exec_func_internal(
    storage: InMemoryLazyStorage,
    module: ModuleId,
    function: &IdentStr,
    type_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
) -> Option<Vec<String>> {
    let natives = natives::aptos_natives(
        NativeGasParameters::zeros(),
        AbstractValueSizeGasParameters::zeros(),
        LATEST_GAS_FEATURE_VERSION,
    );
    let extended_natives: NativeFunctionTable = natives.into_iter()
        .filter(|(_, name, _, _)| name.as_str() != "table")
        .chain(table::table_natives(
        CORE_CODE_ADDRESS,
        table::GasParameters::zeros(),
    )).collect();

    let vm = MoveVM::new(extended_natives).unwrap();

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
            let pretty_print_values: Vec<String> = success_result
                .return_values
                .clone()
                .into_iter()
                .map(|v| {
                    let deserialized_value = MoveValue::simple_deserialize(&*v.0, &v.1).unwrap();
                    format!("{}", deserialized_value)
                })
                .collect();
            return Some(pretty_print_values);
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
    use crate::{
        exec_func, exec_func_internal, get_node_url, ConfigData, ExecutionResult,
        InMemoryLazyStorage, ToolConfig,
    };
    use aptos_sdk::rest_client::Client;
    use log::{debug, LevelFilter};
    use move_core_types::account_address::AccountAddress;
    use move_core_types::identifier::{IdentStr, Identifier};
    use move_core_types::language_storage::{ModuleId, TypeTag};
    use move_core_types::value::MoveValue;
    use once_cell::sync::Lazy;
    use simplelog::{Config, SimpleLogger};

    static MAINNET: Lazy<String> = Lazy::new(|| String::from("mainnet"));

    static TESTNET: Lazy<String> = Lazy::new(|| String::from("testnet"));

    static CONFIG: Lazy<ToolConfig> = Lazy::new(|| ConfigData::default().config);

    #[cfg(test)]
    #[ctor::ctor]
    fn init() {
        SimpleLogger::init(LevelFilter::Info, Config::default()).unwrap();
    }

    #[test]
    fn test_call_aptos_function() {
        let client = Client::new(get_node_url(MAINNET.to_owned(), &CONFIG));
        let storage = InMemoryLazyStorage::new(0, MAINNET.to_owned(), client, String::from("."));
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
            Some(&String::from("0x1::aptos_coin::AptosCoin")),
            Some(&String::from("0xf485fdf431d489c7bd0b83efa2413a6701fe4985d3e64a299a1a2e9fb46bcb82")),
        0,
            String::from("testnet"),
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
            String::from("mainnet"),
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
            String::from("0xa46f37ead5670b6862709a0f17f7464a767877cba7c3c18196bc8e1e0f3c3a89::stability_pool::account_deposit"),
            None,
            Some(&String::from("0xf485fdf431d489c7bd0b83efa2413a6701fe4985d3e64a299a1a2e9fb46bcb82")),
            0,
            String::from("testnet"),
            &CONFIG,
            &mut execution_result,
        );
        assert_eq!(execution_result.return_values.len(), 1);
        debug!("{}", execution_result.return_values[0]);
    }
}
