use std::env;
use std::io;
use std::path::{PathBuf, Path};
use std::str::FromStr;
use aptos_sdk::rest_client::Client;
use path_clean::PathClean;
use url::Url;

pub fn absolute_path(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let path = path.as_ref();

    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()?.join(path)
    }.clean();

    Ok(absolute_path)
}

pub fn get_node_url(network: String) -> Url {
    return Url::from_str(format!("https://fullnode.{}.aptoslabs.com", network).as_str()).unwrap()
}

pub fn get_function_abi(client: Client) {
}