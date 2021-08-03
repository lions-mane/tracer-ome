use std::collections::{BTreeMap, VecDeque};

use chrono::{DateTime, NaiveDateTime, Utc};
use ethereum_types::{Address, U256};

use crate::book::{
    Book, BookError, ExternalBook, Fill, MatchResult, OrderStatus,
};
use crate::order::{Order, OrderSide};

use std::convert::TryFrom;

pub const TEST_RPC_ADDRESS: &str = "http://localhost:3000";

// https://stackoverflow.com/questions/29504514/whats-the-best-way-to-compare-2-vectors-or-strings-element-by-element
fn do_vecs_match<T: PartialEq>(a: &Vec<T>, b: &Vec<T>) -> bool {
    let matching = a.iter().zip(b.iter()).filter(|&(a, b)| a == b).count();
    matching == a.len() && matching == b.len()
}

async fn submit_orders(
    market: Address,
    data: Vec<(Address, OrderSide, u64, u64)>,
) -> Book {
    /* build orders from supplied parameters */
    let orders: Vec<Order> = data
        .iter()
        .map(|(addr, side, price, qty)| {
            Order::new(
                *addr,
                market,
                *side,
                (*price).into(),
                (*qty).into(),
                Utc::now(),
                Utc::now(),
                vec![],
            )
        })
        .collect();

    let mut book: Book = Book::new(market);

    /* apply each order to the book (sadly we can't `map` here due to our blocking requirement) */
    for order in orders {
        book.submit(order.clone())
            .await
            .expect("Failed to submit order to book");
    }

    book
}

async fn setup() -> Book {
    let market: Address = Address::zero();

    /* placeholders for trader addresses (saves us writing real Ethereum addresses) */
    let traders: Vec<Address> =
        (0..10).map(|x| Address::from_low_u64_be(x)).collect();

    let asks: Vec<(Address, OrderSide, u64, u64)> = vec![
        (traders[0], OrderSide::Ask, 100, 10),
        (traders[1], OrderSide::Ask, 99, 2),
        (traders[2], OrderSide::Ask, 98, 35),
        (traders[3], OrderSide::Ask, 97, 15),
        (traders[4], OrderSide::Ask, 96, 5),
    ];

    let bids: Vec<(Address, OrderSide, u64, u64)> = vec![
        (traders[5], OrderSide::Bid, 95, 10),
        (traders[6], OrderSide::Bid, 94, 20),
        (traders[7], OrderSide::Bid, 93, 5),
        (traders[8], OrderSide::Bid, 92, 10),
        (traders[9], OrderSide::Bid, 91, 15),
    ];

    let orders: Vec<(Address, OrderSide, u64, u64)> =
        bids.iter().cloned().chain(asks.iter().cloned()).collect();

    submit_orders(market, orders).await
}

#[tokio::test]
pub async fn test_new_book() {
    let market: Address = Address::zero();
    let book = Book::new(market);

    assert_eq!(book.market(), &market);
    assert_eq!(book.depth(), (0, 0)); // Asserts that there are no orders in the book.
}

#[tokio::test]
pub async fn test_book_depth() {
    let book = setup().await;

    let (bid_length, ask_length) = book.depth();

    assert_eq!(bid_length, 5);
    assert_eq!(ask_length, 5);
}

#[tokio::test]
pub async fn test_simple_buy() {
    let mut book = setup().await;
    let bid = Order::new(
        Address::from_low_u64_be(3),
        Address::zero(),
        OrderSide::Bid,
        U256::from_dec_str(&"96").unwrap(),
        U256::from_dec_str(&"5").unwrap(),
        Utc::now(),
        Utc::now(),
        vec![],
    );

    let submit_res: Result<MatchResult, BookError> = book.submit(bid).await;

    let (bid_length, ask_length) = book.depth();

    assert!(
        submit_res.is_ok() || submit_res.contains_err(&BookError::Web3Error)
    );

    assert_eq!(bid_length, 5);
    assert_eq!(ask_length, 4);
}

#[tokio::test]
pub async fn test_simple_buy_partially_filled() {
    let mut book = setup().await;

    let market_address = Address::zero();
    let bid = Order::new(
        Address::from_low_u64_be(3),
        market_address,
        OrderSide::Bid,
        U256::from_dec_str(&"96").unwrap(),
        U256::from_dec_str(&"3").unwrap(),
        Utc::now(),
        Utc::now(),
        vec![],
    );

    let submit_res: Result<MatchResult, BookError> = book.submit(bid).await;

    let (bid_length, ask_length) = book.depth();

    assert!(
        submit_res.is_ok() || submit_res.contains_err(&BookError::Web3Error)
    );

    // Ensure the depths are correct
    assert_eq!(bid_length, 5);
    assert_eq!(ask_length, 5);
}

