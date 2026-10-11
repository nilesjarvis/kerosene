mod api;
pub(crate) mod approval;
mod model;

pub(crate) use api::{
    cached_order_approval, check_order_approval, fetch_builder_approval, invalidate_order_approval,
};

pub(crate) use model::{BuilderCodeConfig, BuilderFee, normalize_builder_address};

#[cfg(test)]
mod tests;
