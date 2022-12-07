use clap::{command, Parser, ValueEnum};
use move_core_types::value::MoveValue;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use serde_json::Value;

#[derive(Serialize, Debug)]
pub struct ExecutionResult {
    pub(crate) log_path: String,
    pub(crate) return_values: Vec<Value>,
}

#[derive(ValueEnum, Deserialize, Eq, PartialEq, Hash, Clone, Copy, Debug)]
pub enum Network {
    Mainnet,
    Testnet,
    Devnet,
}

impl Display for Network {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Network::Mainnet => "mainnet",
            Network::Testnet => "testnet",
            Network::Devnet => "devnet",
        };
        write!(f, "{}", str)
    }
}

#[derive(ValueEnum, Copy, Clone, Debug)]
pub enum LogLevel {
    /// A level lower than all log levels.
    Off,
    /// Corresponds to the `Error` log level.
    Error,
    /// Corresponds to the `Warn` log level.
    Warn,
    /// Corresponds to the `Info` log level.
    Info,
    /// Corresponds to the `Debug` log level.
    Debug,
    /// Corresponds to the `Trace` log level.
    Trace,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            LogLevel::Off => "off",
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        };
        write!(f, "{}", str)
    }
}

/// Call the view function on Aptos blockchain
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct ViewFunction {
    /// Function name as `<ADDRESS>::<MODULE_ID>::<FUNCTION_NAME>`
    ///
    /// Example: `0x1::block::get_current_block_height`
    #[clap(short, long)]
    pub(crate) function_id: String,

    /// Arguments separated by spaces.
    ///
    /// Supported types [u8, u64, u128, bool, hex, string, address, raw]
    ///
    /// Example: `0x1 true 0`
    #[clap(short, long, num_args = 0..)]
    pub(crate) args: Option<Vec<String>>,

    /// TypeTag arguments separated by spaces.
    ///
    /// Example: `u8 u64 u128 bool address vector signer`
    #[clap(short, long, num_args = 0..)]
    pub(crate) type_args: Option<Vec<String>>,

    /// Ledger version, if not apply or 0, use the latest ledger version.
    #[clap(short, long, default_value_t = 0)]
    pub(crate) ledger_version: u64,

    /// Network to use.
    #[clap(short, long, default_value_t = Network::Mainnet)]
    pub(crate) network: Network,

    /// Config file to use.
    #[clap(short, long)]
    pub(crate) config: Option<String>,

    /// Log level.
    #[clap(long, default_value_t = LogLevel::Off)]
    pub(crate) log_level: LogLevel,
}
