use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Clone)]
pub struct Title {
    pub title: String,
    pub game_id: String,
}

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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Sale {
    price: String,
    date: String,
    tx_operation_type: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiscountItemResponse {
    pub reduced_fees: Vec<DiscountItem>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiscountItem {
    expires_at: i64,
    fraction: String,
    max_price: i64,
    min_price: i64,
    title: String,
}
