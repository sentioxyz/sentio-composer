mod helper;
mod storage;
mod config;
extern crate core;
extern crate log;

use std::fs;

use simplelog::*;

use std::fs::File;
use std::path::Path;

use anyhow::{bail, Result};

use aptos_sdk::rest_client::Client;
use clap::{arg, command};
use log::{error, info, LevelFilter};
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::{IdentStr, Identifier};
use move_core_types::language_storage::{ModuleId, TypeTag};
use move_core_types::value::{MoveValue};
use move_stdlib;
use move_vm_runtime::move_vm::MoveVM;
use move_vm_test_utils::gas_schedule::{CostTable, Gas, GasStatus};
use serde::{Deserialize, Serialize};

use crate::helper::{
    absolute_path, construct_struct_type_tag_from_str, get_function_module, get_node_url,
    serialize_input_params,
};
use crate::storage::InMemoryLazyStorage;
use crate::config::{ConfigData, ToolConfig};

const STD_ADDR: AccountAddress = AccountAddress::ONE;

#[derive(Serialize, Deserialize, Debug)]
struct ExecutionResult {
    log_path: String,
    return_values: Vec<String>,
}

fn main() {
    let config = load_config();
    let log_path = set_up_log(&config);
    let matches =
        command!() // requires `cargo` feature
            .arg(
                arg!(-f --func <FUNCTION> "Function name to call, e.g. 0x1::foo::bar")
                    .required(true),
            )
            .arg(
                arg!(
                    -t --type_params <TYPE_PARAMS> "Type parameters"
                )
                .required(false),
            )
            .arg(
                arg!(
                    -p --params <PARAMS> "Parameters"
                )
                .required(false),
            )
            .arg(
                arg!(
                    -l --ledger_version <LEDGER_VERSION> "Ledger version"
                )
                .required(false)
                .value_parser(clap::value_parser!(u64)),
            )
            .arg(
                arg!(
                    -n --network <NETWORK> "network to use"
                )
                .required(false),
            )
            .get_matches();

    let func;
    let type_params;
    let params;
    let ledger_version: u64;
    let network: String;
    if let Some(matched_func) = matches.get_one::<String>("func") {
        info!("Value for func: {}", matched_func);
        // TODO(pc): check if the function name is legal
        func = matched_func.clone();
    } else {
        return;
    }
    if let Some(matched_tp) = matches.get_one::<String>("type_params") {
        info!("Value for type parameters: {}", matched_tp);
        type_params = matched_tp.clone();
    } else {
        type_params = "".parse().unwrap();
    }
    if let Some(matched_params) = matches.get_one::<String>("params") {
        info!("Value for params: {}", matched_params);
        params = matched_params.clone();
    } else {
        params = "".parse().unwrap();
    }
    if let Some(matched_ledger_version) = matches.get_one::<u64>("ledger_version") {
        info!("Value for ledger version: {}", matched_ledger_version);
        ledger_version = matched_ledger_version.clone();
    } else {
        ledger_version = 0;
    }
    if let Some(matched_network) = matches.get_one::<String>("network") {
        info!("Value for network: {}", matched_network);
        network = matched_network.clone();
    } else {
        network = "mainnet".parse().unwrap();
    }
    let mut execution_result = ExecutionResult {
        log_path,
        return_values: vec![],
    };
    exec_func(
        func,
        type_params,
        params,
        ledger_version,
        network,
        &config,
        &mut execution_result,
    );
    println!("{}", serde_json::to_string_pretty(&execution_result).unwrap())
}

fn load_config() -> ToolConfig {
    let file_path = "config.toml";
    if Path::new(file_path).exists() {
        return ConfigData::from_file(file_path).config;
    }
    ConfigData::default().config
}

fn set_up_log(config: &ToolConfig) -> String {
    let dir = Path::new(config.log_folder.as_ref().unwrap().as_str());
    fs::create_dir_all(dir.clone()).unwrap();
    let file = Path::new(&dir).join("aptos_tool_bin.log");
    let file_path = absolute_path(file)
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();
    WriteLogger::init(
        LevelFilter::Info,
        Config::default(),
        File::create(file_path.clone()).unwrap(),
    )
    .unwrap();
    file_path
}

