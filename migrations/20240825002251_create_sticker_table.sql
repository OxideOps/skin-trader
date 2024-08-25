CREATE TABLE Sticker
(
    id              SERIAL PRIMARY KEY,
    sale_id         INTEGER REFERENCES Sale (id),
    skin_id         INTEGER REFERENCES Skin (id),
    image           VARCHAR(255),
    market_item_id  INTEGER REFERENCES MarketItem (id),
    slot            SMALLINT CHECK (slot >= 0),
    wear            DOUBLE PRECISION,
    suggested_price INTEGER CHECK (suggested_price >= 0),
    offset_x        DOUBLE PRECISION,
    offset_y        DOUBLE PRECISION,
    skin_status     INTEGER,
    rotation        DOUBLE PRECISION
);

CREATE INDEX idx_sticker_sale_id ON Sticker (sale_id);
CREATE INDEX idx_sticker_skin_id ON Sticker (skin_id);
CREATE INDEX idx_sticker_market_item_id ON Sticker (market_item_id);
