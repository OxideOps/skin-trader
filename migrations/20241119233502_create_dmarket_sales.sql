CREATE TABLE dmarket_sales (
    id BIGSERIAL PRIMARY KEY,
    game_id TEXT NOT NULL,
    title TEXT NOT NULL,
    price TEXT NOT NULL,
    date TEXT NOT NULL,
    tx_operation_type TEXT NOT NULL
);

CREATE INDEX idx_dmarket_sales_game_title ON dmarket_sales(game_id, title);
CREATE INDEX idx_dmarket_sales_date ON dmarket_sales(date);