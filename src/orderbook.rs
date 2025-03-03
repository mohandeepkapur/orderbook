use std::{
    cmp::min,
    collections::{BTreeMap, HashMap},
    mem,
};

use crate::{
    error::BookResult, error::OrderBookError::*, error::OrderBookError::*, order::*, trade::*,
};

use linked_hash_map::LinkedHashMap;

#[derive(Clone, Debug)]
pub struct LevelInfo {
    price: Price,
    quantity: Quantity,
}

pub type LevelInfos = Vec<LevelInfo>;

#[derive(Debug)]
pub struct OrderBookLevelInfos {
    bids: LevelInfos,
    asks: LevelInfos,
}

impl OrderBookLevelInfos {
    pub fn new(bids: &LevelInfos, asks: &LevelInfos) -> Self {
        Self {
            bids: bids.clone(),
            asks: asks.clone(),
        }
    }
    pub fn get_bids(&self) -> &LevelInfos {
        &self.bids
    }
    pub fn get_asks(&self) -> &LevelInfos {
        &self.asks
    }
}

/// Keeps track of Order's location in book.
struct OrderEntry {
    book_side: Side,
    price: Price,
    order_id: OrderId,
}

/// An Orderbook ordered according to price time priority.
pub struct OrderBook {
    asset: &'static str,
    bid_side: BTreeMap<Price, OrderRefs>,
    ask_side: BTreeMap<Price, OrderRefs>,
    track_orders: HashMap<OrderId, OrderEntry>,
}

impl OrderBook {
    pub fn new(asset: &'static str) -> Self {
        Self {
            asset,
            bid_side: BTreeMap::new(),
            ask_side: BTreeMap::new(),
            track_orders: HashMap::new(),
        }
    }

    /// Adds an Order to the OrderBook and provides resulting Trades.
    ///
    /// # Errors:
    /// - Returns [`OrderAlreadyExists`](crate::error::OrderBookError)
    /// - Returns [`InternalOrderProcessingError`](crate::error::OrderBookError)
    pub fn add_order(&mut self, order: OrderRef) -> BookResult<Option<Trades>> {
        let order_ref = order.lock().unwrap();

        // check if order to add id exists in book
        let order_id = order_ref.get_order_id();
        if self.track_orders.contains_key(order_id) {
            return Err(OrderAlreadyExists(*order_id));
        }

        // track order to add
        self.track_orders.insert(
            *order_ref.get_order_id(),
            OrderEntry {
                book_side: *order_ref.get_side(),
                price: *order_ref.get_price(),
                order_id: *order_ref.get_order_id(),
            },
        );

        // reject the order if FaK and no liquidity available for it given current state of the book
        if let OrderType::FillAndKill = order_ref.get_order_type() {
            if !self.can_match(order_ref.get_side(), order_ref.get_price()) {
                return Ok(None);
            }
        }

        // determine which side the order will be added to
        let book_side = match order_ref.get_side() {
            Side::Buy => &mut self.bid_side,
            Side::Sell => &mut self.ask_side,
        };

        // check if the price level to add the order to exists and act accordingly
        if let Some(orders) = book_side.get_mut(order_ref.get_price()) {
            orders.insert(*order_ref.get_order_id(), order.clone());
        } else {
            let mut orders: OrderRefs = LinkedHashMap::new();
            orders.insert(*order_ref.get_order_id(), order.clone());
            book_side.insert(*order_ref.get_price(), orders);
        }

        // taking the Rc reference out of scope
        mem::drop(order_ref);

        // return trades!
        Ok(self.match_orders()?)
    }

