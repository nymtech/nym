schema: coconut-bandwidth-schema coconut-dkg-schema mixnet-schema name-service-schema service-provider-directory-schema vesting-schema multisig-schema group-schema

coconut-bandwidth-schema:
	$(MAKE) -C coconut-bandwidth generate-schema

coconut-dkg-schema:
	$(MAKE) -C coconut-dkg generate-schema

mixnet-schema:
	$(MAKE) -C mixnet generate-schema

name-service-schema:
	$(MAKE) -C name-service generate-schema

service-provider-directory-schema:
	$(MAKE) -C service-provider-directory generate-schema

vesting-schema:
	$(MAKE) -C vesting generate-schema

multisig-schema:
	$(MAKE) -C multisig/cw3-flex-multisig generate-schema

group-schema:
	$(MAKE) -C multisig/cw4-group generate-schema
