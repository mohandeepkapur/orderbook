use crate::error::{
    BookResult, OrdResult,
    OrderBookError::*,
    OrderError::{self, *},
};

use linked_hash_map::LinkedHashMap;
use std::{cell::RefCell, fmt::Display, rc::Rc};

#[derive(Clone, Copy, Debug)]
pub enum OrderType {
    // grab whatever is immediately available and get out
    FillAndKill,
    // typically cleared after 30 - 60 days
    GoodTillCancel,
}


#[derive(Clone, Copy, Debug)]
pub enum Side {
    Buy,
    Sell,
}

/// Bid or Ask price for an Order. Unit is cents.
pub type Price = i32;
pub type Quantity = u32;
pub type OrderId = i64;

/// Represents an order sent to an Exchange.
#[derive(Debug)]
pub struct Order {
    order_type: OrderType,
    order_id: OrderId,
    side: Side,
    price: Price,
    initial_quantity: Quantity,
    remaining_quantity: Quantity,
}

impl Order {
    pub fn new(
        order_type: OrderType,
        order_id: OrderId,
        side: Side,
        price: Price,
        quantity: Quantity,
    ) -> Self {
        Self {
            order_type,
            order_id,
            side,
            price,
            initial_quantity: quantity,
            remaining_quantity: quantity,
        }
    }

    pub fn get_order_type(&self) -> &OrderType {
        &self.order_type
    }
    pub fn get_order_id(&self) -> &OrderId {
        &self.order_id
    }
    pub fn get_side(&self) -> &Side {
        &self.side
    }
    pub fn get_price(&self) -> &Price {
        &self.price
    }
    pub fn get_initial_quantity(&self) -> &Quantity {
        &self.initial_quantity
    }
    pub fn get_remaining_quantity(&self) -> &Quantity {
        &self.remaining_quantity
    }
    pub fn get_filled_quantity(&self) -> Quantity {
        self.initial_quantity - self.remaining_quantity
    }
    pub fn is_filled(&self) -> bool {
        self.remaining_quantity == 0
    }

    /// Fills the order.
    ///
    /// # Errors:
    /// Returns [`RequestedFillTooLarge`](crate::error::OrderError) if fill quantity exceeds remaining quantity.
    pub fn fill(&mut self, quantity: Quantity) -> OrdResult<()> {
        if quantity > *self.get_remaining_quantity() {
            return Err(RequestedFillTooLarge {
                surplus: quantity - *self.get_remaining_quantity(),
            });
        }
        self.remaining_quantity -= quantity;
        Ok(())
    }

    pub fn to_order_ref(self) -> OrderRef {
        Rc::new(RefCell::new(self))
    }
}

pub type OrderRef = Rc<RefCell<Order>>;

pub type OrderRefs = LinkedHashMap<OrderId, OrderRef>;

/// Holds modification details for an order.
pub struct OrderModify {
    order_id: OrderId,
    side: Side,
    price: Price,
    quantity: Quantity,
}

impl OrderModify {
    pub fn get_order_id(&self) -> &OrderId {
        &self.order_id
    }
    pub fn get_side(&self) -> &Side {
        &self.side
    }
    pub fn get_price(&self) -> &Price {
        &self.price
    }
    pub fn get_quantity(&self) -> &Quantity {
        &self.quantity
    }

    pub fn to_order_ref(&self, order_type: OrderType) -> OrderRef {
        Rc::new(RefCell::new(Order::new(
            // all values copied over
            order_type,
            self.order_id,
            self.side,
            self.price,
            self.quantity,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Order

    #[test]
    fn test_fill_order() -> OrdResult<()>{
        let mut order = Order::new(
            OrderType::GoodTillCancel,
            101212 as OrderId,
            Side::Sell,
            30 as Price,
            100 as Quantity,
        );

        order.fill(32)?;
        assert_eq!(*order.get_remaining_quantity(), 68 as Quantity);

        let mut order = Order::new(
            OrderType::GoodTillCancel,
            101212 as OrderId,
            Side::Sell,
            30 as Price,
            100 as Quantity,
        );

        order.fill(100)?;
        assert_eq!(*order.get_remaining_quantity(), 0 as Quantity);
        Ok(())
    }

    #[test]
    fn test_over_fill_order() {
        let mut order = Order::new(
            OrderType::GoodTillCancel,
            101212 as OrderId,
            Side::Sell,
            30 as Price,
            100 as Quantity,
        );

        assert_eq!(order.fill(130), Err(OrderError::RequestedFillTooLarge { surplus: 30 }));
    }

    // OrderModify
}