#[tokio::test]
pub async fn test_simple_sell() {
    let mut book = setup().await;
    let ask = Order::new(
        Address::from_low_u64_be(3),
        Address::zero(),
        OrderSide::Ask,
        U256::from_dec_str(&"95").unwrap(),
        U256::from_dec_str(&"10").unwrap(),
        Utc::now(),
        Utc::now(),
        vec![],
    );

    let submit_res: Result<MatchResult, BookError> = book.submit(ask).await;

    let (bid_length, ask_length) = book.depth();

    assert!(
        submit_res.is_ok() || submit_res.contains_err(&BookError::Web3Error)
    );

    assert_eq!(bid_length, 4);
    assert_eq!(ask_length, 5);
}

#[tokio::test]
pub async fn test_simple_sell_partially_filled() {
    let mut book = setup().await;

    let market_address = Address::zero();
    let bid = Order::new(
        Address::from_low_u64_be(3),
        market_address,
        OrderSide::Ask,
        U256::from_dec_str(&"95").unwrap(),
        U256::from_dec_str(&"1").unwrap(),
        Utc::now(),
        Utc::now(),
        vec![],
    );

    let submit_res: Result<MatchResult, BookError> = book.submit(bid).await;

    let (bid_length, ask_length) = book.depth();

    assert!(
        submit_res.is_ok() || submit_res.contains_err(&BookError::Web3Error)
    );

    // Ensure the depths are correct
    assert_eq!(bid_length, 5);
    assert_eq!(ask_length, 5);
}

#[tokio::test]
pub async fn test_deep_buy() {
    let mut book = setup().await;
    let market_address = Address::zero();
    let bid = Order::new(
        Address::from_low_u64_be(10),
        market_address,
        OrderSide::Bid,
        U256::from_dec_str(&"99").unwrap(),
        U256::from_dec_str(&"42").unwrap(),
        Utc::now(),
        Utc::now(),
        vec![],
    );

    let submit_res: Result<MatchResult, BookError> = book.submit(bid).await;

    let (bid_length, ask_length) = book.depth();

    assert!(
        submit_res.is_ok() || submit_res.contains_err(&BookError::Web3Error)
    );

    // Ensure the depths are correct
    assert_eq!(bid_length, 5);
    assert_eq!(ask_length, 3);
}

#[tokio::test]
pub async fn test_no_self_matching() {
    let mut book = setup().await;
    let market: Address = Address::zero();

    let bid: Order = Order::new(
        Address::from_low_u64_be(1), /* previously placed a ask */
        market,
        OrderSide::Bid,
        97.into(),
        15.into(),
        Utc::now(),
        Utc::now(),
        vec![],
    );

    let actual_res: Result<MatchResult, BookError> = book.submit(bid).await;

    let (bid_depth, ask_depth) = book.depth();

    assert!(actual_res.is_ok());
    assert_eq!(actual_res.unwrap().order_status, OrderStatus::FullMatch);
    assert_eq!(bid_depth, 5);
    assert_eq!(ask_depth, 4);
}

#[tokio::test]
pub async fn test_no_self_matching_when_last_order() {
    let market: Address = Address::zero();
    let mut book = Book::new(market);

    let ask: Order = Order::new(
        Address::from_low_u64_be(1),
        market,
        OrderSide::Ask,
        97.into(),
        15.into(),
        Utc::now(),
        Utc::now(),
        vec![],
    );

    let bid: Order = Order::new(
        Address::from_low_u64_be(1), /* previously placed a ask */
        market,
        OrderSide::Bid,
        97.into(),
        15.into(),
        Utc::now(),
        Utc::now(),
        vec![],
    );

    book.submit(ask).await.unwrap();

    let actual_res: Result<MatchResult, BookError> = book.submit(bid).await;

    let (bid_depth, ask_depth) = book.depth();

    assert!(actual_res.is_ok());
    assert_eq!(actual_res.unwrap().order_status, OrderStatus::PartialMatch);
    assert_eq!(bid_depth, 1);
    assert_eq!(ask_depth, 1);
}

