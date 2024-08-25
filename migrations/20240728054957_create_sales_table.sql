CREATE TABLE Skin
(
    id              INTEGER PRIMARY KEY,
    name            VARCHAR(255) UNIQUE NOT NULL,
    class_id        VARCHAR(255)        NOT NULL,
    suggested_price INTEGER
);

CREATE TABLE Sale
(
    id          SERIAL PRIMARY KEY,
    skin_id     INTEGER                     NOT NULL REFERENCES Skin (id),
    created_at  TIMESTAMP WITH TIME ZONE    NOT NULL,
    extras_1    INTEGER CHECK (extras_1 >= 0),
    float_value DOUBLE PRECISION,
    paint_index INTEGER CHECK (paint_index >= 0),
    paint_seed  INTEGER CHECK (paint_seed >= 0),
    phase_id    INTEGER CHECK (phase_id >= 0),
    price       DOUBLE PRECISION NOT NULL
);

CREATE INDEX idx_sale_skin_id ON Sale (skin_id);
