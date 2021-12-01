const { expect } = require("chai");
const { constants, expectRevert, expectEvent } = require('@openzeppelin/test-helpers');
const { artifacts } = require("hardhat");
const BandwidthGenerator = artifacts.require('BandwidthGenerator');
const Gravity = artifacts.require('Gravity');   
const CosmosToken = artifacts.require('CosmosERC20');


contract('BandwidthGenerator', (accounts) => {
  let bandwidthGenerator; 
  let gravity; 
  let erc20Asset;
  let erc20token; 
  let owner = accounts[0];
  let user = accounts[1];

  before('deploy contracts', async () => {
    // deploy gravity bridge with test data
    gravity = await Gravity.new(
      constants.ZERO_BYTES32, 
      1, 
      [owner], 
      [10]
    ); 

    // deploy erc20 NYM from bridge 
    await gravity.deployERC20(
      'eNYM',
      'NYMERC20',
      'NYM',
      6
    ); 

    // grab event args for getting token address
    const logs = await gravity.getPastEvents({
      fromBlock: 0,
      toBlock: "latest",
    });

    console.log(logs[0].args._tokenContract);
    erc20Asset = logs[0].args._tokenContract; 
    
    // deploy bandwidthGenerator contract with contract address of erc20 asset & address of gravity bridge
    bandwidthGenerator = await BandwidthGenerator.new(erc20Asset, gravity.address); 
  });
  
  context(">> deployment parameters are valid", () => {
    it("returns the correct erc20 address", async () => {
        expect((await bandwidthGenerator.erc20()).toString()).to.equal((erc20Asset).toString());
    });
    it("returns the correct gravity address", async () => {
        expect((await bandwidthGenerator.gravityBridge()).toString()).to.equal((gravity.address).toString());
    });
  });

  context(">> deployment parameters are invalid", () => {
    it("cannot be deployed with invalid erc20 address (zero address)", async () => {
        expectRevert(
          BandwidthGenerator.new(constants.ZERO_ADDRESS, gravity.address), 
          "BandwidthGenerator: erc20 address cannot be null"
        )
    });
    it("cannot be deployed with invalid gravity bridge address (zero address)", async () => {
      expectRevert(
        BandwidthGenerator.new(erc20Asset, constants.ZERO_ADDRESS), 
        "BandwidthGenerator: gravity bridge address cannot be null"
      )
    });
  });

  context(">> generateBasicBandwidthCredential()", () => {
    before("", async () => {
      // create contract abstraction of erc20 asset 
      erc20token = await CosmosToken.at(erc20Asset);
      // transfer tokens to account which will create a BBBC 
      await erc20token.mintForUnitTesting(user, 90); 
      // approve transfer to contract
      await erc20token.approve(bandwidthGenerator.address,45,{ from: user }); 
    });

    it("transfers tokens to self and emits an event with the correct values", async () => {
      let tx = await bandwidthGenerator.generateBasicBandwidthCredential(
            45,
            15,
            [0x39, 0x53, 0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba,
              0x39, 0x53, 0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba],
            constants.ZERO_BYTES32,  
            { from: user }
      );

      await expectEvent.inTransaction(tx.tx, bandwidthGenerator, 'BBCredentialPurchased', {
        Bandwidth: ((1024 * 1024 * 1024) * 45).toString(),
        VerificationKey: (15).toString(),
        SignedVerificationKey: '0x39530a00eae2a5aac8144209ccac917ae56bf4a9589544cb0020f92fee35a3ba39530a00eae2a5aac8144209ccac917ae56bf4a9589544cb0020f92fee35a3ba',
        CosmosRecipient: constants.ZERO_BYTES32 
      });

      expect((await erc20token.balanceOf(bandwidthGenerator.address)).toString()).to.equal('0');
      expect((await erc20token.balanceOf(user)).toString()).to.equal('45');
    });

    it("reverts when signed verification key !=64 bytes", async () => {
      await erc20token.approve(bandwidthGenerator.address,45,{ from: user }); 

      await expectRevert(
        bandwidthGenerator.generateBasicBandwidthCredential(
          1,
          16,
          [0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba,
            0x39, 0x53, 0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba],
          constants.ZERO_BYTES32,
          { from: user }
        ), "BandwidthGenerator: Signature doesn't have 64 bytes"
      );
    });

  });

}); 