fn exec_func(
    func: String,
    type_params: String,
    params: String,
    ledger_version: u64,
    network: String,
    config: &ToolConfig,
    execution_res: &mut ExecutionResult,
) {
    // func: 0x54ad3d30af77b60d939ae356e6606de9a4da67583f02b962d2d3f2e481484e90::packet::hash_sha3_packet_bytes
    let mut splitted_func = func.split("::");
    let account = AccountAddress::from_hex_literal(splitted_func.next().unwrap()).unwrap();
    let module = ModuleId::new(
        account,
        Identifier::new(splitted_func.next().unwrap()).unwrap(),
    );
    let func_id = IdentStr::new(splitted_func.next().unwrap()).unwrap();

    let client = Client::new(get_node_url(network.clone(), config));
    // TODO(pcxu): serialize params according to abi
    let (_, abi) = get_function_module(client.clone(), &module, network.clone()).unwrap();

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

    let splitted_params: Vec<&str> = params.split(",").collect();
    let args: Vec<Vec<u8>> = serialize_input_params(splitted_params, param_types);

    // For now, we only support struct type arg
    let splitted_type_params = type_params.split(",");
    let mut type_args: Vec<TypeTag> = vec![];
    splitted_type_params.into_iter().for_each(|tp| {
        if tp.trim().len() > 0 {
            if tp.contains("::") {
                type_args.push(construct_struct_type_tag_from_str(tp));
            } else {
                panic!("only support struct type parameters now!");
            }
        }
    });

    let storage = InMemoryLazyStorage::new(ledger_version, network, client.clone());
    let res = exec_func_internal(storage, module, func_id, type_args, args);
    match res {
        None => execution_res.return_values = vec![],
        Some(vals) => {
            // let deser_vals = vals.into_iter().map(|val| {
            //     MoveValue::simple_deserialize(&*val.0, &val.1).unwrap()
            // }).collect();
            execution_res.return_values = vals
        }
    }
}

fn exec_func_internal(
    storage: InMemoryLazyStorage,
    module: ModuleId,
    function: &IdentStr,
    type_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
) -> Option<Vec<String>> {
    let vm = MoveVM::new(move_stdlib::natives::all_natives(
        STD_ADDR,
        // TODO: come up with a suitable gas schedule
        move_stdlib::natives::GasParameters::zeros(),
    ))
    .unwrap();
    let (mut session, mut gas_status) = {
        let gas_status = get_gas_status(
            &move_vm_test_utils::gas_schedule::INITIAL_COST_SCHEDULE,
            Some(1000000),
        )
        .unwrap();
        let session = vm.new_session(&storage);
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
            info!("result length: {}", success_result.return_values.len());
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
            error!("Error! {}", err.to_string())
        }
    }
    return None;
}

fn get_gas_status(cost_table: &CostTable, gas_budget: Option<u64>) -> Result<GasStatus> {
    let gas_status = if let Some(gas_budget) = gas_budget {
        // TODO(Gas): This should not be hardcoded.
        let max_gas_budget = u64::MAX.checked_div(1000).unwrap();
        if gas_budget >= max_gas_budget {
            bail!("Gas budget set too high; maximum is {}", max_gas_budget)
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
    use crate::{exec_func, exec_func_internal, get_node_url, ExecutionResult, InMemoryLazyStorage, ConfigData, ToolConfig};
    use aptos_sdk::rest_client::Client;
    use log::{info, LevelFilter};
    use move_core_types::account_address::AccountAddress;
    use move_core_types::identifier::{IdentStr, Identifier};
    use move_core_types::language_storage::{ModuleId, TypeTag};
    use move_core_types::value::{MoveValue};
    use once_cell::sync::Lazy;
    use simplelog::{Config, SimpleLogger};

    static MAINNET: Lazy<String> = Lazy::new(|| String::from("mainnet"));

    static TESTNET: Lazy<String> = Lazy::new(|| String::from("testnet"));

    static CONFIG: Lazy<ToolConfig> = Lazy::new(|| ConfigData::default().config);
    #[test]
    fn test_call_aptos_function() {
        SimpleLogger::init(LevelFilter::Info, Config::default()).unwrap();
        let client = Client::new(get_node_url(MAINNET.to_owned(), &CONFIG));
        let storage = InMemoryLazyStorage::new(0, MAINNET.to_owned(), client);
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
                info!("[{}]", val[0]);
            }
        }
    }

    #[test]
    fn test_call_aptos_function_vault_e2e() {
        SimpleLogger::init(LevelFilter::Info, Config::default()).unwrap();
        let mut execution_result = ExecutionResult {
            log_path: String::new(),
            return_values: vec![],
        };
        exec_func(
            String::from("0xeaa6ac31312d55907f6c9d7a66432d92d4da3aeef7ceb4e6242a8414ac67fa82::vault::account_collateral_and_debt"),
            String::from("0x1::aptos_coin::AptosCoin"),
            String::from("0xf485fdf431d489c7bd0b83efa2413a6701fe4985d3e64a299a1a2e9fb46bcb82"),
        0,
            String::from("testnet"),
            &CONFIG,
            &mut execution_result);
        assert_eq!(execution_result.return_values.len(), 2);
        info!("{}", execution_result.return_values[0]);
        info!("{}", execution_result.return_values[1]);
    }
}
