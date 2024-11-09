use crate::{db, http};

impl From<http::Skin> for db::Skin {
    fn from(skin: http::Skin) -> Self {
        Self {
            id: skin.id,
            name: skin.name,
            class_id: skin.class_id,
            suggested_price: skin.suggested_price,
        }
    }
}

impl From<http::MarketItem> for db::MarketItem {
    fn from(item: http::MarketItem) -> Self {
        Self {
            created_at: item.created_at,
            id: item.id.parse().unwrap(),
            skin_id: item.skin_id,
            price: item.price,
            float_value: item.float_value,
        }
    }
}

impl db::Sticker {
    pub fn from_sale(sticker: http::Sticker, sale_id: &i32) -> Self {
        Self {
            id: 0,
            sale_id: Some(*sale_id),
            skin_id: sticker.skin_id,
            image: sticker.image,
            market_item_id: None,
            slot: sticker.slot,
            wear: sticker.wear,
            suggested_price: sticker.suggested_price,
            offset_x: sticker.offset_x,
            offset_y: sticker.offset_y,
            skin_status: sticker.skin_status,
            rotation: sticker.rotation,
        }
    }

    pub fn from_market_item(sticker: http::Sticker, market_item_id: &str) -> Self {
        Self {
            id: 0,
            sale_id: None,
            skin_id: sticker.skin_id,
            image: sticker.image,
            market_item_id: market_item_id.parse().ok(),
            slot: sticker.slot,
            wear: sticker.wear,
            suggested_price: sticker.suggested_price,
            offset_x: sticker.offset_x,
            offset_y: sticker.offset_y,
            skin_status: sticker.skin_status,
            rotation: sticker.rotation,
        }
    }
}

impl db::Sale {
    pub(crate) fn new(sale: &http::Sale, skin_id: i32) -> Self {
        Self {
            id: 0,
            skin_id,
            created_at: sale.created_at.0,
            extras_1: sale.extras_1,
            float_value: sale.float_value,
            paint_index: sale.paint_index,
            paint_seed: sale.paint_seed,
            phase_id: sale.phase_id,
            price: sale.price,
        }
    }
}
