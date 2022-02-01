//SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.10;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Address.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "./CosmosToken.sol";

error InvalidSignature();
error InvalidValsetNonce(uint256 newNonce, uint256 currentNonce);
error InvalidBatchNonce(uint256 newNonce, uint256 currentNonce);
error InvalidLogicCallNonce(uint256 newNonce, uint256 currentNonce);
error InvalidLogicCallTransfers();
error InvalidLogicCallFees();
error InvalidSendToCosmos();
error IncorrectCheckpoint();
error MalformedNewValidatorSet();
error MalformedCurrentValidatorSet();
error MalformedBatch();
error InsufficientPower(uint256 cumulativePower, uint256 powerThreshold);
error BatchTimedOut();
error LogicCallTimedOut();

// This is being used purely to avoid stack too deep errors
struct LogicCallArgs {
	// Transfers out to the logic contract
	uint256[] transferAmounts;
	address[] transferTokenContracts;
	// The fees (transferred to msg.sender)
	uint256[] feeAmounts;
	address[] feeTokenContracts;
	// The arbitrary logic call
	address logicContractAddress;
	bytes payload;
	// Invalidation metadata
	uint256 timeOut;
	bytes32 invalidationId;
	uint256 invalidationNonce;
}

// This is used purely to avoid stack too deep errors
// represents everything about a given validator set
struct ValsetArgs {
	// the validators in this set, represented by an Ethereum address
	address[] validators;
	// the powers of the given validators in the same order as above
	uint256[] powers;
	// the nonce of this validator set
	uint256 valsetNonce;
	// the reward amount denominated in the below reward token, can be
	// set to zero
	uint256 rewardAmount;
	// the reward token, should be set to the zero address if not being used
	address rewardToken;
}

// This represents a validator signature
struct Signature {
	uint8 v;
	bytes32 r;
	bytes32 s;
}

