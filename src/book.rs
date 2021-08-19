#![allow(dead_code)]
//! Contains logic and type definitions for the order book itself and the
//! matching engine also
use std::{
    cmp::Ordering,
    collections::{BTreeMap, VecDeque},
    fmt::{Display, Formatter},
};

use chrono::{DateTime, ParseError, Utc};
use ethereum_types::{FromDecStrErr, U256};
use hex::FromHexError;
use itertools::Either;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use web3::types::Address;

use std::convert::TryFrom;
use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

use crate::order::{
    AddressWrapper, AddressWrapperError, ExternalOrder, Order, OrderId,
    OrderSide, Quantity,
};
use crate::util::{from_hex_de, from_hex_se};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Fill {
    pub maker: OrderId,
    pub taker: OrderId,
    pub quantity: Quantity,
    pub price: U256,
}

pub type Fills = Vec<Fill>;

/// Represents an order book for a particular Tracer market
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Book {
    pub market: Address, /* the address of the Tracer market */
    pub bids: BTreeMap<U256, VecDeque<Order>>, /* buy-side */
    pub asks: BTreeMap<U256, VecDeque<Order>>, /* sell-side */
    #[serde(
        serialize_with = "from_hex_se",
        deserialize_with = "from_hex_de",
        rename = "LTP"
    )]
    pub ltp: U256, /* last traded price */
    pub depth: (usize, usize), /* depth  */
    pub crossed: bool,   /* is book crossed? */
    #[serde(serialize_with = "from_hex_se", deserialize_with = "from_hex_de")]
    pub spread: U256, /* bid-ask spread */
}

#[derive(
    Clone, Copy, Debug, Display, Error, Serialize, Deserialize, PartialEq, Eq,
)]
pub enum BookError {
    Web3Error,
}

impl From<web3::Error> for BookError {
    fn from(_error: web3::Error) -> Self {
        BookError::Web3Error
    }
}

impl From<ethabi::Error> for BookError {
    fn from(_error: ethabi::Error) -> Self {
        BookError::Web3Error
    }
}

/// Represents an error in interpreting an external order book
#[derive(Clone, Copy, Debug, Error, Serialize, Deserialize)]
pub enum BookParseError {
    InvalidHexadecimal,
    InvalidSide,
    InvalidTimestamp,
    IntegerBounds,
    InvalidDecimal,
    AddressWrapperError,
    FromDecStrError,
}

impl Display for BookParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::InvalidHexadecimal => write!(f, "Invalid hexadecimal"),
            _ => write!(f, "Unknown"),
        }
    }
}

impl From<FromHexError> for BookParseError {
    fn from(_value: FromHexError) -> Self {
        BookParseError::InvalidHexadecimal
    }
}

impl From<rustc_hex::FromHexError> for BookParseError {
    fn from(_value: rustc_hex::FromHexError) -> Self {
        BookParseError::InvalidHexadecimal
    }
}

impl From<ParseError> for BookParseError {
    fn from(_value: ParseError) -> Self {
        BookParseError::InvalidTimestamp
    }
}

impl From<ParseIntError> for BookParseError {
    fn from(_value: ParseIntError) -> Self {
        BookParseError::IntegerBounds
    }
}

impl From<AddressWrapperError> for BookParseError {
    fn from(_value: AddressWrapperError) -> Self {
        BookParseError::AddressWrapperError
    }
}

impl From<FromDecStrErr> for BookParseError {
    fn from(_value: FromDecStrErr) -> Self {
        BookParseError::FromDecStrError
    }
}

#[derive(
    Clone, Copy, Debug, Display, Error, Serialize, Deserialize, PartialEq, Eq,
)]
pub enum OrderStatus {
    Placed,
    PartialMatch,
    FullMatch,
}

pub struct MatchResult {
    pub fills: Fills,
    pub order_status: OrderStatus,
}

