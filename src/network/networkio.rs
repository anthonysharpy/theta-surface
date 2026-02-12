use serde::de::DeserializeOwned;

use crate::types::TSError;
use crate::types::TSErrorType::RuntimeError;

#[derive(serde::Deserialize)]
struct JSONRPCStructure<T> {
    result: T,
}

/**
 * Perform an asnyc JSON RPC request, returning the result as a serialised vector of T. Panics
 * if deserialisation fails.
 */
pub async fn do_rpc_request_as_struct<T: DeserializeOwned>(url: &str) -> Result<T, TSError> {
    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|e| TSError::new(RuntimeError, format!("Failed making request to {url}: {e}")))?
        .error_for_status()
        .map_err(|e| TSError::new(RuntimeError, format!("Got a HTML error when making a request to {url}: {e}")))?;

    let data: JSONRPCStructure<T> = response
        .json::<JSONRPCStructure<T>>()
        .await
        .map_err(|e| TSError::new(RuntimeError, format!("Failed deserialising JSON after request to {url}: {e}")))?;

    Ok(data.result)
}
