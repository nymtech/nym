// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use async_trait::async_trait;
use cosmwasm_std::Addr;
use nym_contracts_common::signing::{
    ContractMessageContent, MessageSignature, Nonce, SignableMessage, SigningAlgorithm,
};
use nym_crypto::asymmetric::identity;
use nym_mixnet_contract_common::{
    construct_legacy_mixnode_bonding_sign_payload, Gateway, GatewayBondingPayload, MixNode,
    NodeCostParams, NymNode, NymNodeBondingPayload, SignableGatewayBondingMsg,
    SignableLegacyMixNodeBondingMsg, SignableNymNodeBondingMsg,
};
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::Coin;
use nym_validator_client::DirectSigningHttpRpcValidatorClient;

// define this as a separate trait for mocking purposes
#[async_trait]
pub(crate) trait AddressAndNonceProvider {
    async fn get_signing_nonce(&self) -> Result<Nonce, NyxdError>;
    fn cw_address(&self) -> Addr;
}

#[async_trait]
impl AddressAndNonceProvider for DirectSigningHttpRpcValidatorClient {
    async fn get_signing_nonce(&self) -> Result<Nonce, NyxdError> {
        self.nyxd.get_signing_nonce(&self.nyxd.address()).await
    }

    fn cw_address(&self) -> Addr {
        self.nyxd.cw_address()
    }
}

// since the message has to go back to the user due to the increasing nonce, we might as well sign the entire payload
pub(crate) async fn create_mixnode_bonding_sign_payload<P: AddressAndNonceProvider>(
    client: &P,
    mix_node: MixNode,
    cost_params: NodeCostParams,
    pledge: Coin,
    vesting: bool,
) -> Result<SignableLegacyMixNodeBondingMsg, BackendError> {
    if vesting {
        return Err(BackendError::UnsupportedVestingOperation);
    }
    let sender = client.cw_address();
    let nonce = client.get_signing_nonce().await?;

    Ok(construct_legacy_mixnode_bonding_sign_payload(
        nonce,
        sender,
        pledge.into(),
        mix_node,
        cost_params,
    ))
}

pub(crate) async fn verify_mixnode_bonding_sign_payload<P: AddressAndNonceProvider>(
    client: &P,
    mix_node: &MixNode,
    cost_params: &NodeCostParams,
    pledge: &Coin,
    vesting: bool,
    msg_signature: &MessageSignature,
) -> Result<(), BackendError> {
    if vesting {
        return Err(BackendError::UnsupportedVestingOperation);
    }
    let identity_key = identity::PublicKey::from_base58_string(&mix_node.identity_key)?;
    let signature = identity::Signature::from_bytes(msg_signature.as_ref())?;

    // recreate the plaintext
    let msg = create_mixnode_bonding_sign_payload(
        client,
        mix_node.clone(),
        cost_params.clone(),
        pledge.clone(),
        vesting,
    )
    .await?;
    let plaintext = msg.to_plaintext()?;

    if !msg.algorithm.is_ed25519() {
        return Err(BackendError::UnexpectedSigningAlgorithm {
            received: msg.algorithm,
            expected: SigningAlgorithm::Ed25519,
        });
    }

    // TODO: possibly provide better error message if this check fails
    identity_key.verify(plaintext, &signature)?;
    Ok(())
}

// since the message has to go back to the user due to the increasing nonce, we might as well sign the entire payload
pub(crate) async fn create_gateway_bonding_sign_payload<P: AddressAndNonceProvider>(
    client: &P,
    gateway: Gateway,
    pledge: Coin,
    vesting: bool,
) -> Result<SignableGatewayBondingMsg, BackendError> {
    if vesting {
        return Err(BackendError::UnsupportedVestingOperation);
    }
    let payload = GatewayBondingPayload::new(gateway);
    let sender = client.cw_address();
    let content = ContractMessageContent::new(sender, vec![pledge.into()], payload);
    let nonce = client.get_signing_nonce().await?;

    Ok(SignableMessage::new(nonce, content))
}

