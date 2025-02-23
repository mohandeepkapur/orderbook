use crate::order::{OrderId, Quantity, Side};
use thiserror::Error;

/// Error enum for OrderBook.
#[derive(Error, Debug)]
pub enum OrderBookError {
    #[error("Order {0} not found in book...")]
    OrderNotFound(OrderId),
    #[error("Order {0} already exists in book...")]
    OrderAlreadyExists(OrderId),
    #[error("Book's side is empty...")]
    BookSideEmpty(Side),
    #[error("...")]
    InternalOrderProcessingError(String),
}

/// Error enum for an Order.
#[derive(Error, Debug, PartialEq)]
pub enum OrderError {
    #[error("Tried to overfill Order by {surplus} qty...")]
    RequestedFillTooLarge { surplus: Quantity },
}

impl From<OrderError> for OrderBookError {
    fn from(value: OrderError) -> Self {
        match value {
            OrderError::RequestedFillTooLarge { surplus: quantity } => {
                Self::InternalOrderProcessingError(format!(
                    "Book tried to overfill an order by {}",
                    quantity
                ))
            }
        }
    }
}

pub type BookResult<T> = std::result::Result<T, OrderBookError>;
pub type OrdResult<T> = std::result::Result<T, OrderError>;
