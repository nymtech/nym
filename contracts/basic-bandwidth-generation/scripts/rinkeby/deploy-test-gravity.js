const { ethers } = require('hardhat');
const { constants } = require('@openzeppelin/test-helpers');
const contracts = require("../../contractAddresses.json"); 
const fs = require('file-system');

async function main() {
    const [deployer] = await ethers.getSigners();
    console.log(deployer.address); 
    console.log(constants.ZERO_BYTES32); 
    const Gravity = await ethers.getContractFactory("TestGravity");
    // deploy with args from unit tests 
    const gravity = await Gravity.deploy(
        constants.ZERO_BYTES32, 
        [deployer.address], 
        [2863311531]
    );

    contracts.rinkeby.TEST_GRAVITY = gravity.address;
    // the location of the json file is relative to where you are running the script from - run from root of directory 
    fs.writeFileSync('./contractAddresses.json', JSON.stringify(contracts), (err) => {
        if (err) throw err;
    });
    
    console.log(`gravity.sol deployed at ${gravity.address}`); 
}
  
main()
  .then(() => process.exit(0))
  .catch((error) => {
      console.error(error);
      process.exit(1);
  });
