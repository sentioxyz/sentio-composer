mod storage;
use crate::storage::InMemoryLazyStorage;
use std::io::Read;
use anyhow::{bail, Result};

use aptos_sdk::move_types::account_address::AccountAddress as AptosAccountAddress;
use aptos_sdk::rest_client::aptos_api_types::IdentifierWrapper;
use move_core_types::language_storage::{ModuleId, TypeTag};
use move_vm_runtime::move_vm::MoveVM;
use move_stdlib;
use move_vm_test_utils::gas_schedule::{CostTable, Gas, GasStatus};
use move_core_types::account_address::{AccountAddress, AccountAddressParseError};
use move_core_types::identifier::{Identifier, IdentStr};
use hex;
use move_binary_format::errors::VMResult;
use move_core_types::value::MoveValue;
use move_core_types::resolver::MoveResolver;
use move_vm_runtime::session::Session;
use move_vm_types::values::Value;
use poem::{listener::TcpListener, Route, Server};
use poem_openapi::{payload::PlainText, OpenApi, OpenApiService, ApiRequest};
use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use serde::{Deserialize, Serialize};
use poem_openapi::Object;
const STD_ADDR: AccountAddress = AccountAddress::ONE;

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

pub fn exec_func() {
    let vm = MoveVM::new(move_stdlib::natives::all_natives(
        STD_ADDR,
        // TODO: come up with a suitable gas schedule
        move_stdlib::natives::GasParameters::zeros(),
    )).unwrap();
    let storage = InMemoryLazyStorage::new();
    let (mut session, mut gas_status) = {
        let gas_status = get_gas_status(
            &move_vm_test_utils::gas_schedule::INITIAL_COST_SCHEDULE,
            Some(1000000),
        ).unwrap();
        let session = vm.new_session(&storage);
        (session, gas_status)
    };
    let type_args: Vec<TypeTag> = vec![];
    let signer_account = AccountAddress::from_hex_literal("0x4f31605c22d20bab0488985bda5f310df7b9eca1432e062968b52c1f1a9a92c6").unwrap();
    let args: Vec<Vec<u8>> = vec![
        // MoveValue::Address(signer_account).simple_serialize().unwrap()
        // MoveValue::Signer(signer_account).simple_serialize().unwrap()
        // MoveValue::U64(110).simple_serialize().unwrap(),
        MoveValue::vector_u8("foo".as_bytes().to_vec()).simple_serialize().unwrap(),
        // MoveValue::U64(110).simple_serialize().unwrap(),
        // MoveValue::U64(0).simple_serialize().unwrap()
    ];
    let account = AccountAddress::from_hex_literal("0x54ad3d30af77b60d939ae356e6606de9a4da67583f02b962d2d3f2e481484e90");
    match account {
        Ok(addr) => {
            let module = &ModuleId::new(addr, Identifier::new("packet").unwrap());
            let function = IdentStr::new("hash_sha3_packet_bytes").unwrap();
            let res = session.execute_function_bypass_visibility(module, function, type_args, args, &mut gas_status);
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
        Err(err) => {
            println!("{}", err)
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

struct Api;

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
        PlainText(data.func.to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let api_service =
        OpenApiService::new(Api, "Hello World", "1.0").server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(Route::new().nest("/api", api_service).nest("/", ui))
        .await
}