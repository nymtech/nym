const { ethers } = require('hardhat');
const contracts = require('../contractAddresses.json'); 

async function main() {
    const [deployer] = await ethers.getSigners();
  
    console.log("Deploying contracts with the account:", deployer.address);
  
    // console.log("Account balance:", (await deployer.getBalance()).toString());
    const balance = await provider.getBalance(deployer.address);
    console.log(balance)
  
    // const BurnableToken = await ethers.getContractFactory("BurnableToken");
    // const burnToken = await BurnableToken.deploy("BURNTOKEN","B4A");
    // console.log(`token deployed at ${burnToken.address}`); 

    // const BurnForAccess = await ethers.getContractFactory("BurnForAccess");
    // const burn4access = await BurnForAccess.deploy(burnToken.address);
    // const burn4access = await BurnForAccess.deploy(contracts.Rinkeby.BurnableToken);
  
    // console.log(`Burn4Access contract deployed at ${burn4access.address}`); 
    // TODO automatically update contractAddresses.json 
}
  
main()
  .then(() => process.exit(0))
  .catch((error) => {
      console.error(error);
      process.exit(1);
  });
