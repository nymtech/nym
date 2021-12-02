pragma solidity 0.6.6;

import "./CosmosToken.sol";
import "./Gravity.sol";

/** 
 * @title BandwidthGenerator
 * @dev   Contract for generating basic bandwidth credentials on the Nym cosmos blockchain, 
 *        using ERC20 representations of NYM as payment. 
 * 
 *        Utilises the Gravity Bridge for cross-chain payment. 
 */ 
contract BandwidthGenerator {
    
    CosmosERC20 public erc20;
    Gravity     public gravityBridge; 

    event BBCredentialPurchased(
        uint256 Bandwidth,
        uint256 indexed VerificationKey,
        bytes   SignedVerificationKey, 
        bytes32 CosmosRecipient
    );
    
    constructor(CosmosERC20 _erc20, Gravity _gravityBridge) public {
        require(address(_erc20) != address(0),         "BandwidthGenerator: erc20 address cannot be null"); 
        require(address(_gravityBridge) != address(0), "BandwidthGenerator: gravity bridge address cannot be null"); 
        erc20 = _erc20;
        gravityBridge = _gravityBridge; 
    }

    function bandwidthFromToken(uint256 amount) private pure returns (uint256) {
        // 1 token represents 1GB 
        return amount * 1024 * 1024 * 1024;
    }
    
    function generateBasicBandwidthCredential(uint256 amount, uint256 verificationKey, bytes memory signedVerificationKey, bytes32 cosmosRecipient) public {
        require(signedVerificationKey.length == 64, "BandwidthGenerator: Signature doesn't have 64 bytes");
        erc20.transferFrom(msg.sender, address(this), amount);
        erc20.approve(address(gravityBridge), amount);
        gravityBridge.sendToCosmos(
		    address(erc20),
		    cosmosRecipient,    
		    amount
	    );
        uint256 bandwidth = bandwidthFromToken(amount);
        emit BBCredentialPurchased(
            bandwidth, 
            verificationKey, 
            signedVerificationKey,
            cosmosRecipient
        );
    }
}
 

