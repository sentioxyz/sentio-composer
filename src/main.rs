mod storage;
mod helper;
extern crate log;
extern crate simplelog;

use std::borrow::Borrow;
use std::fs;
use simplelog::*;

use std::fs::File;
use tempfile::tempdir;
use std::path::Path;
use crate::storage::InMemoryLazyStorage;
use std::sync::Arc;
use anyhow::{bail, Result};

use aptos_sdk::rest_client::aptos_api_types::IdentifierWrapper;
use move_core_types::language_storage::{ModuleId, TypeTag};
use move_vm_runtime::move_vm::MoveVM;
use move_stdlib;
use move_vm_test_utils::gas_schedule::{CostTable, Gas, GasStatus};
use move_core_types::account_address::{AccountAddress};
use move_core_types::identifier::{Identifier, IdentStr};
use move_core_types::value::{MoveTypeLayout, MoveValue};
use move_vm_types::values::Value;
use poem_openapi::{payload::PlainText, OpenApi};
use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use serde::{Deserialize, Serialize};
use poem_openapi::Object;
use clap::{arg, command};
use log::{error, info, LevelFilter};
use simplelog::WriteLogger;
use poem_openapi::__private::serde_json;
use crate::helper::absolute_path;

const STD_ADDR: AccountAddress = AccountAddress::ONE;

#[derive(Serialize, Deserialize, Debug)]
struct ExecutionResult {
    log_path: String,
    return_values: Vec<(Vec<u8>, MoveTypeLayout)>
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

pub fn exec_func(storage: InMemoryLazyStorage, module: ModuleId, function: &IdentStr, type_args: Vec<TypeTag>, args: Vec<Vec<u8>>) -> Option<Vec<(Vec<u8>, MoveTypeLayout)>> {
    let vm = MoveVM::new(move_stdlib::natives::all_natives(
        STD_ADDR,
        // TODO: come up with a suitable gas schedule
        move_stdlib::natives::GasParameters::zeros(),
    )).unwrap();
    let (mut session, mut gas_status) = {
        let gas_status = get_gas_status(
            &move_vm_test_utils::gas_schedule::INITIAL_COST_SCHEDULE,
            Some(1000000),
        ).unwrap();
        let session = vm.new_session(&storage);
        (session, gas_status)
    };
    let res = session.execute_function_bypass_visibility(&module, function, type_args, args, &mut gas_status);
    match res {
        Ok(success_result) => {
            info!("result length: {}", success_result.return_values.len());
            return Some(success_result.return_values);
        }
        Err(err) => {
            error!("Error! {}", err.to_string())
        }
    }
    return None
}

/// A request to submit a transaction
///
/// This requires a transaction and a signature of it
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct CallFunctionRequest {
    pub func: IdentifierWrapper,
    pub type_params: IdentifierWrapper,
    pub params: IdentifierWrapper,
}

// Context holds application scope context
#[derive(Clone)]
struct Context {
    // pub db: Arc<dyn DbReader>,
    pub storage: InMemoryLazyStorage,
}

struct Api {
    pub context: Arc<Context>
}

#[OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get")]
    async fn index(&self, name: Query<Option<String>>) -> PlainText<String> {
        match name.0 {
            Some(name) => PlainText(format!("hello, {}!", name)),
            None => PlainText("hello!".to_string()),
        }
    }

    #[oai(path = "/call_function", method = "post")]
    async fn call_function(&self, data: Json<CallFunctionRequest>) -> PlainText<String> {
        let type_args: Vec<TypeTag> = vec![];
        // let signer_account = AccountAddress::from_hex_literal("0x4f31605c22d20bab0488985bda5f310df7b9eca1432e062968b52c1f1a9a92c6").unwrap();
        let args: Vec<Vec<u8>> = vec![
            // MoveValue::Address(signer_account).simple_serialize().unwrap()
            // MoveValue::Signer(signer_account).simple_serialize().unwrap()
            // MoveValue::U64(110).simple_serialize().unwrap(),
            MoveValue::vector_u8("foo".as_bytes().to_vec()).simple_serialize().unwrap(),
            // MoveValue::U64(110).simple_serialize().unwrap(),
            // MoveValue::U64(0).simple_serialize().unwrap()
        ];
        let account = AccountAddress::from_hex_literal("0x54ad3d30af77b60d939ae356e6606de9a4da67583f02b962d2d3f2e481484e90");
        let module: ModuleId;
        match account {
            Ok(addr) => {
                module = ModuleId::new(addr, Identifier::new("packet").unwrap());
            }
            Err(err) => {
                return PlainText(err.to_string())
            }
        }
        exec_func(self.context.storage.clone(), module, IdentStr::new("hash_sha3_packet_bytes").unwrap(), type_args, args);
        PlainText(data.func.to_string())
    }
}

fn main() {
    let matches = command!() // requires `cargo` feature
        .arg(arg!(-f --func <FUNCTION> "Function name to call, e.g. 0x1::foo::bar").required(true))
        .arg(
            arg!(
                -t --type_params <TYPE_PARAMS> "Type parameters"
            ).required(false)
        )
        .arg(arg!(
            -p --params <PARAMS> "Parameters"
        ).required(false))
        .get_matches();

    let func;
    let type_params;
    let params;
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
    let log_path  = set_up_log();
    let mut execution_result = ExecutionResult {
        log_path,
        return_values: vec![]
    };
    example(func, type_params, params, &mut execution_result);
    println!("{}", serde_json::to_string(&execution_result).unwrap())
}

fn set_up_log() -> String {
    let dir = tempdir().unwrap().path().file_name().unwrap().to_owned();
    fs::create_dir_all(dir.clone()).unwrap();
    let file = Path::new(&dir).join("aptos_tool_bin.log");
    let file_path = absolute_path(file).unwrap().into_os_string().into_string().unwrap();
    WriteLogger::init(LevelFilter::Info, Config::default(), File::create(file_path.clone()).unwrap()).unwrap();
    file_path
}

fn example(func: String, type_params: String, params: String, execution_res: &mut ExecutionResult) {
    let type_args: Vec<TypeTag> = vec![];
    // let signer_account = AccountAddress::from_hex_literal("0x4f31605c22d20bab0488985bda5f310df7b9eca1432e062968b52c1f1a9a92c6").unwrap();
    let args: Vec<Vec<u8>> = vec![
        // MoveValue::Address(signer_account).simple_serialize().unwrap()
        // MoveValue::Signer(signer_account).simple_serialize().unwrap()
        // MoveValue::U64(110).simple_serialize().unwrap(),
        MoveValue::vector_u8("foo".as_bytes().to_vec()).simple_serialize().unwrap(),
        // MoveValue::U64(110).simple_serialize().unwrap(),
        // MoveValue::U64(0).simple_serialize().unwrap()
    ];
    // func: 0x54ad3d30af77b60d939ae356e6606de9a4da67583f02b962d2d3f2e481484e90::packet::hash_sha3_packet_bytes
    let mut splitted_func = func.split("::");
    let account = AccountAddress::from_hex_literal(splitted_func.next().unwrap());
    let module: ModuleId;
    match account {
        Ok(addr) => {
            module = ModuleId::new(addr, Identifier::new(splitted_func.next().unwrap()).unwrap());
            let storage = InMemoryLazyStorage::new();
            let res = exec_func(storage, module, IdentStr::new(splitted_func.next().unwrap()).unwrap(), type_args, args);
            match res {
                None => {
                    println!()
                }
                Some(vals) => {
                    execution_res.return_values = vals
                }
            }
        }
        Err(err) => {
            println!("error: {}", err);
        }
    }
}
