// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.

export type Gateway = { host: string, mix_port: number, clients_port: number, location: string, sphinx_key: string, 
/**
 * Base58 encoded ed25519 EdDSA public key of the gateway used to derive shared keys with clients
 */
identity_key: string, version: string, };