pub(crate) async fn verify_gateway_bonding_sign_payload<P: AddressAndNonceProvider>(
    client: &P,
    gateway: &Gateway,
    pledge: &Coin,
    vesting: bool,
    msg_signature: &MessageSignature,
) -> Result<(), BackendError> {
    if vesting {
        return Err(BackendError::UnsupportedVestingOperation);
    }
    let identity_key = identity::PublicKey::from_base58_string(&gateway.identity_key)?;
    let signature = identity::Signature::from_bytes(msg_signature.as_ref())?;

    // recreate the plaintext
    let msg = create_gateway_bonding_sign_payload(client, gateway.clone(), pledge.clone(), vesting)
        .await?;
    let plaintext = msg.to_plaintext()?;

    if !msg.algorithm.is_ed25519() {
        return Err(BackendError::UnexpectedSigningAlgorithm {
            received: msg.algorithm,
            expected: SigningAlgorithm::Ed25519,
        });
    }

    // TODO: possibly provide better error message if this check fails
    identity_key.verify(plaintext, &signature)?;
    Ok(())
}

pub(crate) async fn create_nym_node_bonding_sign_payload<P: AddressAndNonceProvider>(
    client: &P,
    nym_node: NymNode,
    cost_params: NodeCostParams,
    pledge: Coin,
) -> Result<SignableNymNodeBondingMsg, BackendError> {
    let payload = NymNodeBondingPayload::new(nym_node, cost_params);
    let sender = client.cw_address();
    let content = ContractMessageContent::new(sender, vec![pledge.into()], payload);
    let nonce = client.get_signing_nonce().await?;

    Ok(SignableMessage::new(nonce, content))
}

