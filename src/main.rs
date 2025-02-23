/*****************************************************
 * Simple Orderbook from Tomer Tz
 * Roughly interpreted in Rust by Mohandeep Kapur
 *
 * Still a WIP
 *****************************************************/
use orderbook::{error::BookResult, order::*, orderbook::*};
use std::{cell::RefCell, rc::Rc};

fn main() -> BookResult<()> {
    let mut orderbook: OrderBook = OrderBook::new("XYZ");
    Ok(())
}
