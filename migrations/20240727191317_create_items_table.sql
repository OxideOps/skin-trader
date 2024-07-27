CREATE TABLE items (
    id SERIAL PRIMARY KEY,
    created_at DATE,
    float_value DECIMAL(10, 8) NOT NULL,
    price BIGINT NOT NULL,
    skin_id BIGINT NOT NULL
);

CREATE INDEX idx_items_skin_id ON items (skin_id);