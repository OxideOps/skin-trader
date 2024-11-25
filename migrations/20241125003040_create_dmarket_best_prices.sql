CREATE TABLE dmarket_best_prices (
    market_hash_name TEXT PRIMARY KEY,
    offers_best_price TEXT NOT NULL,
    offers_best_count INTEGER NOT NULL,
    orders_best_price TEXT NOT NULL,
    orders_best_count INTEGER NOT NULL
)