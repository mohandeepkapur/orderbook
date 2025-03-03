use std::{cell::RefCell, rc::Rc};

use ::orderbook::orderbook::OrderBook;
use orderbook::{error::BookResult, order::*, trade::*};

// integration tests here

#[test]
fn match_two_good_till_cancels() -> BookResult<()> {
    let mut book: OrderBook = OrderBook::new("QQQ");

    let bid_price: Price = 10000;
    let ask_price: Price = 12000;

    let bid = Order::new(
        OrderType::GoodTillCancel,
        101212 as OrderId,
        Side::Buy,
        bid_price,
        100 as Quantity,
    ).to_order_ref();

    let ask = Order::new(
        OrderType::GoodTillCancel,
        111 as OrderId,
        Side::Sell,
        ask_price,
        100 as Quantity,
    ).to_order_ref();

    book.add_order(bid)?;
    let trade = book.add_order(ask)?;
    println!("{:?}", trade);
    assert!(trade.is_none());
    Ok(())
}