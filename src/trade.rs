use crate::order::{OrderId, Price, Quantity};

/// Represents a successful trade.
#[derive(Clone, Debug)]
pub struct Trade {
    // matched bid and ask
    bid_trade: TradeInfo,
    ask_trade: TradeInfo,
}

/// Information about completed trade.
#[derive(Clone, Debug)]
pub struct TradeInfo {
    pub order_id: OrderId,
    pub price: Price,
    pub quantity: Quantity,
}

impl Trade {
    pub fn new(bid_trade: TradeInfo, ask_trade: TradeInfo) -> Self {
        Self {
            bid_trade: bid_trade,
            ask_trade: ask_trade,
        }
    }
}

/// Collection of Trades.
pub type Trades = Vec<Trade>;