    /// Remove an order from the book immediately.
    ///
    /// # Errors:
    /// - Returns [`OrderNotFound`](crate::error::OrderBookError)
    pub fn cancel_order(&mut self, order_id: OrderId) -> BookResult<OrderId> {
        // confirms order is in book
        let order_entry = self
            .track_orders
            .get(&order_id)
            .ok_or(OrderNotFound(order_id))?;

        let book_side = match order_entry.book_side {
            Side::Buy => &mut self.bid_side,
            Side::Sell => &mut self.ask_side,
        };

        let orders = book_side
            .get_mut(&order_entry.price)
            .ok_or(OrderNotFound(order_id))?;
        orders.remove(&order_entry.order_id);

        orders
            .is_empty()
            .then(|| book_side.remove(&order_entry.price));

        self.track_orders.remove(&order_id);

        Ok(order_id)
    }

    /// Modify order in book.
    ///
    /// # Errors:
    /// - Returns [`OrderNotFound`](crate::error::OrderBookError)
    pub fn modify_order(&mut self, order: OrderModify) -> BookResult<Option<Trades>> {
        let order_id = order.get_order_id();

        // confirms whether order exists
        let old_order = self
            .get_order_ref(order_id)?
            .lock()
            .unwrap()
            // can't move Order out of Ref<'_, Order>, must clone
            // to adhere to new OrderModify API
            .clone();

        // ^ with curr impl, 2 clones needed to modify an Order ***

        self.cancel_order(*order_id)?;

        self.add_order(order.to_order(old_order)?.to_order_ref())
    }

    pub fn get_order_infos(&self) -> OrderBookLevelInfos {
        // grab price, quantity
        // for every price level, sum up all order quantities
        let bids: LevelInfos = self
            .bid_side
            .iter() // price level
            .map(|(price, bids)| {
                let quantity: Quantity = bids
                    .iter()
                    .map(|(_, order)| *order.lock().unwrap().get_remaining_quantity())
                    .sum();
                return LevelInfo {
                    price: *price,
                    quantity,
                };
            })
            .collect();

        let asks: LevelInfos = self
            .ask_side
            .iter() // price level
            .map(|(price, asks)| {
                let quantity: Quantity = asks
                    .iter()
                    .map(|(_, order)| *order.lock().unwrap().get_remaining_quantity())
                    .sum();
                return LevelInfo {
                    price: *price,
                    quantity,
                };
            })
            .collect();

        OrderBookLevelInfos { bids, asks }
    }

    /// Checks whether order can be matched given book's current state.
    fn can_match(&self, side: &Side, price: &Price) -> bool {
        match side {
            Side::Buy => {
                self.ask_side
                    .iter()
                    .next() // None if asks empty
                    .map_or(false, |(best_ask_price, _)| price >= best_ask_price)
            }
            Side::Sell => self
                .bid_side
                .iter()
                .next()
                .map_or(false, |(best_bid_price, _)| price <= best_bid_price),
        }
    }

