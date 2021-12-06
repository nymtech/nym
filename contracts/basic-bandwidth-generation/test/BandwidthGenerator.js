const { expect } = require("chai");
const { constants, expectRevert, expectEvent } = require('@openzeppelin/test-helpers');
const { artifacts, web3 } = require("hardhat");
const BN = require('bn.js');
const BandwidthGenerator = artifacts.require('BandwidthGenerator');
const Gravity = artifacts.require('Gravity');   
const CosmosToken = artifacts.require('CosmosERC20');


contract('BandwidthGenerator', (accounts) => {
  let bandwidthGenerator; 
  let gravity; 
  let erc20token; 
  let owner = accounts[0];
  let user = accounts[1];
  let initialRatio = 1073741824; // 1073741824 bytes = 1GB
  let newRatio; 
  let tokenAmount = web3.utils.toWei('100'); // this is converting 100 tokens to their representation in wei: 100000000000000000000
  let halfTokenAmount = web3.utils.toWei('50');
  let unevenTokenAmount = web3.utils.toWei('11.9'); // 11900000000000000000
  let oneToken = web3.utils.toWei('1');

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
      18
    ); 

    // grab event args for getting token address
    const logs = await gravity.getPastEvents({
      fromBlock: 0,
      toBlock: "latest",
    });

    // create contract abstraction of deployed erc20NYM with address from event args
    erc20token = await CosmosToken.at(logs[0].args._tokenContract);
    
    // deploy bandwidthGenerator contract with contract address of erc20NYM & address of gravity bridge
    bandwidthGenerator = await BandwidthGenerator.new(erc20token.address, gravity.address); 
  });
  
  context(">> deployment parameters are valid", () => {
    it("returns the correct erc20 address", async () => {
        expect((await bandwidthGenerator.erc20()).toString()).to.equal((erc20token.address).toString());
    });
    it("returns the correct gravity address", async () => {
        expect((await bandwidthGenerator.gravityBridge()).toString()).to.equal((gravity.address).toString());
    });
    it("returns the correct initial BytesPerToken ratio", async () => {
        expect((await bandwidthGenerator.BytesPerToken()).toString()).to.equal((initialRatio).toString());
    });
    it("returns the correct contract admin", async () => {
      expect((await bandwidthGenerator.owner()).toString()).to.equal((owner).toString());
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
        BandwidthGenerator.new(erc20token.address, constants.ZERO_ADDRESS), 
        "BandwidthGenerator: gravity bridge address cannot be null"
      )
    });
  });

  context(">> generateBasicBandwidthCredential()", () => {
    before("", async () => {
      // transfer tokens to account which will create a BBCredential 
      await erc20token.mintForUnitTesting(user, tokenAmount); 
      // approve transfer to contract
      await erc20token.approve(bandwidthGenerator.address,(tokenAmount),{ from: user }); 
    });

    it("transfers tokens to bridge and emits an event with the correct values: 50 erc20NYM = 50GB of bandwidth", async () => {
      let tx = await bandwidthGenerator.generateBasicBandwidthCredential(
            halfTokenAmount,
            15,
            [0x39, 0x53, 0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba,
              0x39, 0x53, 0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba],
            constants.ZERO_BYTES32,  
            { from: user }
      );

      let expectedBandwidthInMB = ((halfTokenAmount/10**18)*initialRatio); // 50 * (1024*1024*1024) bytes = 51200MB = 50GB of bandwidth

      await expectEvent.inTransaction(tx.tx, bandwidthGenerator, 'BBCredentialPurchased', {
        Bandwidth: expectedBandwidthInMB.toString(), 
        VerificationKey: '15',
        SignedVerificationKey: '0x39530a00eae2a5aac8144209ccac917ae56bf4a9589544cb0020f92fee35a3ba39530a00eae2a5aac8144209ccac917ae56bf4a9589544cb0020f92fee35a3ba',
        CosmosRecipient: constants.ZERO_BYTES32 
      });

      await expectEvent.inTransaction(tx.tx, erc20token, 'Transfer', {
        from: user,
        to: bandwidthGenerator.address,
      });

      await expectEvent.inTransaction(tx.tx, gravity, 'SendToCosmosEvent', {
        _tokenContract: erc20token.address,
        _sender: bandwidthGenerator.address,
        _destination: constants.ZERO_BYTES32,
        _amount: halfTokenAmount
      });

      expect((await erc20token.balanceOf(bandwidthGenerator.address)).toString()).to.equal('0');
      expect((await erc20token.balanceOf(user)).toString()).to.equal(halfTokenAmount.toString());
    });

    /**
     * This can be out by a float still with amounts such as '.1' - hunt down 
     */
    it("it transfers for uneven token amounts", async () => {
      let tx = await bandwidthGenerator.generateBasicBandwidthCredential(
        unevenTokenAmount,
        15,
        [0x39, 0x53, 0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba,
          0x39, 0x53, 0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba],
        constants.ZERO_BYTES32,  
        { from: user }
      );
      
      let newexpectedBandwidthInMB = ((11900000000000000000*initialRatio)/10**18);  

      await expectEvent.inTransaction(tx.tx, bandwidthGenerator, 'BBCredentialPurchased', {
        Bandwidth: newexpectedBandwidthInMB.toString(), 
        VerificationKey: '15',
        SignedVerificationKey: '0x39530a00eae2a5aac8144209ccac917ae56bf4a9589544cb0020f92fee35a3ba39530a00eae2a5aac8144209ccac917ae56bf4a9589544cb0020f92fee35a3ba',
        CosmosRecipient: constants.ZERO_BYTES32 
      });

      await expectEvent.inTransaction(tx.tx, erc20token, 'Transfer', {
        from: user,
        to: bandwidthGenerator.address,
      });

      await expectEvent.inTransaction(tx.tx, gravity, 'SendToCosmosEvent', {
        _tokenContract: erc20token.address,
        _sender: bandwidthGenerator.address,
        _destination: constants.ZERO_BYTES32,
        _amount: unevenTokenAmount
      });
    });

    it("reverts when signed verification key !=64 bytes", async () => {
      await erc20token.approve(bandwidthGenerator.address,(halfTokenAmount),{ from: user }); 

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

    // seems to be a bug with hardhat at the moment re: expectRevert .. looking into finding a way around this
    it("reverts when cosmos address !=32 bytes", async () => {
      let badBytes = constants.ZERO_BYTES32.slice(0,-1); 
      // console.log(badBytes.length);

      await expectRevert(
        bandwidthGenerator.generateBasicBandwidthCredential(
          1,
          16,
          [0x39, 0x53, 0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba,
            0x39, 0x53, 0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba],
          badBytes,
          { from: user }
        ), "Cosmos address doesn't have 32 bytes"
      );
    });
  });

  context(">> changeRatio()", () => {
    it("only admin can change token to MB ratio", async () => {
      newRatio = 10 * initialRatio; // 10GB of bandwidth per 1 erc20NYM
      await expectRevert(
        bandwidthGenerator.changeRatio(newRatio, {from: user}), 
        "Ownable: caller is not the owner"
      );
    });
    it("admin can change ratio, emits 'RatioChanged' event", async () => {
        let tx = await bandwidthGenerator.changeRatio(newRatio, {from: owner});
        await expectEvent.inTransaction(tx.tx, bandwidthGenerator, 'RatioChanged', {
          NewBytesPerToken: newRatio.toString()
        });
        expect((await bandwidthGenerator.BytesPerToken()).toString()).to.equal((newRatio).toString());
    });
    it("BBCredential represents new ratio after change: 1 erc20NYM = 10GB of bandwidth", async () => {
      let tx = await bandwidthGenerator.generateBasicBandwidthCredential(
        oneToken,
        15,
        [0x39, 0x53, 0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba,
          0x39, 0x53, 0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba],
        constants.ZERO_BYTES32,  
        { from: user }
      );

      let expectedBandwidthInMB = ((oneToken/10**18)*newRatio); 

      await expectEvent.inTransaction(tx.tx, bandwidthGenerator, 'BBCredentialPurchased', {
        Bandwidth: expectedBandwidthInMB.toString(), 
        VerificationKey: '15',
        SignedVerificationKey: '0x39530a00eae2a5aac8144209ccac917ae56bf4a9589544cb0020f92fee35a3ba39530a00eae2a5aac8144209ccac917ae56bf4a9589544cb0020f92fee35a3ba',
        CosmosRecipient: constants.ZERO_BYTES32 
      });

      await expectEvent.inTransaction(tx.tx, erc20token, 'Transfer', {
        from: user,
        to: bandwidthGenerator.address,
      });

      await expectEvent.inTransaction(tx.tx, gravity, 'SendToCosmosEvent', {
        _tokenContract: erc20token.address,
        _sender: bandwidthGenerator.address,
        _destination: constants.ZERO_BYTES32,
        _amount: oneToken
      });       
    });
  });  

}); 

