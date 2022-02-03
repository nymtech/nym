const { ethers } = require('hardhat');
const { constants } = require('@openzeppelin/test-helpers');
const contracts = require("../../contractAddresses.json"); 
const fs = require('file-system');

async function main() {
    const BandwidthGenerator = await ethers.getContractFactory("BandwidthGeneratorNoGravity");
    // if this is failing, check whether the ERC20 address has been manually added to the contract addresses json file 
    const bandwidthGenerator = await BandwidthGenerator.deploy(
        contracts.rinkeby.NYM_ERC20 
    ); 

    console.log("deploying..."); 

    contracts.rinkeby.BANDWIDTH_GENERATOR_NO_GRAVITY = bandwidthGenerator.address;
    // the location of the json file is relative to where you are running the script from - run from root of directory 
    fs.writeFileSync('./contractAddresses.json', JSON.stringify(contracts), (err) => {
        if (err) throw err;
    });
    
    console.log(`...bandwidthGenerator.sol deployed at ${bandwidthGenerator.address}`); 
}
  
main()
  .then(() => process.exit(0))
  .catch((error) => {
      console.error(error);
      process.exit(1);
  });