contract Gravity is ReentrancyGuard {
	using SafeERC20 for IERC20;

	// The number of 'votes' required to execute a valset
	// update or batch execution, set to 2/3 of 2^32
	uint256 constant constant_powerThreshold = 2863311530;

	// These are updated often
	bytes32 public state_lastValsetCheckpoint;
	mapping(address => uint256) public state_lastBatchNonces;
	mapping(bytes32 => uint256) public state_invalidationMapping;
	uint256 public state_lastValsetNonce = 0;
	// event nonce zero is reserved by the Cosmos module as a special
	// value indicating that no events have yet been submitted
	uint256 public state_lastEventNonce = 1;

	// This is set once at initialization
	bytes32 public immutable state_gravityId;

	// TransactionBatchExecutedEvent and SendToCosmosEvent both include the field _eventNonce.
	// This is incremented every time one of these events is emitted. It is checked by the
	// Cosmos module to ensure that all events are received in order, and that none are lost.
	//
	// ValsetUpdatedEvent does not include the field _eventNonce because it is never submitted to the Cosmos
	// module. It is purely for the use of relayers to allow them to successfully submit batches.
	event TransactionBatchExecutedEvent(
		uint256 indexed _batchNonce,
		address indexed _token,
		uint256 _eventNonce
	);
	event SendToCosmosEvent(
		address indexed _tokenContract,
		address indexed _sender,
		string _destination,
		uint256 _amount,
		uint256 _eventNonce
	);
	event ERC20DeployedEvent(
		// FYI: Can't index on a string without doing a bunch of weird stuff
		string _cosmosDenom,
		address indexed _tokenContract,
		string _name,
		string _symbol,
		uint8 _decimals,
		uint256 _eventNonce
	);
	event ValsetUpdatedEvent(
		uint256 indexed _newValsetNonce,
		uint256 _eventNonce,
		uint256 _rewardAmount,
		address _rewardToken,
		address[] _validators,
		uint256[] _powers
	);
	event LogicCallEvent(
		bytes32 _invalidationId,
		uint256 _invalidationNonce,
		bytes _returnData,
		uint256 _eventNonce
	);

	// TEST FIXTURES
	// These are here to make it easier to measure gas usage. They should be removed before production
	function testMakeCheckpoint(ValsetArgs calldata _valsetArgs, bytes32 _gravityId) external pure {
		makeCheckpoint(_valsetArgs, _gravityId);
	}

	function testCheckValidatorSignatures(
		ValsetArgs calldata _currentValset,
		Signature[] calldata _sigs,
		bytes32 _theHash,
		uint256 _powerThreshold
	) external pure {
		checkValidatorSignatures(_currentValset, _sigs, _theHash, _powerThreshold);
	}

	// END TEST FIXTURES

	function lastBatchNonce(address _erc20Address) external view returns (uint256) {
		return state_lastBatchNonces[_erc20Address];
	}

	function lastLogicCallNonce(bytes32 _invalidation_id) external view returns (uint256) {
		return state_invalidationMapping[_invalidation_id];
	}

	// Utility function to verify geth style signatures
	function verifySig(
		address _signer,
		bytes32 _theHash,
		Signature calldata _sig
	) private pure returns (bool) {
		bytes32 messageDigest = keccak256(
			abi.encodePacked("\x19Ethereum Signed Message:\n32", _theHash)
		);
		return _signer == ECDSA.recover(messageDigest, _sig.v, _sig.r, _sig.s);
	}

	// Utility function to determine that a validator set and signatures are well formed
	function validateValset(ValsetArgs calldata _valset, Signature[] calldata _sigs) private pure {
		// Check that current validators, powers, and signatures (v,r,s) set is well-formed
		if (
			_valset.validators.length != _valset.powers.length ||
			_valset.validators.length != _sigs.length
		) {
			revert MalformedCurrentValidatorSet();
		}
	}

	// Make a new checkpoint from the supplied validator set
	// A checkpoint is a hash of all relevant information about the valset. This is stored by the contract,
	// instead of storing the information directly. This saves on storage and gas.
	// The format of the checkpoint is:
	// h(gravityId, "checkpoint", valsetNonce, validators[], powers[])
	// Where h is the keccak256 hash function.
	// The validator powers must be decreasing or equal. This is important for checking the signatures on the
	// next valset, since it allows the caller to stop verifying signatures once a quorum of signatures have been verified.
	function makeCheckpoint(ValsetArgs memory _valsetArgs, bytes32 _gravityId)
		private
		pure
		returns (bytes32)
	{
		// bytes32 encoding of the string "checkpoint"
		bytes32 methodName = 0x636865636b706f696e7400000000000000000000000000000000000000000000;

		bytes32 checkpoint = keccak256(
			abi.encode(
				_gravityId,
				methodName,
				_valsetArgs.valsetNonce,
				_valsetArgs.validators,
				_valsetArgs.powers,
				_valsetArgs.rewardAmount,
				_valsetArgs.rewardToken
			)
		);

		return checkpoint;
	}

	function checkValidatorSignatures(
		// The current validator set and their powers
		ValsetArgs calldata _currentValset,
		// The current validator's signatures
		Signature[] calldata _sigs,
		// This is what we are checking they have signed
		bytes32 _theHash,
		uint256 _powerThreshold
	) private pure {
		uint256 cumulativePower = 0;

		for (uint256 i = 0; i < _currentValset.validators.length; i++) {
			// If v is set to 0, this signifies that it was not possible to get a signature from this validator and we skip evaluation
			// (In a valid signature, it is either 27 or 28)
			if (_sigs[i].v != 0) {
				// Check that the current validator has signed off on the hash
				if (!verifySig(_currentValset.validators[i], _theHash, _sigs[i])) {
					revert InvalidSignature();
				}

				// Sum up cumulative power
				cumulativePower = cumulativePower + _currentValset.powers[i];

				// Break early to avoid wasting gas
				if (cumulativePower > _powerThreshold) {
					break;
				}
			}
		}

		// Check that there was enough power
		if (cumulativePower <= _powerThreshold) {
			revert InsufficientPower(cumulativePower, _powerThreshold);
		}
		// Success
	}

	// This updates the valset by checking that the validators in the current valset have signed off on the
	// new valset. The signatures supplied are the signatures of the current valset over the checkpoint hash
	// generated from the new valset.
	// Anyone can call this function, but they must supply valid signatures of constant_powerThreshold of the current valset over
	// the new valset.
	function updateValset(
		// The new version of the validator set
		ValsetArgs calldata _newValset,
		// The current validators that approve the change
		ValsetArgs calldata _currentValset,
		// These are arrays of the parts of the current validator's signatures
		Signature[] calldata _sigs
	) external {
		// CHECKS

		// Check that the valset nonce is greater than the old one
		if (_newValset.valsetNonce <= _currentValset.valsetNonce) {
			revert InvalidValsetNonce({
				newNonce: _newValset.valsetNonce,
				currentNonce: _currentValset.valsetNonce
			});
		}

		// Check that the valset nonce is less than a million nonces forward from the old one
		// this makes it difficult for an attacker to lock out the contract by getting a single
		// bad validator set through with uint256 max nonce
		if (_newValset.valsetNonce > _currentValset.valsetNonce + 1000000) {
			revert InvalidValsetNonce({
				newNonce: _newValset.valsetNonce,
				currentNonce: _currentValset.valsetNonce
			});
		}

		// Check that new validators and powers set is well-formed
		if (
			_newValset.validators.length != _newValset.powers.length ||
			_newValset.validators.length == 0
		) {
			revert MalformedNewValidatorSet();
		}

		// Check that current validators, powers, and signatures (v,r,s) set is well-formed
		validateValset(_currentValset, _sigs);

		// Check cumulative power to ensure the contract has sufficient power to actually
		// pass a vote
		uint256 cumulativePower = 0;
		for (uint256 i = 0; i < _newValset.powers.length; i++) {
			cumulativePower = cumulativePower + _newValset.powers[i];
			if (cumulativePower > constant_powerThreshold) {
				break;
			}
		}
		if (cumulativePower <= constant_powerThreshold) {
			revert InsufficientPower({
				cumulativePower: cumulativePower,
				powerThreshold: constant_powerThreshold
			});
		}

		// Check that the supplied current validator set matches the saved checkpoint
		if (makeCheckpoint(_currentValset, state_gravityId) != state_lastValsetCheckpoint) {
			revert IncorrectCheckpoint();
		}

		// Check that enough current validators have signed off on the new validator set
		bytes32 newCheckpoint = makeCheckpoint(_newValset, state_gravityId);

		checkValidatorSignatures(_currentValset, _sigs, newCheckpoint, constant_powerThreshold);

		// ACTIONS

		// Stored to be used next time to validate that the valset
		// supplied by the caller is correct.
		state_lastValsetCheckpoint = newCheckpoint;

		// Store new nonce
		state_lastValsetNonce = _newValset.valsetNonce;

		// Send submission reward to msg.sender if reward token is a valid value
		if (_newValset.rewardToken != address(0) && _newValset.rewardAmount != 0) {
			IERC20(_newValset.rewardToken).safeTransfer(msg.sender, _newValset.rewardAmount);
		}

		// LOGS

		state_lastEventNonce = state_lastEventNonce + 1;
		emit ValsetUpdatedEvent(
			_newValset.valsetNonce,
			state_lastEventNonce,
			_newValset.rewardAmount,
			_newValset.rewardToken,
			_newValset.validators,
			_newValset.powers
		);
	}

	// submitBatch processes a batch of Cosmos -> Ethereum transactions by sending the tokens in the transactions
	// to the destination addresses. It is approved by the current Cosmos validator set.
	// Anyone can call this function, but they must supply valid signatures of constant_powerThreshold of the current valset over
	// the batch.
	function submitBatch(
		// The validators that approve the batch
		ValsetArgs calldata _currentValset,
		// These are arrays of the parts of the validators signatures
		Signature[] calldata _sigs,
		// The batch of transactions
		uint256[] calldata _amounts,
		address[] calldata _destinations,
		uint256[] calldata _fees,
		uint256 _batchNonce,
		address _tokenContract,
		// a block height beyond which this batch is not valid
		// used to provide a fee-free timeout
		uint256 _batchTimeout
	) external nonReentrant {
		// CHECKS scoped to reduce stack depth
		{
			// Check that the batch nonce is higher than the last nonce for this token
			if (_batchNonce <= state_lastBatchNonces[_tokenContract]) {
				revert InvalidBatchNonce({
					newNonce: _batchNonce,
					currentNonce: state_lastBatchNonces[_tokenContract]
				});
			}

			// Check that the batch nonce is less than one million nonces forward from the old one
			// this makes it difficult for an attacker to lock out the contract by getting a single
			// bad batch through with uint256 max nonce
			if (_batchNonce > state_lastBatchNonces[_tokenContract] + 1000000) {
				revert InvalidBatchNonce({
					newNonce: _batchNonce,
					currentNonce: state_lastBatchNonces[_tokenContract]
				});
			}

			// Check that the block height is less than the timeout height
			if (block.number >= _batchTimeout) {
				revert BatchTimedOut();
			}

			// Check that current validators, powers, and signatures (v,r,s) set is well-formed
			validateValset(_currentValset, _sigs);

			// Check that the supplied current validator set matches the saved checkpoint
			if (makeCheckpoint(_currentValset, state_gravityId) != state_lastValsetCheckpoint) {
				revert IncorrectCheckpoint();
			}

			// Check that the transaction batch is well-formed
			if (_amounts.length != _destinations.length || _amounts.length != _fees.length) {
				revert MalformedBatch();
			}

			// Check that enough current validators have signed off on the transaction batch and valset
			checkValidatorSignatures(
				_currentValset,
				_sigs,
				// Get hash of the transaction batch and checkpoint
				keccak256(
					abi.encode(
						state_gravityId,
						// bytes32 encoding of "transactionBatch"
						0x7472616e73616374696f6e426174636800000000000000000000000000000000,
						_amounts,
						_destinations,
						_fees,
						_batchNonce,
						_tokenContract,
						_batchTimeout
					)
				),
				constant_powerThreshold
			);

			// ACTIONS

			// Store batch nonce
			state_lastBatchNonces[_tokenContract] = _batchNonce;

			{
				// Send transaction amounts to destinations
				uint256 totalFee;
				for (uint256 i = 0; i < _amounts.length; i++) {
					IERC20(_tokenContract).safeTransfer(_destinations[i], _amounts[i]);
					totalFee = totalFee + _fees[i];
				}

				// Send transaction fees to msg.sender
				IERC20(_tokenContract).safeTransfer(msg.sender, totalFee);
			}
		}

		// LOGS scoped to reduce stack depth
		{
			state_lastEventNonce = state_lastEventNonce + 1;
			emit TransactionBatchExecutedEvent(_batchNonce, _tokenContract, state_lastEventNonce);
		}
	}

	// This makes calls to contracts that execute arbitrary logic
	// First, it gives the logic contract some tokens
	// Then, it gives msg.senders tokens for fees
	// Then, it calls an arbitrary function on the logic contract
	// invalidationId and invalidationNonce are used for replay prevention.
	// They can be used to implement a per-token nonce by setting the token
	// address as the invalidationId and incrementing the nonce each call.
	// They can be used for nonce-free replay prevention by using a different invalidationId
	// for each call.
	function submitLogicCall(
		// The validators that approve the call
		ValsetArgs calldata _currentValset,
		// These are arrays of the parts of the validators signatures
		Signature[] calldata _sigs,
		LogicCallArgs memory _args
	) external nonReentrant {
		// CHECKS scoped to reduce stack depth
		{
			// Check that the call has not timed out
			if (block.number >= _args.timeOut) {
				revert LogicCallTimedOut();
			}

			// Check that the invalidation nonce is higher than the last nonce for this invalidation Id
			if (state_invalidationMapping[_args.invalidationId] >= _args.invalidationNonce) {
				revert InvalidLogicCallNonce({
					newNonce: _args.invalidationNonce,
					currentNonce: state_invalidationMapping[_args.invalidationId]
				});
			}

			// note the lack of nonce skipping check, it's not needed here since an attacker
			// will never be able to fill the invalidationId space, therefore a nonce lockout
			// is simply not possible

			// Check that current validators, powers, and signatures (v,r,s) set is well-formed
			validateValset(_currentValset, _sigs);

			// Check that the supplied current validator set matches the saved checkpoint
			if (makeCheckpoint(_currentValset, state_gravityId) != state_lastValsetCheckpoint) {
				revert IncorrectCheckpoint();
			}

			if (_args.transferAmounts.length != _args.transferTokenContracts.length) {
				revert InvalidLogicCallTransfers();
			}

			if (_args.feeAmounts.length != _args.feeTokenContracts.length) {
				revert InvalidLogicCallFees();
			}
		}
		{
			bytes32 argsHash = keccak256(
				abi.encode(
					state_gravityId,
					// bytes32 encoding of "logicCall"
					0x6c6f67696343616c6c0000000000000000000000000000000000000000000000,
					_args.transferAmounts,
					_args.transferTokenContracts,
					_args.feeAmounts,
					_args.feeTokenContracts,
					_args.logicContractAddress,
					_args.payload,
					_args.timeOut,
					_args.invalidationId,
					_args.invalidationNonce
				)
			);

			// Check that enough current validators have signed off on the transaction batch and valset
			checkValidatorSignatures(
				_currentValset,
				_sigs,
				// Get hash of the transaction batch and checkpoint
				argsHash,
				constant_powerThreshold
			);
		}

		// ACTIONS

		// Update invaldiation nonce
		state_invalidationMapping[_args.invalidationId] = _args.invalidationNonce;

		// Send tokens to the logic contract
		for (uint256 i = 0; i < _args.transferAmounts.length; i++) {
			IERC20(_args.transferTokenContracts[i]).safeTransfer(
				_args.logicContractAddress,
				_args.transferAmounts[i]
			);
		}

		// Make call to logic contract
		bytes memory returnData = Address.functionCall(_args.logicContractAddress, _args.payload);

		// Send fees to msg.sender
		for (uint256 i = 0; i < _args.feeAmounts.length; i++) {
			IERC20(_args.feeTokenContracts[i]).safeTransfer(msg.sender, _args.feeAmounts[i]);
		}

		// LOGS scoped to reduce stack depth
		{
			state_lastEventNonce = state_lastEventNonce + 1;
			emit LogicCallEvent(
				_args.invalidationId,
				_args.invalidationNonce,
				returnData,
				state_lastEventNonce
			);
		}
	}

	function sendToCosmos(
		address _tokenContract,
		string calldata _destination,
		uint256 _amount
	) external nonReentrant {
		// we snapshot our current balance of this token
		uint256 ourStartingBalance = IERC20(_tokenContract).balanceOf(address(this));

		// attempt to transfer the user specified amount
		IERC20(_tokenContract).safeTransferFrom(msg.sender, address(this), _amount);

		// check what this particular ERC20 implementation actually gave us, since it doesn't
		// have to be at all related to the _amount
		uint256 ourEndingBalance = IERC20(_tokenContract).balanceOf(address(this));

		// a very strange ERC20 may trigger this condition, if we didn't have this we would
		// underflow, so it's mostly just an error message printer
		if (ourEndingBalance <= ourStartingBalance) {
			revert InvalidSendToCosmos();
		}

		state_lastEventNonce = state_lastEventNonce + 1;

		// emit to Cosmos the actual amount our balance has changed, rather than the user
		// provided amount. This protects against a small set of wonky ERC20 behavior, like
		// burning on send but not tokens that for example change every users balance every day.
		emit SendToCosmosEvent(
			_tokenContract,
			msg.sender,
			_destination,
			ourEndingBalance - ourStartingBalance,
			state_lastEventNonce
		);
	}

	function deployERC20(
		string calldata _cosmosDenom,
		string calldata _name,
		string calldata _symbol,
		uint8 _decimals
	) external {
		// Deploy an ERC20 with entire supply granted to Gravity.sol
		CosmosERC20 erc20 = new CosmosERC20(address(this), _name, _symbol, _decimals);

		// Fire an event to let the Cosmos module know
		state_lastEventNonce = state_lastEventNonce + 1;
		emit ERC20DeployedEvent(
			_cosmosDenom,
			address(erc20),
			_name,
			_symbol,
			_decimals,
			state_lastEventNonce
		);
	}

	constructor(
		// A unique identifier for this gravity instance to use in signatures
		bytes32 _gravityId,
		// The validator set, not in valset args format since many of it's
		// arguments would never be used in this case
		address[] memory _validators,
		uint256[] memory _powers
	) {
		// CHECKS

		// Check that validators, powers, and signatures (v,r,s) set is well-formed
		if (_validators.length != _powers.length || _validators.length == 0) {
			revert MalformedCurrentValidatorSet();
		}

		// Check cumulative power to ensure the contract has sufficient power to actually
		// pass a vote
		uint256 cumulativePower = 0;
		for (uint256 i = 0; i < _powers.length; i++) {
			cumulativePower = cumulativePower + _powers[i];
			if (cumulativePower > constant_powerThreshold) {
				break;
			}
		}
		if (cumulativePower <= constant_powerThreshold) {
			revert InsufficientPower({
				cumulativePower: cumulativePower,
				powerThreshold: constant_powerThreshold
			});
		}

		ValsetArgs memory _valset;
		_valset = ValsetArgs(_validators, _powers, 0, 0, address(0));

		bytes32 newCheckpoint = makeCheckpoint(_valset, _gravityId);

		// ACTIONS

		state_gravityId = _gravityId;
		state_lastValsetCheckpoint = newCheckpoint;

		// LOGS

		emit ValsetUpdatedEvent(
			state_lastValsetNonce,
			state_lastEventNonce,
			0,
			address(0),
			_validators,
			_powers
		);
	}
}