impl Book {
    /// Constructor for the `Book` type
    ///
    /// Takes the address of the underlying Tracer contract as its sole
    /// argument, then initialises both sides of the book to be empty.
    pub fn new(market: Address) -> Self {
        Self {
            market,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            ltp: Default::default(),
            depth: (0, 0),
            crossed: false,
            spread: Default::default(),
        }
    }

    /// Returns the ticker of this market
    pub fn market(&self) -> &Address {
        &self.market
    }

    /// Returns a reference to the order matching the provided order ID
    pub fn order(&self, id: OrderId) -> Option<&Order> {
        /* search bids */
        for (_, curr_level) in self.bids.iter() {
            for curr_order in curr_level.iter() {
                if curr_order.id == id {
                    return Some(curr_order);
                }
            }
        }

        /* search asks */
        for (_, curr_level) in self.asks.iter() {
            for curr_order in curr_level.iter() {
                if curr_order.id == id {
                    return Some(curr_order);
                }
            }
        }

        None
    }

    /// Returns a mutable reference to the order matching the provided order ID
    pub fn order_mut(&mut self, id: OrderId) -> Option<&mut Order> {
        /* search bids */
        for (_, curr_level) in self.bids.iter_mut() {
            for curr_order in curr_level.iter_mut() {
                if curr_order.id == id {
                    return Some(curr_order);
                }
            }
        }

        /* search asks */
        for (_, curr_level) in self.asks.iter_mut() {
            for curr_order in curr_level.iter_mut() {
                if curr_order.id == id {
                    return Some(curr_order);
                }
            }
        }

        None
    }

    /// Returns the last traded price of the order book
    pub fn ltp(&self) -> U256 {
        self.ltp
    }

    /// Returns a pair (2-tuple) containing the depths of each side of the book
    pub fn depth(&self) -> (usize, usize) {
        (
            self.bids
                .values()
                .flatten()
                .filter(|order| !order.remaining.is_zero())
                .count(),
            self.asks
                .values()
                .flatten()
                .filter(|order| !order.remaining.is_zero())
                .count(),
        )
    }

    /// Returns whether the order book is currently crossed or not
    pub fn crossed(&self) -> bool {
        self.crossed
    }

    /// Returns the bid-ask spread of the book
    pub fn spread(&self) -> U256 {
        self.spread
    }

    pub fn top(&self) -> (Option<U256>, Option<U256>) {
        (
            self.bids.last_key_value().map(|t| *t.0),
            self.asks.first_key_value().map(|t| *t.0),
        )
    }

    fn price_viable(
        opposite: U256,
        incoming: U256,
        incoming_side: OrderSide,
    ) -> bool {
        match incoming_side {
            OrderSide::Bid => opposite <= incoming,
            OrderSide::Ask => opposite >= incoming,
        }
    }

    fn build_match_result(
        order_status: OrderStatus,
        fills: Fills,
    ) -> MatchResult {
        MatchResult {
            fills,
            order_status,
        }
    }

    fn build_fill(
        maker: OrderId,
        taker: OrderId,
        quantity: Quantity,
        price: U256,
    ) -> Fill {
        Fill {
            maker,
            taker,
            quantity,
            price,
        }
    }

