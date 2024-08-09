CREATE TABLE price_statistics (
    weapon_skin_id INTEGER PRIMARY KEY REFERENCES Skin(id),
    mean_price DOUBLE PRECISION,
    std_dev_price DOUBLE PRECISION,
    sale_count INTEGER NOT NULL,
    min_float DOUBLE PRECISION,
    max_float DOUBLE PRECISION,
    time_correlation DOUBLE PRECISION,
    price_slope DOUBLE PRECISION,
    last_update TIMESTAMP WITH TIME ZONE NOT NULL
);