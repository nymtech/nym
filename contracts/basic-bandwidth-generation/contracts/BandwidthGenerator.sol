pragma solidity 0.6.6;

import "./CosmosToken.sol";
import "./Gravity.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/** 
 * @title BandwidthGenerator
 * @dev   Contract for generating Basic Bandwidth Credentials (BBCs) on the Nym cosmos blockchain, 
 *        using ERC20 representations of NYM as payment. Utilises the Gravity Bridge for cross-chain payment. 
 *        
 *        Credentials represent a certain amount of bandwidth which can be sent through the Nym Mixnet. 
 *        By default 1 NYM = 1 GB of bandwidth. The `MBPerToken` amount can be adjusted by the contract owner. 
 *        The final amount of bandwidth is calculated according to the following formula: 
 *        `(Token amount in 'wei' / 10**18) * MBPerToken`
 */ 
contract BandwidthGenerator is Ownable {
    
    CosmosERC20 public erc20;
    Gravity     public gravityBridge; 
    uint256     public MBPerToken; 
    
    event BBCredentialPurchased(
        uint256 indexed Bandwidth,
        uint256 indexed VerificationKey,
        bytes   SignedVerificationKey, 
        bytes32 indexed CosmosRecipient
    );

    event RatioChanged(
        uint256 indexed NewMBPerToken
    );
    
    constructor(CosmosERC20 _erc20, Gravity _gravityBridge) public {
        require(address(_erc20) != address(0),         "BandwidthGenerator: erc20 address cannot be null"); 
        require(address(_gravityBridge) != address(0), "BandwidthGenerator: gravity bridge address cannot be null"); 
        erc20 = _erc20;
        gravityBridge = _gravityBridge; 
        MBPerToken = 1024; // default amount set at deployment 
    }

    function changeRatio(uint256 _newMBPerTokenAmount) public onlyOwner { 
        require(_newMBPerTokenAmount != 0, "BandwidthGenerator: price cannot be 0"); 
        MBPerToken = _newMBPerTokenAmount;  
        emit RatioChanged(_newMBPerTokenAmount);
    }
    
    function generateBasicBandwidthCredential(uint256 _amount, uint256 _verificationKey, bytes memory _signedVerificationKey, bytes32 _cosmosRecipient) public {
        require(_signedVerificationKey.length == 64, "BandwidthGenerator: Signature doesn't have 64 bytes");
        require(_cosmosRecipient.length == 32,       "BandwidthGenerator: Cosmos address doesn't have 32 bytes");
        erc20.transferFrom(msg.sender, address(this), _amount);
        erc20.approve(address(gravityBridge), _amount); 
        gravityBridge.sendToCosmos(
		    address(erc20),
		    _cosmosRecipient,    
		    _amount
	    );
        uint256 bandwidth = bandwidthFromToken(_amount);
        emit BBCredentialPurchased(
            bandwidth, 
            _verificationKey, 
            _signedVerificationKey,
            _cosmosRecipient
        );
    }

    function bandwidthFromToken(uint256 _amount) public view returns (uint256) {
        return (_amount/10**18) * MBPerToken;
    }

}
 

