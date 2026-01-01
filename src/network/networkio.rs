use serde::de::DeserializeOwned;

#[derive(serde::Deserialize)]
struct JSONRPCStructure<T> {
    result: T,
}

/**
 * Perform an asnyc JSON RPC request, returning the result as a serialised vector of T. Panics
 * if deserialisation fails.
 */
pub async fn do_rpc_request_as_struct<T: DeserializeOwned>(url: &str) -> Result<T, reqwest::Error> {
    let response = reqwest::Client::new().get(url).send().await?.error_for_status()?;

    let data: JSONRPCStructure<T> = match response.json::<JSONRPCStructure<T>>().await {
        Ok(v) => v,
        Err(e) => {
            panic!("Failed deserialising JSON after request to {url}: {e:#?}")
        }
    };

    Ok(data.result)
}
