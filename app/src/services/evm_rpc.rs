use anyhow::{anyhow, Result};
use serde::Deserialize;

#[derive(Clone)]
pub struct EvmRpcService {
    client: reqwest::Client,
    rpc_url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcLog {
    pub transaction_hash: Option<String>,
    pub block_number: Option<String>,
    pub log_index: Option<String>,
    pub topics: Vec<String>,
    pub data: String,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    message: String,
}

impl EvmRpcService {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            rpc_url: rpc_url.to_string(),
        }
    }

    pub async fn eth_call(&self, to: &str, data: &str) -> Result<String> {
        let params = serde_json::json!([
            {
                "to": to,
                "data": data
            },
            "latest"
        ]);
        let value = self.rpc_value_call("eth_call", params).await?;
        value
            .as_str()
            .map(|v| v.to_string())
            .ok_or_else(|| anyhow!("Invalid eth_call response"))
    }

    pub async fn eth_block_number(&self) -> Result<u64> {
        let value = self
            .rpc_value_call("eth_blockNumber", serde_json::json!([]))
            .await?;
        let raw = value
            .as_str()
            .ok_or_else(|| anyhow!("Invalid eth_blockNumber response"))?;
        parse_u64_hex(raw)
    }

    pub async fn eth_get_logs(
        &self,
        address: &str,
        topic0: &str,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<RpcLog>> {
        let params = serde_json::json!([
            {
                "address": address,
                "fromBlock": quantity_hex(from_block),
                "toBlock": quantity_hex(to_block),
                "topics": [topic0]
            }
        ]);
        let value = self.rpc_value_call("eth_getLogs", params).await?;
        serde_json::from_value(value).map_err(|e| anyhow!("Invalid eth_getLogs payload: {}", e))
    }

    pub async fn eth_get_block_timestamp(&self, block_number: u64) -> Result<u64> {
        let params = serde_json::json!([quantity_hex(block_number), false]);
        let value = self.rpc_value_call("eth_getBlockByNumber", params).await?;
        let timestamp = value
            .get("timestamp")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Invalid block payload"))?;
        parse_u64_hex(timestamp)
    }

    pub async fn eth_send_raw_transaction(&self, raw_tx: &str) -> Result<String> {
        let params = serde_json::json!([raw_tx]);
        let value = self
            .rpc_value_call("eth_sendRawTransaction", params)
            .await?;
        value
            .as_str()
            .map(|v| v.to_string())
            .ok_or_else(|| anyhow!("Invalid eth_sendRawTransaction response"))
    }

    async fn rpc_value_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let response = self
            .client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Base RPC request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Base RPC returned non-success status: {}",
                response.status()
            ));
        }

        let payload: JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to decode Base RPC response: {}", e))?;

        if let Some(error) = payload.error {
            return Err(anyhow!("Base RPC error: {}", error.message));
        }

        payload
            .result
            .ok_or_else(|| anyhow!("Base RPC response missing result"))
    }
}

pub fn quantity_hex(value: u64) -> String {
    format!("0x{:x}", value)
}

pub fn parse_u64_hex(value: &str) -> Result<u64> {
    let trimmed = value.trim_start_matches("0x");
    if trimmed.is_empty() {
        return Err(anyhow!("Invalid RPC hex value"));
    }

    let normalized = trimmed.trim_start_matches('0');
    if normalized.is_empty() {
        return Ok(0);
    }
    if normalized.len() > 16 {
        return Err(anyhow!("RPC value out of range for u64"));
    }

    u64::from_str_radix(normalized, 16).map_err(|_| anyhow!("Invalid RPC hex value"))
}