    #[allow(unused_must_use)]
    async fn r#match(
        &mut self,
        mut order: Order,
        opposing_top: Option<U256>,
    ) -> Result<MatchResult, BookError> {
        info!("Matching {}...", order);

        let mut fills: Fills = Vec::new();

        let opposing_side: &mut BTreeMap<U256, VecDeque<Order>> =
            match order.side {
                OrderSide::Bid => &mut self.asks,
                OrderSide::Ask => &mut self.bids,
            };
        let mut running_total: U256 = order.remaining;
        let mut done: bool = false;

        /* if we haven't crossed the spread, we're not going to match */
        if opposing_top.is_none()
            || !Book::price_viable(
                opposing_top.unwrap(),
                order.price,
                order.side,
            )
        {
            info!("{} does not cross, adding...", order);
            self.add_order(order);
            return Ok(Book::build_match_result(OrderStatus::Placed, fills));
        }

        let opposing_side_iterator = match order.side {
            OrderSide::Bid => Either::Left(opposing_side.iter_mut()),
            OrderSide::Ask => Either::Right(opposing_side.iter_mut().rev()),
        };

        for (price, opposites) in opposing_side_iterator {
            /* if we've run out of viable prices or we're done, halt */
            if done || !Book::price_viable(*price, order.price, order.side) {
                break;
            }

            for opposite in opposites {
                /* no self-trading allowed */
                if opposite.trader == order.trader {
                    info!("Self-trade, skipping...");
                    continue;
                }

                /* determine how much to match */
                let amount: U256 =
                    match opposite.remaining.cmp(&order.remaining) {
                        Ordering::Greater => order.remaining,
                        _ => opposite.remaining,
                    };
                info!("Matching with amount of {}...", amount);

                /* match */
                order = Book::fill(order, amount);
                *opposite = Book::fill(opposite.clone(), amount);

                fills.push(Book::build_fill(
                    opposite.id,
                    order.id,
                    amount,
                    opposite.price,
                ));

                self.ltp = *price;
                info!("LTP updated, is now {}", self.ltp);

                running_total -= amount;

                /* check if we've totally matched our incoming order */
                if running_total.is_zero() {
                    info!("Totally matched {}", order);
                    done = true;
                    break;
                }
            }
        }

        /* if our incoming order has any volume left, add it to the book */
        if running_total > U256::zero() {
            self.add_order(order);
            Ok(Book::build_match_result(OrderStatus::PartialMatch, fills))
        } else {
            Ok(Book::build_match_result(OrderStatus::FullMatch, fills))
        }
    }

    fn fill(order: Order, amount: U256) -> Order {
        info!("Filling {} of {}...", amount, order);
        match amount.cmp(&order.remaining) {
            Ordering::Greater => order,
            _ => Order {
                id: order.id,
                trader: order.trader,
                market: order.market,
                side: order.side,
                price: order.price,
                quantity: order.quantity,
                remaining: order.remaining - amount,
                expiration: order.expiration,
                created: order.created,
                signed_data: order.signed_data,
            },
        }
    }

    fn prune(&mut self) {
        for (_price, orders) in self.bids.iter_mut() {
            orders.retain(|order| !order.remaining.is_zero());
        }

        for (_price, orders) in self.asks.iter_mut() {
            orders.retain(|order| !order.remaining.is_zero());
        }

        self.bids.retain(|_price, orders| !orders.is_empty());
        self.asks.retain(|_price, orders| !orders.is_empty());
    }

    /// Submits an order to the matching engine
    ///
    /// In the event the order cannot be (fully) matched, it will be stored
    /// in the order book for future matching.
    pub async fn submit(
        &mut self,
        order: Order,
    ) -> Result<MatchResult, BookError> {
        info!("Submitting {}...", order);

        let match_result: Result<MatchResult, BookError> = match order.side {
            OrderSide::Bid => self.r#match(order, self.top().1).await,
            OrderSide::Ask => self.r#match(order, self.top().0).await,
        };

        self.update();

        match_result
    }

    #[allow(clippy::unnecessary_wraps)]
    fn add_order(&mut self, order: Order) -> Result<(), BookError> {
        info!("Adding {}...", order);

        let tmp_order: Order = order.clone();
        let order_side = order.side;
        let order_price = order.price;
        let orders = VecDeque::new();

        match order_side {
            OrderSide::Bid => {
                self.bids
                    .entry(order_price)
                    .or_insert(orders)
                    .push_back(order);
                info!("Added to bid-side");
            }
            OrderSide::Ask => {
                self.asks
                    .entry(order_price)
                    .or_insert(orders)
                    .push_back(order);
                info!("Added to ask-side");
            }
        }

        info!("Added {}", tmp_order);

        Ok(())
    }

    /*******************HELPER FUNCTIONS FOR TESTING END************************/

    /// Cancels the open order currently in the order book with the matching ID
    ///
    /// # Returns #
    ///
    /// Returns `Ok(Some(dt))` upon success, where `dt` is a `DateTime<Utc>`
    /// type representing the time of successful cancellation of the order.
    ///
    /// Returns `Ok(None)` if there is no such order currently in the book.
    ///
    /// Returns a `BookError` if there is an error condition
    #[allow(unused_variables)] /* TODO: remove when cancel is implemented */
    pub fn cancel(
        &mut self,
        order_id: OrderId,
    ) -> Result<Option<DateTime<Utc>>, BookError> {
        for (_, orders) in self.bids.iter_mut() {
            for (index, order) in orders.iter_mut().enumerate() {
                if order.id == order_id {
                    info!("Cancelled {}", order.clone());
                    orders.remove(index);
                    return Ok(Some(Utc::now()));
                }
            }
        }

        for (_, orders) in self.asks.iter_mut() {
            for (index, order) in orders.iter_mut().enumerate() {
                if order.id == order_id {
                    info!("Cancelled {}", order.clone());
                    orders.remove(index);
                    return Ok(Some(Utc::now()));
                }
            }
        }

        Ok(None)
    }

    /// Updates internal metadata of the order book
    ///
    /// Should be called *after successful* mutation of order book state.
    #[allow(dead_code)]
    fn update(&mut self) {
        self.prune();
        self.depth = self.depth();
        info!("Updated book metadata");
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ExternalBook {
    pub market: String, /* the address of the Tracer market */
    pub bids: BTreeMap<String, VecDeque<ExternalOrder>>, /* buy-side */
    pub asks: BTreeMap<String, VecDeque<ExternalOrder>>, /* sell-side */
    pub ltp: String,    /* last traded price */
    pub depth: (usize, usize), /* depth  */
    pub crossed: bool,  /* is book crossed? */
    pub spread: String, /* bid-ask spread */
}

impl From<Book> for ExternalBook {
    fn from(value: Book) -> Self {
        Self {
            market: AddressWrapper::from(value.market).to_hex_string(),
            bids: value
                .bids
                .iter()
                .map(|(price, orders)| {
                    (
                        price.to_string(),
                        orders
                            .iter()
                            .map(|order| ExternalOrder::from(order.clone()))
                            .collect(),
                    )
                })
                .collect(),
            asks: value
                .asks
                .iter()
                .map(|(price, orders)| {
                    (
                        price.to_string(),
                        orders
                            .iter()
                            .map(|order| ExternalOrder::from(order.clone()))
                            .collect(),
                    )
                })
                .collect(),
            ltp: value.ltp.to_string(),
            depth: value.depth,
            crossed: value.crossed,
            spread: value.spread.to_string(),
        }
    }
}

impl TryFrom<ExternalBook> for Book {
    type Error = BookParseError;

    fn try_from(value: ExternalBook) -> Result<Self, Self::Error> {
        let market: Address = match AddressWrapper::from_str(&value.market) {
            Ok(t) => Address::from(t),
            Err(e) => return Err(e.into()),
        };

        let bids: BTreeMap<U256, VecDeque<Order>> = value
            .bids
            .iter()
            .map(|(price, orders)| {
                (
                    U256::from_dec_str(price).unwrap(),
                    orders
                        .iter()
                        .map(|order| Order::try_from(order.clone()).unwrap())
                        .collect(),
                )
            })
            .collect();

        let asks: BTreeMap<U256, VecDeque<Order>> = value
            .asks
            .iter()
            .map(|(price, orders)| {
                (
                    U256::from_dec_str(price).unwrap(),
                    orders
                        .iter()
                        .map(|order| Order::try_from(order.clone()).unwrap())
                        .collect(),
                )
            })
            .collect();

        let ltp: U256 = match U256::from_dec_str(&value.ltp) {
            Ok(t) => t,
            Err(e) => return Err(e.into()),
        };

        let spread: U256 = match U256::from_dec_str(&value.spread) {
            Ok(t) => t,
            Err(e) => return Err(e.into()),
        };

        Ok(Self {
            market,
            bids,
            asks,
            ltp,
            depth: value.depth,
            crossed: value.crossed,
            spread,
        })
    }
}