#[tokio::test]
pub async fn test_deep_buy_with_limit() {
    let mut book = setup().await;
    let market_address = Address::zero();
    let bid = Order::new(
        Address::from_low_u64_be(10),
        market_address,
        OrderSide::Bid,
        U256::from_dec_str(&"97").unwrap(),
        U256::from_dec_str(&"42").unwrap(),
        Utc::now(),
        Utc::now(),
        vec![],
    );

    let submit_res: Result<MatchResult, BookError> = book.submit(bid).await;

    let (bid_length, ask_length) = book.depth();

    assert!(
        submit_res.is_ok() || submit_res.contains_err(&BookError::Web3Error)
    );

    assert_eq!(bid_length, 6); // There should be one more bid with 22 units at 97.
    assert_eq!(ask_length, 3);
}

#[tokio::test]
pub async fn test_deep_sell() {
    let mut book = setup().await;
    let market_address = Address::zero();
    let ask = Order::new(
        Address::from_low_u64_be(10),
        market_address,
        OrderSide::Ask,
        U256::from_dec_str(&"94").unwrap(),
        U256::from_dec_str(&"20").unwrap(),
        Utc::now(),
        Utc::now(),
        vec![],
    );

    let submit_res: Result<MatchResult, BookError> = book.submit(ask).await;

    let (bid_length, ask_length) = book.depth();

    assert!(
        submit_res.is_ok() || submit_res.contains_err(&BookError::Web3Error)
    );

    // Ensure the depths are correct
    assert_eq!(bid_length, 4);
    assert_eq!(ask_length, 5);
}

#[tokio::test]
pub async fn test_deep_sell_with_limit() {
    let mut book = setup().await;
    let market_address = Address::zero();
    let ask = Order::new(
        Address::from_low_u64_be(33),
        market_address,
        OrderSide::Ask,
        U256::from_dec_str(&"94").unwrap(),
        U256::from_dec_str(&"35").unwrap(),
        Utc::now(),
        Utc::now(),
        vec![],
    );

    let submit_res: Result<MatchResult, BookError> = book.submit(ask).await;

    let (bid_length, ask_length) = book.depth();

    assert!(
        submit_res.is_ok() || submit_res.contains_err(&BookError::Web3Error)
    );

    assert_eq!(bid_length, 3);
    assert_eq!(ask_length, 6); // There should be one more ask with 5 units at 94.
}

#[tokio::test]
pub async fn test_partial_matching_mutability() {
    /* need at least three for this test */
    let traders: Vec<Address> =
        vec![Address::random(), Address::random(), Address::random()];
    let the_far_future: DateTime<Utc> = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(1699025703, 0),
        Utc,
    );
    let market: Address = Address::zero();

    let orders: Vec<Order> = vec![
        /* SHORT @ $1.15 for 1.2 */
        Order::new(
            traders[0],
            market,
            OrderSide::Ask,
            U256::from_dec_str("1150000000000000000").unwrap(),
            U256::from_dec_str("1200000000000000000").unwrap(),
            the_far_future,
            Utc::now(),
            vec![],
        ),
        /* LONG @ $1.20 for 1 */
        Order::new(
            traders[1],
            market,
            OrderSide::Bid,
            U256::from_dec_str("1200000000000000000").unwrap(),
            U256::from_dec_str("1000000000000000000").unwrap(),
            the_far_future,
            Utc::now(),
            vec![],
        ),
        /* LONG @ $1.20 for 1 */
        Order::new(
            traders[2],
            market,
            OrderSide::Bid,
            U256::from_dec_str("1200000000000000000").unwrap(),
            U256::from_dec_str("1000000000000000000").unwrap(),
            the_far_future,
            Utc::now(),
            vec![],
        ),
    ];

    let mut actual_book: Book = Book::new(market);

    for order in orders.iter() {
        actual_book.submit(order.clone()).await.unwrap();
    }

    let expected_book: Book = Book {
        market,
        bids: {
            let mut side: BTreeMap<U256, VecDeque<Order>> = BTreeMap::new();
            let mut level: VecDeque<Order> = VecDeque::new();
            let mut order: Order = orders[2].clone();
            order.remaining =
                U256::from_dec_str("0800000000000000000").unwrap();
            level.push_back(order);
            side.insert(orders[2].clone().price, level);
            side
        },
        asks: BTreeMap::new(),
        ltp: orders[0].price, // trade price is whichever order came first eg make
        depth: (1, 0),
        crossed: false,
        spread: U256::from_dec_str("0").unwrap(), // todo check how this is calculated
    };

    assert_eq!(actual_book, expected_book);
}

