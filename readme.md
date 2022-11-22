# <img src="./static/logo.png" alt="drawing" width="40"/> Sentio Composer - Execute Any Functions on Aptos Blockchain
This project can be used to call view functions on the aptos blockchain. Given the input of the function and the corresponding ledger version, tool can execute that function and return the corresponding result.

It doesn't require the view functions as entry functions, and now supports the networks including mainnet, testnet and devnet.

This project includes a CLI tool, and we've also built a demo web app on top of it.
You can choose to use the CLI tool or directly start the web stack locally via docker.

## Visit demo website at http://composer.sentio.xyz/
We host a demo website for preview use, have a try :sparkles: !

## Start the Web APP locally
```shell
sudo docker run --env BIN_PATH="/app/view-function" -d -p 8080:4000 poytr1/sentio-composer-app:latest
```
Now open `http://localhost:8080/`, APP is up.

![app_screenshot](./static/img.png)
## Install the CLI tool
You can choose to build the CLI tool on your own or download the binary directly from [release](https://github.com/sentioxyz/sentio-composer/releases) page.
### To build it on your own:
#### Install the Rust and Cargo
`curl https://sh.rustup.rs -sSf | sh`
#### To install the CLI tool globally
`cargo install --git https://github.com/sentioxyz/sentio-composer`
### To build debug or release binary locally
1. Clone the project, and run `cargo build` in the root of the project.
2. Release build, run `cargo build --release`.
3. Check the target folder, you should find the binary **target/debug/view-function** or **target/release/view-function**.

## CLI Usage
### Run the CLI
`view-function -h`
```
Usage: view-function [OPTIONS] --func <FUNCTION>

Options:
  -F, --func <FUNCTION>
          Function name to call, e.g. 0x1::foo::bar.
  -T, --type_args <TYPE_ARGS>
          Type parameters, seperated by ',' e.g. 0x1::aptos_coin::AptosCoin.
  -A, --args <ARGS>
          Parameters, seperated by ',' e.g. foo, bar.
  -L, --ledger_version <LEDGER_VERSION>
          Ledger version, if not apply or 0, use the latest ledger version. [default: 0]
  -N, --network <NETWORK>
          Network to use, e.g. mainnet. [default: mainnet]
  -C, --config <CONFIG_FILE>
          Config file to use. [default: config.toml]
      --log_level <LOG_LEVEL>
          log level, one of 'Off', 'Error', 'Warn', 'Info', 'Debug', 'Trace'. [default: Off]
  -h, --help
          Print help information
  -V, --version
          Print version information
```
### Example
```shell
# command
view-function \
--func 0x1::coin::balance \
--type_args 0x1::aptos_coin::AptosCoin \
--args 0x21ddba785f3ae9c6f03664ab07e9ad83595a0fa5ca556cec2b9d9e7100db0f07 \
--ledger_version 35842267 \
--network mainnet
# output
{
  "log_path": "",
  "return_values": [
    "3120544100u64"
  ]
}
```
### Config file
You can also write some static configurations like log folder and network url settings to a local configuration file, the default file is `config.toml` in the tool running directory.
Here is an example:
```toml
[config]
log_folder = ".log"
cache_folder = "."
network_configs = { testnet = "https://fullnode.mainnet.aptoslabs.com/v1" }
```
### Logs
The default log folder is `.log` in the tool running directory.

## Web App
We also build a web app on top of the CLI tool, but it's for demo purpose only.
### To start the web service:
```shell
cd app/server
npm install
npm run start
```
### To start the frontend in dev mode
```shell
cd app/client
npm install
npm run serve
```
Now open `http://localhost:8080/`, you should find the App is up.
