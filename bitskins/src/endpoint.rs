use strum::EnumCount;
use strum_macros::{Display, EnumCount, EnumString};
use tokio::sync::Mutex;

static ENDPOINT_LOCKS: EndpointLocks = EndpointLocks::new();

pub(crate) fn get_lock_for_endpoint(endpoint: Endpoint) -> &'static Mutex<()> {
    &ENDPOINT_LOCKS.locks[endpoint as usize]
}

/// Enum for  all endpoints for Bitskins API
#[derive(EnumString, Display, EnumCount, Copy, Clone)]
#[strum(serialize_all = "snake_case")]
pub enum Endpoint {
    #[strum(serialize = "/market/delist/single")]
    DelistSingle,
    #[strum(serialize = "/market/update_price/single")]
    UpdatePriceSingle,
    #[strum(serialize = "/market/relist/single")]
    RelistSingle,
    #[strum(serialize = "/account/profile/balance")]
    ProfileBalance,
    #[strum(serialize = "/market/buy/single")]
    BuySingle,
    #[strum(serialize = "/market/pricing/list")]
    PricingList,
    #[strum(serialize = "/market/skin")]
    Skin,
    #[strum(serialize = "/market/search/get")]
    SearchGet,
    #[strum(serialize = "/market/search/730")]
    SearchCsgo,
    #[strum(serialize = "/market/history/list")]
    HistoryList,
}

/// Contains an array that holds a mutex for each possible endpoint
pub(crate) struct EndpointLocks {
    pub(crate) locks: [Mutex<()>; Endpoint::COUNT],
}

impl EndpointLocks {
    const fn new() -> Self {
        #[allow(clippy::declare_interior_mutable_const)]
        const INIT: Mutex<()> = Mutex::const_new(());
        Self {
            locks: [INIT; Endpoint::COUNT],
        }
    }
}
