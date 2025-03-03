use crate::error::{
    BookResult, OrdResult,
    OrderBookError::*,
    OrderError::{self, *},
};

use linked_hash_map::LinkedHashMap;
use std::{cell::RefCell, fmt::Display, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OrderType {
    // grab whatever is immediately available and get out
    FillAndKill,
    // typically cleared after 30 - 60 days
    GoodTillCancel,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Side {
    Buy,
    Sell,
}

/// Bid or Ask price for an Order. Unit is cents.
pub type Price = i32;
pub type Quantity = u32;
pub type OrderId = i64;

/// Represents an order sent to an Exchange.
#[derive(Debug, Clone, PartialEq)]
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
    side: Option<Side>,
    price: Option<Price>,
    quantity: Option<Quantity>,
}

impl OrderModify {
    pub fn new(
        order_id: OrderId,
        side: Option<Side>,
        price: Option<Price>,
        quantity: Option<Quantity>,
    ) -> Self {
        Self {
            order_id,
            side,
            price,
            quantity,
        }
    }

    pub fn get_order_id(&self) -> &OrderId {
        &self.order_id
    }
    pub fn get_side(&self) -> &Option<Side> {
        &self.side
    }
    pub fn get_price(&self) -> &Option<Price> {
        &self.price
    }
    pub fn get_quantity(&self) -> &Option<Quantity> {
        &self.quantity
    }

    pub fn to_order(&self, order_to_modify: Order) -> OrdResult<Order> {
        if order_to_modify.get_order_id() != self.get_order_id() {
            return Err(ModificationError(format!(
                "Provided order {} doesn't have desired order id {} ...",
                order_to_modify.get_order_id(),
                self.get_order_id()
            )));
        }

        let new_side = match self.get_side() {
            Some(side) => *side,
            None => *order_to_modify.get_side(),
        };

        let new_price = match self.get_price() {
            Some(price) => *price,
            None => *order_to_modify.get_price(),
        };

        let new_quantity = match self.get_quantity() {
            Some(quantity) => *quantity,
            None => *order_to_modify.get_initial_quantity(),
        };

        Ok(Order::new(
            *order_to_modify.get_order_type(),
            self.order_id,
            new_side,
            new_price,
            new_quantity,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Order

    #[test]
    fn test_fill_order() -> OrdResult<()> {
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

        assert_eq!(
            order.fill(130),
            Err(OrderError::RequestedFillTooLarge { surplus: 30 })
        );
    }

    // OrderModify

    #[test]
    fn test_modify_order() -> OrdResult<()> {
        let order_id: OrderId = 101212;

        let order_to_modify = Order::new(
            OrderType::GoodTillCancel,
            order_id,
            Side::Sell,
            30 as Price,
            100 as Quantity,
        );

        let mod_details_1 = OrderModify::new(order_id, None, Some(44), None);

        let order = mod_details_1.to_order(order_to_modify)?;

        assert_eq!(
            order,
            Order::new(
                OrderType::GoodTillCancel,
                order_id,
                Side::Sell,
                44 as Price,
                100 as Quantity
            )
        );

        let mod_details_2 = OrderModify::new(101212 as OrderId, Some(Side::Buy), None, Some(400));

        let order = mod_details_2.to_order(order)?;

        assert_eq!(
            order,
            Order::new(
                OrderType::GoodTillCancel,
                order_id,
                Side::Buy,
                44 as Price,
                400 as Quantity
            )
        );

        Ok(())
    }

    #[test]
    fn test_modify_order_mismatched_ids() {
        let order_to_modify = Order::new(
            OrderType::GoodTillCancel,
            101212 as OrderId,
            Side::Sell,
            30 as Price,
            100 as Quantity,
        );

        let mod_details_1 = OrderModify::new(10, None, Some(44), None);

        let order = mod_details_1.to_order(order_to_modify);

        assert!(matches!(order, Err(OrderError::ModificationError(_))));
    }
}
