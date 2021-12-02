pragma solidity ^0.6.6;
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

/**
* This is a slightly modified version of the cosmos erc20 contract 
* which I have done for unit testing. 
* 
* All that has been changed is the MAX_UINT variable to allow 
* me to mint some tokens more easily in unit tests, and the 
* addition of the public mint() function. 
 */

contract CosmosERC20 is ERC20 {
	/* canonical amount */
	// uint256 MAX_UINT = 2**256 - 1;

	/* unit testing amount */
	uint256 HALF_MAX_UINT = 2**256 / 2;

	constructor(
		address _gravityAddress,
		string memory _name,
		string memory _symbol,
		uint8 _decimals
	) public ERC20(_name, _symbol) {
		_setupDecimals(_decimals);
		_mint(_gravityAddress, HALF_MAX_UINT);
	}

	function mintForUnitTesting(address _to, uint _amount) public {
		_mint(_to, _amount); 
	}
}
