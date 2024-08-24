CREATE TABLE MarketData (
    id          INTEGER PRIMARY KEY,
    skin_id     INTEGER NOT NULL REFERENCES Skin(id),
    price       DOUBLE PRECISION NOT NULL,
    created_at  TIMESTAMP WITH TIME ZONE NOT NULL,
    discount    INTEGER,
    float_value DOUBLE PRECISION
);

CREATE INDEX idx_MarketData_skin_id ON MarketData(skin_id)