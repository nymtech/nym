// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.

/**
 * Information provided by the node operator during bonding that are used to allow other entities to use the services of this node.
 */
export type NymNode = { 
/**
 * Network address of this nym-node, for example 1.1.1.1 or foo.mixnode.com
 * that is used to discover other capabilities of this node.
 */
host: string, 
/**
 * Allow specifying custom port for accessing the http, and thus self-described, api
 * of this node for the capabilities discovery.
 */
custom_http_port: number | null, 
/**
 * Base58-encoded ed25519 EdDSA public key.
 */
identity_key: string, };
