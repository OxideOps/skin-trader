CREATE TABLE MarketItem (
    id          INTEGER PRIMARY KEY,
    skin_id     INTEGER NOT NULL REFERENCES Skin(id),
    price       DOUBLE PRECISION NOT NULL,
    created_at  TIMESTAMP WITH TIME ZONE NOT NULL,
    phase_id    INTEGER,
    float_value DOUBLE PRECISION
);

CREATE INDEX idx_MarketItem_skin_id ON MarketItem(skin_id)