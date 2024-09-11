CREATE TABLE dmarket_items (
    item_id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    amount BIGINT NOT NULL,
    created_at BIGINT NOT NULL,
    discount BIGINT NOT NULL,
    category TEXT,
    float_value DOUBLE PRECISION,
    is_new BOOLEAN NOT NULL,
    tradable BOOLEAN NOT NULL,
    status TEXT NOT NULL,
    price_usd TEXT,
    instant_price_usd TEXT,
    suggested_price_usd TEXT,
    type TEXT NOT NULL
);

CREATE INDEX idx_dmarket_items_status ON dmarket_items(status);
CREATE INDEX idx_dmarket_items_type ON dmarket_items(type);