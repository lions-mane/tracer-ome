//! Contains logic and type definitions for orders
use std::fmt;

use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use web3::types::{Address, U256};

use crate::util::{from_hex_de, from_hex_se};

/// Magic string representing the function signature
pub const FUNCTION_SIGNATURE: &str = "LimitOrder(uint256 amount,uint256 price,bool side,address user,uint256 expiration,address target_tracer)";

/// Magic pre-computed hash of the EIP712 domain prefix
pub const DOMAIN_HASH: &str =
    "49854490ba36fba358fe1019f097d8b566d011cfb3fd67c6fce6a40624150034";

/// Magic number prefix for EIP712
pub const EIP712_MAGIC_PREFIX: &str = "1901";

pub type OrderId = u64;

/// Represents which side of the market an order is on
///
/// This type is simply an enum with two fields:
/// - `Bid` (buy-side)
/// - `Ask` (sell-side)
#[derive(
    Clone, Copy, PartialEq, Eq, Debug, Display, Serialize, Deserialize,
)]
pub enum OrderSide {
    Bid,
    Ask,
}

impl OrderSide {
    /// Returns a byte slice of the market side
    ///
    /// This is simply one byte long as there will only ever be two market sides
    /// (realistically)
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            OrderSide::Bid => &[0x00],
            OrderSide::Ask => &[0x01],
        }
    }
}

/// Represents an actual order in the market
///
/// Comprises a struct with all order fields needed for the Tracer market.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Order {
    pub id: u64,                /* SHA3-256 hash of other fields */
    pub user: Address,          /* Ethereum address of trader */
    pub target_tracer: Address, /* Ethereum address of the Tracer smart contract */
    pub side: OrderSide,        /* side of the market of the order */
    #[serde(serialize_with = "from_hex_se", deserialize_with = "from_hex_de")]
    pub price: U256, /* price */
    #[serde(serialize_with = "from_hex_se", deserialize_with = "from_hex_de")]
    pub amount: U256, /* quantity */
    #[serde(with = "ts_seconds")]
    pub expiration: DateTime<Utc>, /* expiration of the order */
    pub signed_data: Vec<u8>,   /* digital signature of the order */
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "#{} [{}] {} {} @ {}",
            self.id, self.target_tracer, self.side, self.amount, self.price
        )
    }
}

/// Represents an error in interpreting a byte-level representation of an order
#[derive(Clone, Copy, Debug, Error, Serialize, Deserialize)]
pub enum OrderParseError {/* TODO: add specific errors here */}

impl Order {
    /// Constructor for the `Order` type
    ///
    /// Accepts all but the ID field of an order. Then calculates the order's ID
    /// and populates an `Order` struct.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user: Address,
        target_tracer: Address,
        side: OrderSide,
        price: U256,
        amount: U256,
        expiration: DateTime<Utc>,
        signed_data: Vec<u8>,
    ) -> Self {
        let id: OrderId = 0; /* TODO: determine how IDs are to be generated */

        Self {
            id,
            user,
            target_tracer,
            side,
            price,
            amount,
            expiration,
            signed_data,
        }
    }

    /// Returns a mutable reference to the unique identifier of this order
    pub fn id_mut(&mut self) -> &mut u64 {
        &mut self.id
    }

    /// Returns a mutable reference to the address of the owner of this order
    pub fn address_mut(&mut self) -> &mut Address {
        &mut self.user
    }

    /// Returns a mutable reference to the address of the market of this order
    pub fn market_mut(&mut self) -> &mut Address {
        &mut self.target_tracer
    }

    /// Returns a mutable reference to the market side of this order
    pub fn side_mut(&mut self) -> &mut OrderSide {
        &mut self.side
    }

    /// Returns a mutable reference to the price of this order
    pub fn price_mut(&mut self) -> &mut U256 {
        &mut self.price
    }

    /// Returns a mutable reference to the quantity of this order
    pub fn amount_mut(&mut self) -> &mut U256 {
        &mut self.amount
    }

    /// Returns a mutable reference to the expiration of this order
    pub fn expiration_mut(&mut self) -> &mut DateTime<Utc> {
        &mut self.expiration
    }
}
