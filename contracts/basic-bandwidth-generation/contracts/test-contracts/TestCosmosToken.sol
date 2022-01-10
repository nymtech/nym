//SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.10;
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

/**
* This is a slightly modified version of the cosmos erc20 contract 
* which I have done for unit testing. 
* 
* All that has been changed is the MAX_UINT variable to allow 
* me to mint some tokens more easily in unit tests, and the 
* addition of the public mint() function. 
*/


contract TestCosmosERC20 is ERC20 {
	/* canonical amount */
	// uint256 MAX_UINT = 2**256 - 1;

	/* unit testing amount */
	uint256 HALF_MAX_UINT = 2**256 / 2;

	uint8 private cosmosDecimals;
	address private gravityAddress;

	// This override ensures we return the proper number of decimals
	// for the cosmos token
	function decimals() public view virtual override returns (uint8) {
		return cosmosDecimals;
	}

	// This is not an accurate total supply. Instead this is the total supply
	// of the given cosmos asset on Ethereum at this moment in time. Keeping
	// a totally accurate supply would require constant updates from the Cosmos
	// side, while in theory this could be piggy-backed on some existing bridge
	// operation it's a lot of complextiy to add so we chose to forgoe it.
	function totalSupply() public view virtual override returns (uint256) {
		return HALF_MAX_UINT - balanceOf(gravityAddress);
	}

	constructor(
		address _gravityAddress,
		string memory _name,
		string memory _symbol,
		uint8 _decimals
	) ERC20(_name, _symbol) {
		cosmosDecimals = _decimals;
		gravityAddress = _gravityAddress;
		_mint(_gravityAddress, HALF_MAX_UINT);
	}

	// Additional function for our (nym repo) unit tests with bridge
	function mintForUnitTesting(address _to, uint _amount) public {
		_mint(_to, _amount); 
	}
}