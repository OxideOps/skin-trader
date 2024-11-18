CREATE TABLE Account (
    id BOOLEAN PRIMARY KEY CHECK (id),
    balance DOUBLE PRECISION NOT NULL
);

INSERT INTO Account (id, balance) VALUES (TRUE, 0.0);
