CREATE TABLE dmarket_account (
    id BOOLEAN PRIMARY KEY CHECK (id),
    balance INTEGER NOT NULL
);

INSERT INTO dmarket_account (id, balance) VALUES (TRUE, 0.0);
