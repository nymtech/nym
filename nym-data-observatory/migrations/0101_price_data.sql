CREATE TABLE price_history (
    timestamp bigint PRIMARY KEY,
    chf double precision NOT NULL,
    usd double precision NOT NULL,
    eur double precision NOT NULL,
    btc double precision NOT NULL,
    gbp double precision NOT NULL
);