#[tokio::test]
pub async fn test_fills_output_order_placed() {
    /* need at least three for this test */
    let traders: Vec<Address> =
        vec![Address::random(), Address::random(), Address::random()];
    let the_far_future: DateTime<Utc> = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(1699025703, 0),
        Utc,
    );
    let market: Address = Address::zero();

    let order: Order = Order::new(
        traders[0],
        market,
        OrderSide::Ask,
        U256::from_dec_str("1150000000000000000").unwrap(),
        U256::from_dec_str("1200000000000000000").unwrap(),
        the_far_future,
        Utc::now(),
        vec![],
    );

    let mut actual_book: Book = Book::new(market);

    let match_result = actual_book.submit(order.clone()).await.unwrap();

    assert_eq!(match_result.order_status, OrderStatus::Placed);
    assert!(match_result.fills.is_empty());
}

#[tokio::test]
pub async fn test_fills_output_taker_partially_matched_multiple_makers() {
    /* need at least three for this test */
    let traders: Vec<Address> =
        vec![Address::random(), Address::random(), Address::random()];
    let the_far_future: DateTime<Utc> = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(1699025703, 0),
        Utc,
    );
    let market: Address = Address::zero();

    let orders: Vec<Order> = vec![
        /* SHORT @ $1 for 0.5 */
        Order::new(
            traders[0],
            market,
            OrderSide::Ask,
            U256::from_dec_str("1000000000000000000").unwrap(),
            U256::from_dec_str("0500000000000000000").unwrap(),
            the_far_future,
            Utc::now(),
            vec![],
        ),
        /* SHORT @ $1 for 0.25 */
        Order::new(
            traders[1],
            market,
            OrderSide::Ask,
            U256::from_dec_str("1000000000000000000").unwrap(),
            U256::from_dec_str("0250000000000000000").unwrap(),
            the_far_future,
            Utc::now(),
            vec![],
        ),
    ];

    let mut actual_book: Book = Book::new(market);

    for order in orders.iter() {
        actual_book.submit(order.clone()).await.unwrap();
    }

    // Long @ $1 for 1
    let order: Order = Order::new(
        traders[2],
        market,
        OrderSide::Bid,
        U256::from_dec_str("1000000000000000000").unwrap(),
        U256::from_dec_str("1000000000000000000").unwrap(),
        the_far_future,
        Utc::now(),
        vec![],
    );

    let match_result = actual_book.submit(order.clone()).await.unwrap();

    assert_eq!(match_result.order_status, OrderStatus::PartialMatch);

    let expected_fills: Vec<Fill> = vec![
        Fill {
            maker: orders[0].id,
            taker: order.id,
            price: U256::from_dec_str("1000000000000000000").unwrap(),
            quantity: U256::from_dec_str("0500000000000000000").unwrap(),
        },
        Fill {
            maker: orders[1].id,
            taker: order.id,
            price: U256::from_dec_str("1000000000000000000").unwrap(),
            quantity: U256::from_dec_str("0250000000000000000").unwrap(),
        },
    ];

    assert!(do_vecs_match(&match_result.fills, &expected_fills));
}

#[tokio::test]
pub async fn test_fills_output_taker_fully_matched_multiple_makers() {
    /* need at least three for this test */
    let traders: Vec<Address> =
        vec![Address::random(), Address::random(), Address::random()];
    let the_far_future: DateTime<Utc> = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(1699025703, 0),
        Utc,
    );
    let market: Address = Address::zero();

    let orders: Vec<Order> = vec![
        /* SHORT @ $1 for 0.5 */
        Order::new(
            traders[0],
            market,
            OrderSide::Ask,
            U256::from_dec_str("1000000000000000000").unwrap(),
            U256::from_dec_str("0500000000000000000").unwrap(),
            the_far_future,
            Utc::now(),
            vec![],
        ),
        /* SHORT @ $1 for 0.25 */
        Order::new(
            traders[1],
            market,
            OrderSide::Ask,
            U256::from_dec_str("1000000000000000000").unwrap(),
            U256::from_dec_str("0750000000000000000").unwrap(),
            the_far_future,
            Utc::now(),
            vec![],
        ),
    ];

    let mut actual_book: Book = Book::new(market);

    for order in orders.iter() {
        actual_book.submit(order.clone()).await.unwrap();
    }

    // Long @ $1 for 1
    let order: Order = Order::new(
        traders[2],
        market,
        OrderSide::Bid,
        U256::from_dec_str("1000000000000000000").unwrap(),
        U256::from_dec_str("1000000000000000000").unwrap(),
        the_far_future,
        Utc::now(),
        vec![],
    );

    let match_result = actual_book.submit(order.clone()).await.unwrap();

    assert_eq!(match_result.order_status, OrderStatus::FullMatch);

    let expected_fills: Vec<Fill> = vec![
        Fill {
            maker: orders[0].id,
            taker: order.id,
            price: U256::from_dec_str("1000000000000000000").unwrap(),
            quantity: U256::from_dec_str("0500000000000000000").unwrap(),
        },
        Fill {
            maker: orders[1].id,
            taker: order.id,
            price: U256::from_dec_str("1000000000000000000").unwrap(),
            quantity: U256::from_dec_str("0500000000000000000").unwrap(),
        },
    ];

    assert!(do_vecs_match(&match_result.fills, &expected_fills));
}

