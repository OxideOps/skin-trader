CREATE TABLE Sale (
    id SERIAL PRIMARY KEY,
    weapon_skin_id INTEGER NOT NULL,
    created_at DATE NOT NULL,
    extras_1 INTEGER,
    float_value DOUBLE PRECISION,
    paint_index INTEGER,
    paint_seed INTEGER,
    phase_id INTEGER,
    price DOUBLE PRECISION NOT NULL
);

CREATE INDEX idx_sale_weapon_skin_id ON Sale(weapon_skin_id);

CREATE TABLE Sticker (
    id SERIAL PRIMARY KEY,
    sale_id INTEGER REFERENCES Sale(id),
    class_id VARCHAR(255),
    skin_id INTEGER,
    image VARCHAR(255),
    name VARCHAR(255),
    slot SMALLINT,
    wear DOUBLE PRECISION,
    suggested_price INTEGER,
    offset_x DOUBLE PRECISION,
    offset_y DOUBLE PRECISION,
    skin_status INTEGER,
    rotation DOUBLE PRECISION
);

CREATE INDEX idx_sticker_sale_id ON Sticker(sale_id);