#!/usr/bin/python
import json

# This script alters the genesis file so that only user account "dave"
# can upload smart contracts.

genesis_filename = "/home/dave/.nymd/config/genesis.json"
dave_address_filename = "./accounts/dave.address"

with open(dave_address_filename, "r") as dave_address_file:
    dave_address = dave_address_file.readline()


genesis_file = open(genesis_filename, "r")
genesis_json = json.load(genesis_file)
genesis_file.close()
wasm_params = genesis_json['app_state']['wasm']['params']
wasm_uploads = wasm_params['code_upload_access']
wasm_uploads['permission'] = "OnlyAddress"
wasm_uploads['address'] = dave_address.rstrip()
print(wasm_params)
print(wasm_uploads)

print(genesis_json)
genesis_file = open(genesis_filename, "w")
json.dump(genesis_json, genesis_file)
genesis_file.close()
