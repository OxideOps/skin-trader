#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(FromRow, Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub game_id: String,
    pub item_id: Uuid,
    pub title: String,
    pub amount: i64,
    pub created_at: i64,
    pub discount: i64,
    #[sqlx(flatten)]
    pub extra: Extra,
    pub status: ItemStatus,
    #[sqlx(flatten)]
    pub price: Option<Price>,
    #[sqlx(flatten)]
    pub instant_price: Option<Price>,
    #[sqlx(flatten)]
    pub suggested_price: Option<Price>,
    pub r#type: ItemType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    Default,
    Active,
    Inactive,
    InTransfer,
    Sold,
    Recalled,
    Unavailable,
    Locked,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ItemType {
    Item,
    Offer,
    Target,
    Class,
    Airdrop,
    Sale,
    Product,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Extra {
    pub category: Option<String>,
    pub float_value: Option<f64>,
    pub is_new: bool,
    pub tradable: bool,
    pub offer_id: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Price {
    #[serde(rename = "USD")]
    pub usd: String,
}

#[derive(Deserialize, Debug)]
pub struct ItemResponse {
    pub cursor: Option<String>,
    pub objects: Vec<Item>,
    pub total: Total,
}

#[derive(Deserialize, Debug)]
pub struct Total {
    pub closed_targets: Option<usize>,
    pub completed_offers: Option<usize>,
    pub items: usize,
    pub offers: usize,
    pub targets: usize,
}

#[derive(Deserialize, Debug)]
pub struct SaleResponse {
    pub sales: Vec<Sale>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sale {
    pub price: String,
    pub date: String,
    pub tx_operation_type: String,

    // DB-only fields, skipped during deserialization
    #[serde(skip)]
    pub id: i64,
    #[serde(skip)]
    pub game_title: GameTitle,
}

impl Sale {
    pub fn with_game_title(mut self, game_title: &GameTitle) -> Self {
        self.game_title = game_title.clone();
        self
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListFeeResponse {
    pub default_fee: ListDefaultFee,
    pub reduced_fees: Vec<ListPersonalFee>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListDefaultFee {
    pub fraction: String,
    pub min_amount: i64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListPersonalFee {
    pub expires_at: i64,
    pub fraction: String,
    pub max_price: i64,
    pub min_price: i64,
    pub title: String,
}

#[derive(Debug, Hash, Eq, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct GameTitle {
    pub game_id: String,
    pub title: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
    pub usd: String,
    pub usd_available_to_withdraw: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct BestPricesResponse {
    pub error: Option<String>,
    pub total: String,
    pub aggregated_titles: Vec<BestPrices>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct BestPrices {
    pub market_hash_name: String,
    pub offers: BestPrice,
    pub orders: BestPrice,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct BestPrice {
    pub best_price: String,
    pub count: i32,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub offer_id: String,
    pub price: OfferMoney,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OfferMoney {
    pub amount: String,
    pub currency: String,
}