    /// Match bids and asks.
    /// Returns None if no matches are currently possible.
    ///
    /// # Errors:
    /// - Returns [`OrderNotFound`](crate::error::OrderBookError)
    fn match_orders(&mut self) -> BookResult<Option<Trades>> {
        let mut trades: Vec<Trade> = vec![];
        trades.reserve(self.track_orders.len());

        // loops as long as there are orders to match
        loop {
            // if either bids or asks empty, no matches possible
            if self.ask_side.is_empty() || self.bid_side.is_empty() {
                break;
            }

            let (best_bid_price, mut bids) = match self.bid_side.pop_last() {
                Some((b_b_p, bids)) => (b_b_p, bids),
                None => break,
            };

            // self.ask_side.pop_first().map_or(break, |(bap, asks)| (bap, asks));
            let (best_ask_price, mut asks) = match self.ask_side.pop_first() {
                Some((b_a_p, asks)) => (b_a_p, asks),
                None => break,
            };

            if best_bid_price < best_ask_price {
                // no matches possible
                break;
            }

            // match best bids with best asks
            while bids.len() != 0 && asks.len() != 0 {
                let mut bid = match bids.front() {
                    Some((_, bid)) => bid.lock().unwrap(),
                    None => break, // unreachable
                };

                let mut ask = match asks.front() {
                    Some((_, ask)) => ask.lock().unwrap(),
                    None => break, // unreachable
                };

                let fill_quantity =
                    min(*bid.get_remaining_quantity(), *ask.get_remaining_quantity());

                bid.fill(fill_quantity)?;
                ask.fill(fill_quantity)?;

                let trade = Trade::new(
                    TradeInfo {
                        order_id: *bid.get_order_id(),
                        price: *bid.get_price(),
                        quantity: fill_quantity,
                    },
                    TradeInfo {
                        order_id: *ask.get_order_id(),
                        price: *ask.get_price(),
                        quantity: fill_quantity,
                    },
                );

                println!("{:?}", trade);

                trades.push(trade);

                if bid.is_filled() {
                    mem::drop(bid);
                    bids.pop_front();
                }

                if ask.is_filled() {
                    mem::drop(ask);
                    asks.pop_front();
                }
            }

            if bids.is_empty() {
                self.bid_side.remove(&best_bid_price);
            } else {
                self.bid_side.insert(best_bid_price, bids);
            }

            if asks.is_empty() {
                self.ask_side.remove(&best_ask_price);
            } else {
                self.ask_side.insert(best_ask_price, asks);
            }
        }

        if !self.bid_side.is_empty() {
            // ok for below to fail
            let _ = self.prune_fak_from_order_book(Side::Buy);
        }

        if !self.ask_side.is_empty() {
            let _ = self.prune_fak_from_order_book(Side::Sell);
        }

        match trades.is_empty() {
            true => Ok(None),
            false => Ok(Some(trades)),
        }
    }

    fn prune_fak_from_order_book(&mut self, side: Side) -> BookResult<()> {
        let orders = match side {
            Side::Buy => self
                .bid_side
                .iter()
                .next()
                // need to create new err type, or make order_id optional?
                .map_or(Err(BookSideEmpty(side)), |(_, orders)| Ok(orders)),
            Side::Sell => self
                .ask_side
                .iter()
                .rev()
                .next()
                .map_or(Err(BookSideEmpty(side)), |(_, orders)| Ok(orders)),
        }?;

        let fak_order_id: Option<OrderId> = {
            let order = orders
                .back()
                .map_or(Err(BookSideEmpty(side)), |(_, order)| Ok(order))?
                .lock()
                .unwrap();
            match order.get_order_type() {
                OrderType::FillAndKill => Some(*order.get_order_id()),
                _ => None,
            }
        };

        if let Some(order_id) = fak_order_id {
            self.cancel_order(order_id)?;
        }

        Ok(())
    }

    /// Get shared reference to an order within book given its id.
    ///
    /// # Errors:
    /// - Returns [`OrderNotFound`](crate::error::OrderBookError)
    fn get_order_ref(&self, order_id: &OrderId) -> BookResult<&OrderRef> {
        let order_entry = self
            .track_orders
            .get(&order_id)
            .ok_or(OrderNotFound(*order_id))?;

        let book_side = match order_entry.book_side {
            Side::Buy => &self.bid_side,
            Side::Sell => &self.ask_side,
        };

        let order = book_side
            .get(&order_entry.price)
            .ok_or(OrderNotFound(*order_id))?
            .get(&order_id)
            .ok_or(OrderNotFound(*order_id))?;

        Ok(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_order() -> BookResult<()> {
        todo!()
    }

    #[test]
    fn test_add_duplicate_order_id() -> BookResult<()> {
        todo!()
    }

    #[test]
    fn test_cancel_order_non_existent_id() -> BookResult<()> {
        todo!()
    }

    #[test]
    fn test_cancel_order() -> BookResult<()> {
        todo!()
    }

    #[test]
    fn test_modify_order() -> BookResult<()> {
        todo!()
    }

    #[test]
    fn test_modify_non_existent_order() -> BookResult<()> {
        todo!()
    }

    #[test]
    fn test_order_infos_book_empty_state() -> BookResult<()> {
        todo!()
    }

    #[test]
    fn test_order_infos_book_non_empty_state() -> BookResult<()> {
        todo!()
    }
}
