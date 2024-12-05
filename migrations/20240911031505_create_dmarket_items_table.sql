CREATE TABLE dmarket_items (
    game_id TEXT NOT NULL,
    item_id UUID NOT NULL,
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
    type TEXT NOT NULL,
    offer_id UUID,
    PRIMARY KEY (game_id, item_id)
);

-- Create other useful indexes
CREATE INDEX idx_dmarket_items_status ON dmarket_items(status);
CREATE INDEX idx_dmarket_items_type ON dmarket_items(type);
CREATE INDEX idx_dmarket_items_game_id ON dmarket_items(game_id);