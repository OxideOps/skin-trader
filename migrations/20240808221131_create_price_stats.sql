CREATE TABLE price_statistics (
    skin_id INTEGER PRIMARY KEY REFERENCES Skin(id),
    mean_price DOUBLE PRECISION,
    sale_count INTEGER NOT NULL,
    price_slope DOUBLE PRECISION
);
