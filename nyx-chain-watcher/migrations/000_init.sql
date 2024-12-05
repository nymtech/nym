CREATE TABLE responses (
    id SERIAL PRIMARY KEY,
    joke_id VARCHAR NOT NULL UNIQUE,
    joke TEXT NOT NULL,
    date_created INTEGER NOT NULL
);