#[tokio::test]
pub async fn test_fills_output_taker_fully_matched_single_maker() {
    /* need at least three for this test */
    let traders: Vec<Address> =
        vec![Address::random(), Address::random(), Address::random()];
    let the_far_future: DateTime<Utc> = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(1699025703, 0),
        Utc,
    );
    let market: Address = Address::zero();

    let orders: Vec<Order> = vec![/* SHORT @ $1 for 0.5 */ Order::new(
        traders[0],
        market,
        OrderSide::Ask,
        U256::from_dec_str("1000000000000000000").unwrap(),
        U256::from_dec_str("1500000000000000000").unwrap(),
        the_far_future,
        Utc::now(),
        vec![],
    )];

    let mut actual_book: Book = Book::new(market);

    for order in orders.iter() {
        actual_book.submit(order.clone()).await.unwrap();
    }

    // Long @ $1 for 1
    let order: Order = Order::new(
        traders[2],
        market,
        OrderSide::Bid,
        U256::from_dec_str("1000000000000000000").unwrap(),
        U256::from_dec_str("1000000000000000000").unwrap(),
        the_far_future,
        Utc::now(),
        vec![],
    );

    let match_result = actual_book.submit(order.clone()).await.unwrap();

    assert_eq!(match_result.order_status, OrderStatus::FullMatch);

    let expected_fills: Vec<Fill> = vec![Fill {
        maker: orders[0].id,
        taker: order.id,
        price: U256::from_dec_str("1000000000000000000").unwrap(),
        quantity: U256::from_dec_str("1000000000000000000").unwrap(),
    }];

    assert!(do_vecs_match(&match_result.fills, &expected_fills));
}

#[tokio::test]
pub async fn test_converting_book_to_external_book_and_back() {
    /* need at least three for this test */
    let traders: Vec<Address> =
        vec![Address::random(), Address::random(), Address::random()];
    let the_far_future: DateTime<Utc> = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(1699025703, 0),
        Utc,
    );

    let now_unix: DateTime<Utc> = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0),
        Utc,
    );

    let market: Address = Address::random();

    let orders: Vec<Order> = vec![
        /* SHORT @ $1.15 for 1.2 */
        Order::new(
            traders[0],
            market,
            OrderSide::Ask,
            U256::from_dec_str("1150000000000000000").unwrap(),
            U256::from_dec_str("1200000000000000000").unwrap(),
            the_far_future,
            now_unix,
            vec![25, 44],
        ),
        /* LONG @ $1.20 for 1 */
        Order::new(
            traders[1],
            market,
            OrderSide::Bid,
            U256::from_dec_str("1200000000000000000").unwrap(),
            U256::from_dec_str("1000000000000000000").unwrap(),
            the_far_future,
            now_unix,
            vec![25, 42],
        ),
        /* LONG @ $1.20 for 1 */
        Order::new(
            traders[2],
            market,
            OrderSide::Bid,
            U256::from_dec_str("1200000000000000000").unwrap(),
            U256::from_dec_str("1000000000000000000").unwrap(),
            the_far_future,
            now_unix,
            vec![35, 44],
        ),
    ];

    let mut actual_book: Book = Book::new(market);

    for order in orders.iter() {
        actual_book.submit(order.clone()).await.unwrap();
    }

    let external_book = ExternalBook::from(actual_book.clone());

    let converted_book = Book::try_from(external_book);

    assert_eq!(actual_book, converted_book.unwrap());
}
