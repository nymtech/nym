const { expect } = require("chai");
const { constants, expectRevert, expectEvent } = require('@openzeppelin/test-helpers');
const { artifacts, web3 } = require("hardhat");
const BN = require('bn.js');
const BandwidthGenerator = artifacts.require('BandwidthGenerator');
const Gravity = artifacts.require('test-contracts/TestGravity');   
const CosmosToken = artifacts.require('TestCosmosERC20');


contract('BandwidthGenerator', (accounts) => {
  let bandwidthGenerator; 
  let gravity; 
  let erc20token; 
  let owner = accounts[0];
  let user = accounts[1];
  let cosmosRecipient = "nymt1f06hzmwf9chqewkpv93ajk6tayzp4784m2da9x"; // random sandbox testnet address
  let initialRatio = 1073741824; // 1073741824 bytes = 1GB
  let newRatio; 
  let tokenAmount = 100000000; 
  let halfTokenAmount = 50000000;
  let unevenTokenAmount = 11500000; 
  let oneToken = 1000000;

  before('deploy contracts', async () => {

    // deploy gravity bridge with test data
    gravity = await Gravity.new(
      constants.ZERO_BYTES32, 
      [owner], 
      [2863311531]
    ); 

    // deploy erc20 NYM from bridge 
    await gravity.deployERC20(
      'cosmosNYMDenomination',
      'NYMERC20',
      'NYM',
      6
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
    it("returns the correct default generation state: true", async () => {
      expect((await bandwidthGenerator.credentialGenerationEnabled())).to.equal(true); 
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
    before("mint tokens & approve", async () => {
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
            cosmosRecipient,  
            { from: user }
      );

      let expectedBandwidthInMB = ((halfTokenAmount/10**6)*initialRatio); // 50 * (1024*1024*1024) bytes = 51200MB = 50GB of bandwidth

      await expectEvent.inTransaction(tx.tx, bandwidthGenerator, 'BBCredentialPurchased', {
        Bandwidth: expectedBandwidthInMB.toString(), 
        VerificationKey: '15',
        SignedVerificationKey: '0x39530a00eae2a5aac8144209ccac917ae56bf4a9589544cb0020f92fee35a3ba39530a00eae2a5aac8144209ccac917ae56bf4a9589544cb0020f92fee35a3ba',
        CosmosRecipient: cosmosRecipient 
      });

      await expectEvent.inTransaction(tx.tx, erc20token, 'Transfer', {
        from: user,
        to: bandwidthGenerator.address,
      });

      await expectEvent.inTransaction(tx.tx, gravity, 'SendToCosmosEvent', {
        _tokenContract: erc20token.address,
        _sender: bandwidthGenerator.address,
        _destination: cosmosRecipient,
        _amount: halfTokenAmount.toString()
      });

      expect((await erc20token.balanceOf(bandwidthGenerator.address)).toString()).to.equal('0');
      expect((await erc20token.balanceOf(user)).toString()).to.equal(halfTokenAmount.toString());
    });

    /**
     * This can be out by a float still with amounts such as '.1'
     */
    it("it transfers for uneven token amounts", async () => {
      let tx = await bandwidthGenerator.generateBasicBandwidthCredential(
        unevenTokenAmount,
        15,
        [0x39, 0x53, 0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba,
          0x39, 0x53, 0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba],
        cosmosRecipient,  
        { from: user }
      );
      
      let newexpectedBandwidthInMB = ((11500000*initialRatio)/10**6);  

      await expectEvent.inTransaction(tx.tx, bandwidthGenerator, 'BBCredentialPurchased', {
        Bandwidth: newexpectedBandwidthInMB.toString(), 
        VerificationKey: '15',
        SignedVerificationKey: '0x39530a00eae2a5aac8144209ccac917ae56bf4a9589544cb0020f92fee35a3ba39530a00eae2a5aac8144209ccac917ae56bf4a9589544cb0020f92fee35a3ba',
        CosmosRecipient: cosmosRecipient 
      });

      await expectEvent.inTransaction(tx.tx, erc20token, 'Transfer', {
        from: user,
        to: bandwidthGenerator.address,
      });

      await expectEvent.inTransaction(tx.tx, gravity, 'SendToCosmosEvent', {
        _tokenContract: erc20token.address,
        _sender: bandwidthGenerator.address,
        _destination: cosmosRecipient,
        _amount: unevenTokenAmount.toString()
      });
    });

    it.skip("reverts when signed verification key !=64 bytes", async () => {
      await erc20token.approve(bandwidthGenerator.address,(halfTokenAmount),{ from: user }); 

      await expectRevert(
        bandwidthGenerator.generateBasicBandwidthCredential(
          1,
          16,
          [0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba,
            0x39, 0x53, 0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba],
          cosmosRecipient,
          { from: user }
        ), "BandwidthGenerator: Signature doesn't have 64 bytes"
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
        cosmosRecipient,  
        { from: user }
      );

      let expectedBandwidthInMB = ((oneToken/10**6)*newRatio); 

      await expectEvent.inTransaction(tx.tx, bandwidthGenerator, 'BBCredentialPurchased', {
        Bandwidth: expectedBandwidthInMB.toString(), 
        VerificationKey: '15',
        SignedVerificationKey: '0x39530a00eae2a5aac8144209ccac917ae56bf4a9589544cb0020f92fee35a3ba39530a00eae2a5aac8144209ccac917ae56bf4a9589544cb0020f92fee35a3ba',
        CosmosRecipient: cosmosRecipient
      });

      await expectEvent.inTransaction(tx.tx, erc20token, 'Transfer', {
        from: user,
        to: bandwidthGenerator.address,
      });

      await expectEvent.inTransaction(tx.tx, gravity, 'SendToCosmosEvent', {
        _tokenContract: erc20token.address,
        _sender: bandwidthGenerator.address,
        _destination: cosmosRecipient,
        _amount: oneToken
      });       
    });
  });  
  context(">>credential generation admin switch", () => {
    it("only admin can switch credential generation off", async () => {
      await expectRevert(
        bandwidthGenerator.credentialGenerationSwitch(false, { from: user }), 
        "Ownable: caller is not the owner"
      )
    }); 
    it("admin can switch credential generation on/off & switch generates an event", async () => {
      let tx = await bandwidthGenerator.credentialGenerationSwitch(false); 
      expect((await bandwidthGenerator.credentialGenerationEnabled())).to.equal(false); 

      await expectEvent.inTransaction(tx.tx, bandwidthGenerator, 'CredentialGenerationSwitch', {
        Enabled: false, 
      });

      tx = await bandwidthGenerator.credentialGenerationSwitch(true); 
      expect((await bandwidthGenerator.credentialGenerationEnabled())).to.equal(true); 

      await expectEvent.inTransaction(tx.tx, bandwidthGenerator, 'CredentialGenerationSwitch', {
        Enabled: true, 
      });
    }); 
    it("cannot generate credentials if switch = false", async () => {
      await bandwidthGenerator.credentialGenerationSwitch(false); 
      await expectRevert(
        bandwidthGenerator.generateBasicBandwidthCredential(
          oneToken,
          15,
          [0x39, 0x53, 0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba,
            0x39, 0x53, 0x0a, 0x00, 0xea, 0xe2, 0xa5, 0xaa, 0xc8, 0x14, 0x42, 0x09, 0xcc, 0xac, 0x91, 0x7a, 0xe5, 0x6b, 0xf4, 0xa9, 0x58, 0x95, 0x44, 0xcb, 0x00, 0x20, 0xf9, 0x2f, 0xee, 0x35, 0xa3, 0xba],
          cosmosRecipient,  
          { from: user }
        ), "BandwidthGenerator: credential generation isn't currently enabled"      
      ); 
    }); 
  }); 
}); 

