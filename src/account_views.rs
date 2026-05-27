mod balances;
mod history;
mod history_tables;
mod income;
mod orders;
mod picker;
mod portfolio;
mod positions;
mod style;
mod summary;
pub(in crate::account_views) mod table_helpers;
mod tabs;

use crate::helpers::invalid_data_placeholder;

#[cfg(test)]
mod tests;

pub(in crate::account_views) fn invalid_account_data() -> String {
    invalid_data_placeholder()
}
