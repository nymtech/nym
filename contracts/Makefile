schema: coconut-dkg-schema mixnet-schema vesting-schema multisig-schema group-schema ecash-schema nym-pool-schema performance-schema offline-signers-schema

coconut-dkg-schema:
	$(MAKE) -C coconut-dkg generate-schema

mixnet-schema:
	$(MAKE) -C mixnet generate-schema

vesting-schema:
	$(MAKE) -C vesting generate-schema

ecash-schema:
	$(MAKE) -C ecash generate-schema

multisig-schema:
	$(MAKE) -C multisig/cw3-flex-multisig generate-schema

group-schema:
	$(MAKE) -C multisig/cw4-group generate-schema

nym-pool-schema:
	$(MAKE) -C nym-pool generate-schema

performance-schema:
	$(MAKE) -C performance generate-schema

offline-signers-schema:
	$(MAKE) -C offline-signers generate-schema