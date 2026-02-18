use std::sync::OnceLock;

use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::types::TSError;
use crate::types::TSErrorType::RuntimeError;

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

#[derive(serde::Deserialize)]
struct JsonRpcStructure<T> {
    result: T,
}

fn get_http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(Client::new)
}

/// Perform an async JSON RPC request, returning the result as a T.
pub async fn do_rpc_request_as_struct<T: DeserializeOwned>(url: &str) -> Result<T, TSError> {
    let response = get_http_client()
        .get(url)
        .send()
        .await
        .map_err(|e| TSError::new(RuntimeError, format!("Failed making request to {url}: {e}")))?
        .error_for_status()
        .map_err(|e| TSError::new(RuntimeError, format!("Got a HTTP error when making a request to {url}: {e}")))?;

    let data: JsonRpcStructure<T> = response
        .json::<JsonRpcStructure<T>>()
        .await
        .map_err(|e| TSError::new(RuntimeError, format!("Failed deserialising JSON after request to {url}: {e}")))?;

    Ok(data.result)
}
