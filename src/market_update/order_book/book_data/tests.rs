use super::planning::{
    OrderBookFetchPlan, order_book_needs_precision_refresh,
    order_book_response_matches_expected_precision, plan_order_book_fetch,
};
use super::*;
use crate::market_state::{OrderBookInstance, OrderBookSymbolMode};

mod availability;
mod planning;
mod precision_refresh;
mod response_precision;

fn required_plan(plan: Option<OrderBookFetchPlan>, reason: &str) -> OrderBookFetchPlan {
    match plan {
        Some(plan) => plan,
        None => panic!("{reason}"),
    }
}
