use strum_macros::{Display, EnumString};
/// Enum for  all endpoints for Bitskins API
#[derive(EnumString, Display, Copy, Clone, Debug)]
pub enum Endpoint {
    #[strum(serialize = "/market/delist/single")]
    DelistSingle,
    #[strum(serialize = "/market/update_price/single")]
    UpdatePriceSingle,
    #[strum(serialize = "/market/relist/single")]
    RelistSingle,
    #[strum(serialize = "/market/relist/many")]
    RelistMany,
    #[strum(serialize = "/account/profile/balance")]
    ProfileBalance,
    #[strum(serialize = "/market/buy/single")]
    BuySingle,
    #[strum(serialize = "/market/pricing/list")]
    PricingList,
    #[strum(serialize = "/market/skin/730")]
    Skin,
    #[strum(serialize = "/market/search/get")]
    SearchGet,
    #[strum(serialize = "/market/search/730")]
    SearchCsgo,
    #[strum(serialize = "/market/history/list")]
    HistoryList,
    #[strum(serialize = "/market/search/mine/730")]
    Inventory,
    #[strum(serialize = "/market/update_price/many")]
    UpdateOfferPrices,
    #[strum(serialize = "/wallet/transaction/list")]
    Transactions,
}
