schema: coconut-dkg-schema mixnet-schema vesting-schema multisig-schema group-schema ecash-schema

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