pub(crate) async fn verify_nym_node_bonding_sign_payload<P: AddressAndNonceProvider>(
    client: &P,
    nym_node: &NymNode,
    cost_params: &NodeCostParams,
    pledge: &Coin,
    msg_signature: &MessageSignature,
) -> Result<(), BackendError> {
    let identity_key = identity::PublicKey::from_base58_string(&nym_node.identity_key)?;
    let signature = identity::Signature::from_bytes(msg_signature.as_ref())?;

    // recreate the plaintext
    let msg = create_nym_node_bonding_sign_payload(
        client,
        nym_node.clone(),
        cost_params.clone(),
        pledge.clone(),
    )
    .await?;
    let plaintext = msg.to_plaintext()?;

    if !msg.algorithm.is_ed25519() {
        return Err(BackendError::UnexpectedSigningAlgorithm {
            received: msg.algorithm,
            expected: SigningAlgorithm::Ed25519,
        });
    }

    // TODO: possibly provide better error message if this check fails
    identity_key.verify(plaintext, &signature)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::coin;
    use nym_contracts_common::Percent;
    use nym_mixnet_contract_common::NodeCostParams;
    use rand_chacha::rand_core::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    // use rng with constant seed for all tests so that they would be deterministic
    pub fn test_rng() -> ChaCha20Rng {
        let dummy_seed = [42u8; 32];
        rand_chacha::ChaCha20Rng::from_seed(dummy_seed)
    }

    struct MockClient {
        address: Addr,
        signing_nonce: Nonce,
    }

    #[async_trait]
    impl AddressAndNonceProvider for MockClient {
        async fn get_signing_nonce(&self) -> Result<Nonce, NyxdError> {
            Ok(self.signing_nonce)
        }

        fn cw_address(&self) -> Addr {
            self.address.clone()
        }
    }

    #[tokio::test]
    async fn dummy_mix_bonding_signature_verification() {
        let mut rng = test_rng();
        let identity_keypair = identity::KeyPair::new(&mut rng);
        let dummy_mixnode = MixNode {
            host: "1.2.3.4".to_string(),
            mix_port: 1234,
            verloc_port: 2345,
            http_api_port: 3456,
            sphinx_key: "totally-legit-sphinx-key".to_string(),
            identity_key: identity_keypair.public_key().to_base58_string(),
            version: "v1.2.3".to_string(),
        };
        let dummy_cost_params = NodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(42).unwrap(),
            interval_operating_cost: coin(1111111, "unym"),
        };
        let dummy_pledge: Coin = coin(10000000000, "unym").into();

        let dummy_account = Addr::unchecked("n16t2umcd83zjpl5puyuuq6lgmy4p3qedjd8ynn6");
        let dummy_client = MockClient {
            address: dummy_account,
            signing_nonce: 42,
        };

        let signing_msg_liquid = create_mixnode_bonding_sign_payload(
            &dummy_client,
            dummy_mixnode.clone(),
            dummy_cost_params.clone(),
            dummy_pledge.clone(),
            false,
        )
        .await
        .unwrap();

        let plaintext_liquid = signing_msg_liquid.to_plaintext().unwrap();
        let sig_liquid: MessageSignature = identity_keypair
            .private_key()
            .sign(&plaintext_liquid)
            .to_bytes()
            .as_ref()
            .into();

        let signing_msg_vesting = create_mixnode_bonding_sign_payload(
            &dummy_client,
            dummy_mixnode.clone(),
            dummy_cost_params.clone(),
            dummy_pledge.clone(),
            true,
        )
        .await;
        assert!(signing_msg_vesting.is_err());

        let res = verify_mixnode_bonding_sign_payload(
            &dummy_client,
            &dummy_mixnode,
            &dummy_cost_params,
            &dummy_pledge,
            false,
            &sig_liquid,
        )
        .await;
        assert!(res.is_ok());

        let res = verify_mixnode_bonding_sign_payload(
            &dummy_client,
            &dummy_mixnode,
            &dummy_cost_params,
            &dummy_pledge,
            true,
            &sig_liquid,
        )
        .await;
        assert!(res.is_err())
    }

    #[tokio::test]
    async fn dummy_gateway_bonding_signature_verification() {
        let mut rng = test_rng();
        let identity_keypair = identity::KeyPair::new(&mut rng);
        let dummy_gateway = Gateway {
            host: "1.2.3.4".to_string(),
            mix_port: 1234,
            clients_port: 2345,
            location: "whatever".to_string(),
            sphinx_key: "totally-legit-sphinx-key".to_string(),
            identity_key: identity_keypair.public_key().to_base58_string(),
            version: "v1.2.3".to_string(),
        };

        let dummy_pledge: Coin = coin(10000000000, "unym").into();

        let dummy_account = Addr::unchecked("n16t2umcd83zjpl5puyuuq6lgmy4p3qedjd8ynn6");
        let dummy_client = MockClient {
            address: dummy_account,
            signing_nonce: 42,
        };

        let signing_msg_liquid = create_gateway_bonding_sign_payload(
            &dummy_client,
            dummy_gateway.clone(),
            dummy_pledge.clone(),
            false,
        )
        .await
        .unwrap();

        let plaintext_liquid = signing_msg_liquid.to_plaintext().unwrap();
        let sig_liquid: MessageSignature = identity_keypair
            .private_key()
            .sign(&plaintext_liquid)
            .to_bytes()
            .as_ref()
            .into();

        let signing_msg_vesting = create_gateway_bonding_sign_payload(
            &dummy_client,
            dummy_gateway.clone(),
            dummy_pledge.clone(),
            true,
        )
        .await;
        assert!(signing_msg_vesting.is_err());

        let res = verify_gateway_bonding_sign_payload(
            &dummy_client,
            &dummy_gateway,
            &dummy_pledge,
            false,
            &sig_liquid,
        )
        .await;
        assert!(res.is_ok());

        let res = verify_gateway_bonding_sign_payload(
            &dummy_client,
            &dummy_gateway,
            &dummy_pledge,
            true,
            &sig_liquid,
        )
        .await;
        assert!(res.is_err())
    }
}
