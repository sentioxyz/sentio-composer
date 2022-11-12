# Aptos tool to execute any on-chain function

## build
`cargo build`

## run
`./target/debug/view-function -h`
```shell
Usage: view-function [OPTIONS] --func <FUNCTION>

Options:
  -f, --func <FUNCTION>                  Function name to call, e.g. 0x1::foo::bar
  -t, --type_params <TYPE_PARAMS>        Type parameters
  -p, --params <PARAMS>                  Parameters
  -l, --ledger_version <LEDGER_VERSION>  Ledger version
  -n, --network <NETWORK>                network to use
  -h, --help                             Print help information
  -V, --version                          Print version information

```

## example
```shell
# command
view-function --func 0xeaa6ac31312d55907f6c9d7a66432d92d4da3aeef7ceb4e6242a8414ac67fa82::vault::account_collateral_and_debt --type_params 0x1::aptos_coin::AptosCoin --params 0xf485fdf431d489c7bd0b83efa2413a6701fe4985d3e64a299a1a2e9fb46bcb82 --ledger_version 0 --network testnet
# output
{
  "log_path": "/Users/poytr1/workspace/move_prototype/.tmpuWIdjA/aptos_tool_bin.log",
  "return_values": [
    "800000000u64",
    "1103000000u64"
  ]
}
```