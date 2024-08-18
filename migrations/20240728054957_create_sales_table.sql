CREATE TABLE Skin (
    id INTEGER PRIMARY KEY,
    name VARCHAR(255) UNIQUE NOT NULL,
    class_id VARCHAR(255) NOT NULL,
    suggested_price INTEGER
);

CREATE TABLE Sale (
    id SERIAL PRIMARY KEY,
    skin_id INTEGER NOT NULL REFERENCES Skin(id),
    created_at DATE NOT NULL,
    extras_1 INTEGER CHECK (extras_1 >= 0),
    float_value DOUBLE PRECISION,
    paint_index INTEGER CHECK (paint_index >= 0),
    paint_seed INTEGER CHECK (paint_seed >= 0),
    phase_id INTEGER CHECK (phase_id >= 0),
    price DOUBLE PRECISION NOT NULL
);

CREATE INDEX idx_sale_skin_id ON Sale(skin_id);

CREATE TABLE Sticker (
    id SERIAL PRIMARY KEY,
    sale_id INTEGER REFERENCES Sale(id),
    skin_id INTEGER REFERENCES Skin(id),
    image VARCHAR(255),
    slot SMALLINT CHECK (slot >= 0),
    wear DOUBLE PRECISION,
    suggested_price INTEGER CHECK (suggested_price >= 0),
    offset_x DOUBLE PRECISION,
    offset_y DOUBLE PRECISION,
    skin_status INTEGER,
    rotation DOUBLE PRECISION
);

CREATE INDEX idx_sticker_sale_id ON Sticker(sale_id);
CREATE INDEX idx_sticker_skin_id ON Sticker(skin_id);