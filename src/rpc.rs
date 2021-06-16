use std::fmt::Display;
use std::str::FromStr;

use reqwest::{header, Client, Response};
use serde::{Deserialize, Serialize};
use web3::types::H160;

use crate::order::{ExternalOrder, Order};

#[derive(Display, Debug)]
pub enum RpcError {
    HttpError,
    ContractError,
    InvalidResponse,
}

impl From<reqwest::Error> for RpcError {
    fn from(_value: reqwest::Error) -> Self {
        Self::HttpError
    }
}

impl From<rustc_hex::FromHexError> for RpcError {
    fn from(_value: rustc_hex::FromHexError) -> Self {
        Self::InvalidResponse
    }
}

#[derive(Serialize, Deserialize)]
pub struct MatchRequest {
    maker: ExternalOrder,
    taker: ExternalOrder,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckRequest {
    order: ExternalOrder,
}

#[allow(unused_must_use)]
pub async fn check_order_validity(
    order: Order,
    address: String,
) -> Result<bool, RpcError> {
    let endpoint: String = address + "/check";
    let client: Client = Client::new();
    let payload: CheckRequest = CheckRequest {
        order: ExternalOrder::from(order.clone()),
    };

    info!(
        "Checking order validity by sending {} to {}...",
        order, endpoint
    );

    let response: Response = match client
        .post(endpoint.clone())
        .header(header::CONTENT_TYPE, "application/json")
        .body(serde_json::to_string(&payload).unwrap())
        .send()
        .await
    {
        Ok(t) => t,
        Err(e) => return Err(e.into()),
    };

    info!("{} said {}", endpoint, response.status());

    Ok(response.status().is_success())
}

pub async fn send_matched_orders(
    maker: Order,
    taker: Order,
    address: String,
) -> Result<H160, RpcError> {
    info!(
        "Forwarding matched pair ({}, {}) to {}...",
        maker, taker, address
    );

    let payload: MatchRequest = MatchRequest {
        maker: maker.into(),
        taker: taker.into(),
    };
    let client: Client = Client::new();
    let endpoint: String = address.clone() + "/submit";

    /* post the matched orders to the forwarder */
    let result: Response = match client
        .post(endpoint)
        .header(header::CONTENT_TYPE, "application/json")
        .body(serde_json::to_string(&payload).unwrap())
        .send()
        .await
    {
        Ok(t) => t,
        Err(e) => {
            return Err(RpcError::from(e));
        }
    };

    info!("{} said {}", address, result.status());

    /* extract the transaction hash from the response body */
    let hash: H160 = match result.text().await {
        Ok(t) => match H160::from_str(&t) {
            Ok(s) => s,
            Err(l) => {
                return Err(RpcError::from(l));
            }
        },
        Err(e) => return Err(RpcError::from(e)),
    };

    Ok(hash)
}
