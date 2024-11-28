CREATE TABLE dmarket_stats (
    game_id TEXT NOT NULL,
    title TEXT NOT NULL,
    mean DOUBLE PRECISION,
    sale_count INTEGER,
    price_slope DOUBLE PRECISION,
    PRIMARY KEY (game_id, title)
);