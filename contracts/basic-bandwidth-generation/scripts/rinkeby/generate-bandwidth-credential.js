const { ethers } = require('hardhat');
const { constants } = require('@openzeppelin/test-helpers');
const contracts = require("../../contractAddresses.json"); 
const fs = require('file-system');

async function main() {

    const BandwidthGenerator = await ethers.getContractFactory("BandwidthGenerator");
    const bandwidthGenerator = await BandwidthGenerator.attach(contracts.rinkeby.BANDWIDTH_GENERATOR); 
    console.log(`grabbed BANDWIDTH contract at ${bandwidthGenerator.address}`); 

    const Token = await ethers.getContractFactory("CosmosERC20"); 
    const erc20nym = await Token.attach(contracts.rinkeby.NYM_ERC20); 
    console.log(`grabbed ERC20 NYM contract at ${erc20nym.address}`); 

    let check = await bandwidthGenerator.owner(); 
    console.log(`owner is ${check}`); 

    console.log(`approving 1 NYM for transfer to BANDWIDTH`); 
    await erc20nym.approve(bandwidthGenerator.address, 1000000); 
    console.log(`approved successfully`); 
    console.log('...'); 

    console.log(`generating bbc`); 
    await bandwidthGenerator.generateBasicBandwidthCredential(
        1000000,
        "8513226889913878556430235353454720565401242598665291259741189869506713110335", 
        "0x045184e31380e0201c65c96a9a2947007f919c814c612898d062943d6dce06d4401cd319471d63dd65920ea0698310a832feeb9fbfc98b7bddefdcdfc1f8c60e",
        "nymt134er36g8l5h36u3y793jcxxvvaqncgqvemww2l"
    ); 
    console.log(`success! check etherscan for emitted events: https://rinkeby.etherscan.io/address/${bandwidthGenerator.address}`); 

}
  
main()
  .then(() => process.exit(0))
  .catch((error) => {
      console.error(error);
      process.exit(1);
  });
