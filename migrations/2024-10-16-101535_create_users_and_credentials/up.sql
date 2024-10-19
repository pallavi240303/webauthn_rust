-- Your SQL goes here
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    unique_id UUID NOT NULL,
    username VARCHAR NOT NULL UNIQUE
);

CREATE TABLE passkeys (
    id SERIAL PRIMARY KEY,
    user_id UUID REFERENCES users(unique_id) ON DELETE CASCADE,
    passkey_data BYTEA NOT NULL
);
