CREATE TABLE items (
    id SERIAL PRIMARY KEY,
    skin_id BIGINT NOT NULL,
    price BIGINT NOT NULL,
    float_value DECIMAL(10, 8) NOT NULL,
    created_at DATE,
);

CREATE INDEX idx_items_skin_id ON items (skin_id);
