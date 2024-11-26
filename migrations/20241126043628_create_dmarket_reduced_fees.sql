CREATE TABLE dmarket_reduced_fees (
    game_id TEXT NOT NULL,
    title TEXT NOT NULL,
    expires_at BIGINT NOT NULL,
    fraction TEXT NOT NULL,
    max_price BIGINT NOT NULL,
    min_price BIGINT NOT NULL,
    PRIMARY KEY (game_id, title)
)

