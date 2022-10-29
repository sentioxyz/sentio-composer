mod storage;

use anyhow::{bail, Result};

use aptos_sdk::move_types::account_address::AccountAddress as AptosAccountAddress;
use move_core_types::language_storage::{ModuleId, TypeTag};
use move_vm_runtime::move_vm::MoveVM;
use move_stdlib;
use move_vm_test_utils::gas_schedule::{CostTable, Gas, GasStatus};
use move_core_types::account_address::{AccountAddress, AccountAddressParseError};
use move_core_types::identifier::{Identifier, IdentStr};
use crate::storage::InMemoryLazyStorage;
use hex;
use move_core_types::value::MoveValue;

const STD_ADDR: AccountAddress = AccountAddress::ONE;

fn main() {
    println!("Hello, world!");
    exec_func();
}

pub fn get_gas_status(cost_table: &CostTable, gas_budget: Option<u64>) -> Result<GasStatus> {
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

fn exec_func() {
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
    let signer_account = AccountAddress::from_hex_literal("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap();
    let args: Vec<Vec<u8>> = vec![MoveValue::Signer(signer_account).simple_serialize().unwrap(), MoveValue::U64(195).simple_serialize().unwrap()];
    let account = AccountAddress::from_hex_literal("0xa99959ba3c86270f47e5379958ea4918a6ccc78659a0b37348163e19af54d549");
    match account {
        Ok(addr) => {
            let module = &ModuleId::new(addr, Identifier::new("xen").unwrap());
            let function = IdentStr::new("claim_rank").unwrap();
            let res = session.execute_function_bypass_visibility(module, function, type_args, args, &mut gas_status);
            println!("{}", res.unwrap().return_values.len())
        }
        Err(err) => {
            println!("{}", err)
        }
    }
}
