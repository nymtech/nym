// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.10;

import "./CosmosToken.sol";
import "./Gravity.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/math/SafeMath.sol";

/** 
 * @title BandwidthGenerator
 * @dev   Contract for generating Basic Bandwidth Credentials (BBCs) on the Nym cosmos blockchain, 
 *        using ERC20 representations of NYM as payment. Utilises the Gravity Bridge for cross-chain payment. 
 * 
 *        Credential generation can be switched on/off by the contract owner.
 *        
 *        Credentials represent a certain amount of bandwidth which can be sent through the Nym Mixnet. 
 *        By default 1 NYM = 1 GB of bandwidth. The `BytesPerToken` amount can be adjusted by the contract owner. 
 *        The amount of bandwidth bought is calculated according to the following formula: 
 *        `(Token amount in 'wei' / 10**6) * BytesPerToken`
 */ 
contract BandwidthGenerator is Ownable {

    using SafeMath for uint256; 

    CosmosERC20 public erc20;
    Gravity     public gravityBridge; 
    uint256     public BytesPerToken; 
    bool        public credentialGenerationEnabled;
    
    event BBCredentialPurchased(
        uint256 Bandwidth,
        uint256 indexed VerificationKey,
        bytes   SignedVerificationKey, 
        string  CosmosRecipient
    );

    event RatioChanged(
        uint256 indexed NewBytesPerToken
    );

    event CredentialGenerationSwitch(
        bool Enabled
    ); 

    modifier checkEnabled() {
        require(credentialGenerationEnabled, "BandwidthGenerator: credential generation isn't currently enabled");
        _;
    }

    /**
     * @param _erc20          Address of the erc20NYM deployed through the Gravity Bridge.
     * @param _gravityBridge  Address of the deployed Gravity Bridge. 
     */
    constructor(CosmosERC20 _erc20, Gravity _gravityBridge) {
        require(address(_erc20) != address(0),         "BandwidthGenerator: erc20 address cannot be null"); 
        require(address(_gravityBridge) != address(0), "BandwidthGenerator: gravity bridge address cannot be null"); 
        erc20 = _erc20;
        gravityBridge = _gravityBridge; 
        BytesPerToken = 1073741824; // default amount set at deployment: 1 erc20NYM = 1073741824 Bytes = 1GB
        credentialGenerationEnabled = true;
    }

    /**
     * @dev                            Changes amount of Bytes each erc20NYM is tradable for. Can only be called by Owner. 
     * @param _newBytesPerTokenAmount  Amount of Bytes BBC is worth per 1 erc20NYM token.
     */    
    function changeRatio(uint256 _newBytesPerTokenAmount) public onlyOwner { 
        require(_newBytesPerTokenAmount != 0, "BandwidthGenerator: price cannot be 0"); 
        BytesPerToken = _newBytesPerTokenAmount;  
        emit RatioChanged(_newBytesPerTokenAmount);
    }

    /**
     * @dev                            Switches credential generation on/off. Can only be called by Owner. 
     * @param _generation              Whether credential generation is turned on/off. 
     */  
    function credentialGenerationSwitch(bool _generation) public onlyOwner {
        credentialGenerationEnabled = _generation; 
        emit CredentialGenerationSwitch(_generation); 
    }
    
    /**
     * @dev                           Function to create a BBC for account owning the verification key on the Nym Cosmos Blockchain
     *                                by transfering erc20NYM via the Gravity Bridge. 
     * @param _amount                 Amount of erc20NYM tokens to spend on BBC - denominated in wei. 
     * @param _verificationKey        Verification key of account on Nym blockchain who is purchasing BBC.
     * @param _signedVerificationKey  Number of erc20NYMs to spend signed by _verificationKey for auth on Cosmos Blockchain.
     * @param _cosmosRecipient        Address of the recipient of payment on Nym Cosmos Blockchain.
     */    
    function generateBasicBandwidthCredential(uint256 _amount, uint256 _verificationKey, bytes memory _signedVerificationKey, string calldata _cosmosRecipient) public checkEnabled {
        require(_signedVerificationKey.length == 64, "BandwidthGenerator: Signature doesn't have 64 bytes");
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
        uint256 amountMulBytes = _amount.mul(BytesPerToken);
        return amountMulBytes.div(10**6); 
    }

}
 

