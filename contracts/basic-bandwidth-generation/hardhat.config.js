require("@nomiclabs/hardhat-etherscan");
require("@nomiclabs/hardhat-truffle5");
require("@nomiclabs/hardhat-web3");
require("@nomiclabs/hardhat-ethers");
require('dotenv').config({ path: require('find-config')('.env') });

/**
 * @type import('hardhat/config').HardhatUserConfig
 */
module.exports = {
  solidity: {
    version: "0.8.10",
    settings: {
      optimizer: {
        enabled: true
      }
    }  },
  // paths: {
  //   artifacts: "./artifacts/contracts"
  // },
  networks: {
   rinkeby: {
      url: process.env.RINKEBY_URL, //Infura url with projectId
      accounts: [process.env.PRIV_KEY], // private key of account used for contract interaction
      gas: "auto", 
      gasPrice: "auto"
    },
  }, 
  etherscan: {
    // Your API key for Etherscan
    // Obtain one at https://etherscan.io/
    apiKey: process.env.ETHERSCAN_API_KEY
  }
};  
