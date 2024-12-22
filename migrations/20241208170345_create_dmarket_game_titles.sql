CREATE TABLE dmarket_game_titles (
    game_id TEXT NOT NULL,
    title TEXT NOT NULL,
    mean_price DOUBLE PRECISION,
    sale_count INTEGER,
    monthly_sales INTEGER,
    PRIMARY KEY (game_id, title)
)
