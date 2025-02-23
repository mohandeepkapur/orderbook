/*****************************************************
 * Simple Orderbook from Tomer Tz
 * Roughly interpreted in Rust by Mohandeep Kapur
 *
 * Still a WIP
 *****************************************************/
use orderbook::{error::BookResult, order::*, orderbook::*};
use std::{cell::RefCell, rc::Rc};

fn main() -> BookResult<()> {
    let mut orderbook: OrderBook = OrderBook::new("AAPL");

    let id1: OrderId = 1;

    let _ = orderbook.add_order(Rc::new(RefCell::new(Order::new(
        OrderType::GoodTillCancel,
        id1,
        Side::Buy,
        100,
        10,
    ))));

    // println!("{:?}", orderbook.get_order_infos());

    // orderbook.cancel_order(id1);

    // println!("{:?}", orderbook.get_order_infos());

    let result = orderbook.add_order(Rc::new(RefCell::new(Order::new(
        OrderType::GoodTillCancel,
        id1,
        Side::Buy,
        120,
        10,
    ))));

    println!("{:?}", result);

    let trade = orderbook.add_order(Rc::new(RefCell::new(Order::new(
        OrderType::GoodTillCancel,
        2 as OrderId,
        Side::Sell,
        100,
        10,
    ))));

    println!("{:?}", trade);

    Ok(())
}
