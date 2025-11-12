CREATE TABLE wasm_execute_contract
(
    sender               TEXT      NOT NULL,
    contract_address     TEXT      NOT NULL,
    message_type         TEXT      NULL,
    raw_contract_message JSONB     NOT NULL DEFAULT '{}'::JSONB,
    funds                COIN[]    NOT NULL DEFAULT '{}',
    fee                  COIN[]    NOT NULL DEFAULT '{}',
    executed_at          TIMESTAMP NOT NULL,
    height               BIGINT    NOT NULL,
    hash                 TEXT      NOT NULL,
    message_index        BIGINT    NOT NULL,
    memo                 TEXT      NULL
);
CREATE INDEX execute_contract_height_index ON wasm_execute_contract (height);
CREATE INDEX execute_contract_executed_at_index ON wasm_execute_contract (executed_at);
CREATE INDEX execute_contract_message_type_index ON wasm_execute_contract (message_type);
CREATE INDEX execute_contract_sender ON wasm_execute_contract (sender);
