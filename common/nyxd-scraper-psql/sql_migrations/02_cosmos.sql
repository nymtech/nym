CREATE TABLE validator (
    consensus_address TEXT NOT NULL PRIMARY KEY,
    /* Validator consensus address */
    consensus_pubkey TEXT NOT NULL UNIQUE
    /* Validator consensus public key */
);

CREATE TABLE pre_commit (
    validator_address TEXT NOT NULL REFERENCES validator (consensus_address),
    height BIGINT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    voting_power BIGINT NOT NULL,
    proposer_priority BIGINT NOT NULL,
    UNIQUE (validator_address, timestamp)
);

CREATE INDEX pre_commit_validator_address_index ON pre_commit (validator_address);

CREATE INDEX pre_commit_height_index ON pre_commit (height);

CREATE TABLE block (
    height BIGINT UNIQUE PRIMARY KEY,
    hash TEXT NOT NULL UNIQUE,
    num_txs INTEGER DEFAULT 0,
    total_gas BIGINT DEFAULT 0,
    proposer_address TEXT REFERENCES validator (consensus_address),
    timestamp TIMESTAMPTZ NOT NULL
);

CREATE INDEX block_height_index ON block (height);

CREATE INDEX block_hash_index ON block (hash);

CREATE INDEX block_proposer_address_index ON block (proposer_address);

CREATE TABLE "transaction" (
    hash TEXT UNIQUE NOT NULL,
    height BIGINT NOT NULL REFERENCES block (height),
    "index" INTEGER NOT NULL,
    success BOOLEAN NOT NULL,
    /* Body */
    num_messages INTEGER NOT NULL,
    messages JSONB NOT NULL DEFAULT '[]',
    memo TEXT,
    signatures TEXT [] NOT NULL,
    /* AuthInfo */
    signer_infos JSONB NOT NULL DEFAULT '[]'::JSONB,
    fee JSONB NOT NULL DEFAULT '{}'::JSONB,
    /* Tx response */
    gas_wanted BIGINT DEFAULT 0,
    gas_used BIGINT DEFAULT 0,
    raw_log TEXT
);

CREATE INDEX transaction_hash_index ON "transaction" (hash);

CREATE INDEX transaction_height_index ON "transaction" (height);

CREATE TABLE message (
    transaction_hash TEXT NOT NULL REFERENCES "transaction" (hash),
    "index" BIGINT NOT NULL,
    TYPE TEXT NOT NULL,
    value JSONB NOT NULL,
    involved_accounts_addresses TEXT [] NOT NULL,
    height BIGINT NOT NULL,
    CONSTRAINT unique_message_per_tx UNIQUE (transaction_hash, "index")
);

CREATE INDEX message_transaction_hash_index ON message (transaction_hash);

CREATE INDEX message_type_index ON message (TYPE);

CREATE TABLE pruning (last_pruned_height BIGINT NOT NULL);