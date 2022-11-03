mod storage;
use crate::storage::{InMemoryLazyStorage, NODE_URL};
use std::sync::Arc;
use anyhow::{bail, Result};

use aptos_sdk::rest_client::aptos_api_types::IdentifierWrapper;
use aptos_sdk::rest_client::Client;
use move_core_types::language_storage::{ModuleId, TypeTag};
use move_vm_runtime::move_vm::MoveVM;
use move_stdlib;
use move_vm_test_utils::gas_schedule::{CostTable, Gas, GasStatus};
use move_core_types::account_address::{AccountAddress};
use move_core_types::identifier::{Identifier, IdentStr};
use move_core_types::value::MoveValue;
use move_vm_types::values::Value;
use poem::{listener::TcpListener, Route, Server};
use poem_openapi::{payload::PlainText, OpenApi, OpenApiService};
use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use serde::{Deserialize, Serialize};
use poem_openapi::Object;
use clap::Parser;

const STD_ADDR: AccountAddress = AccountAddress::ONE;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    func: String,
    #[arg(short, long)]
    type_params: String,
    #[arg(short, long)]
    params: String,
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

pub fn exec_func(storage: InMemoryLazyStorage, module: ModuleId, function: &IdentStr, type_args: Vec<TypeTag>, args: Vec<Vec<u8>>) {
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
            println!("length: {}", success_result.return_values.len());
            let val = success_result.return_values.get(0).unwrap().clone();
            let deser_val = Value::simple_deserialize(&val.0.to_vec(), &val.1);
            println!("deserialized val {}", deser_val.unwrap().to_string());
        }
        Err(err) => {
            println!("Error! {}", err.to_string())
        }
    }
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
    let args = Args::parse();
    let func: String = args.func;
    let type_params = args.type_params;
    let params = args.params;
    example(func, type_params, params)
}

fn example(func: String, type_params: String, params: String) {
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
            exec_func(storage, module, IdentStr::new(splitted_func.next().unwrap()).unwrap(), type_args, args);
        }
        Err(err) => {
            println!("error: {}", err);
        }
    }
}

// #[tokio::main]
// async fn main() -> Result<(), std::io::Error> {
//     if std::env::var_os("RUST_LOG").is_none() {
//         std::env::set_var("RUST_LOG", "poem=debug");
//     }
//     tracing_subscriber::fmt::init();
//     let api = Api {
//         context: Arc::new(Context {
//             storage: InMemoryLazyStorage::new()
//         })
//     };
//     let api_service =
//         OpenApiService::new(api, "Hello World", "1.0").server("http://localhost:3000/api");
//     let ui = api_service.swagger_ui();
//
//     Server::new(TcpListener::bind("127.0.0.1:3000"))
//         .run(Route::new().nest("/api", api_service).nest("/", ui))
//         .await
// }