use std::{cell::RefCell, rc::Rc};

use ::orderbook::orderbook::OrderBook;
use orderbook::{error::BookResult, order::*, trade::*};

// integration tests here

#[test]
fn match_two_good_till_cancels() -> BookResult<()> {
    let mut book: OrderBook = OrderBook::new("QQQ");

    let bid_price: Price = 10000;
    let ask_price: Price = 12000;

    let bid = produce_order_ref(Order::new(
        OrderType::GoodTillCancel,
        101212 as OrderId,
        Side::Buy,
        bid_price,
        100 as Quantity,
    ));

    let ask = produce_order_ref(Order::new(
        OrderType::GoodTillCancel,
        101212 as OrderId,
        Side::Sell,
        ask_price,
        100 as Quantity,
    ));

    book.add_order(bid)?;
    let trade = book.add_order(ask)?;
    println!("{:?}", trade);
    assert!(trade.is_none());
    Ok(())
}

fn produce_order_ref(order: Order) -> OrderRef {
    Rc::new(RefCell::new(order))
}
