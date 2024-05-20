#!/usr/bin/python3

import requests as r
import argparse
import sys
import pandas as pd
import json

class MainFunctions:

    def __init__(self):
        self.api_url = "https://validator.nymtech.net/api/v1"
        #self.api_existing_endpoints_url = "https://validator.nymtech.net/api/v1/openapi.json"
        self.api_endpoints_json = "api_endpoints.json"

    def display_results(self,args):
        id_key = args.id
        gateways_unfiltered, mixnodes_unfiltered = self.get_unfiltered_data()
        gateways_df = self._json_to_dataframe(gateways_unfiltered)
        mixnodes_df = self._json_to_dataframe(mixnodes_unfiltered)
        mode, node_series = self.node_type_check(id_key, gateways_df, mixnodes_df)
        print(f"mode = {mode}")
        node_data = self.get_node_data(mode, node_series, id_key)
        print(node_series.T, node_data)

    def node_type_check(self,id_key, gateways_df, mixnodes_df):
        if id_key in mixnodes_df['mixnode_details.bond_information.mix_node.identity_key'].values:

            node = mixnodes_df.loc[mixnodes_df['mixnode_details.bond_information.mix_node.identity_key'] == id_key]
            mode = "mixnode"
        elif id_key in gateways_df['gateway_bond.gateway.identity_key'].values:
            node = gateways_df.loc[gateways_df['gateway_bond.gateway.identity_key'] == id_key]
            mode = "gateway"
        else:
            print(f"The identity key '{id_key}' does not exist.")

        return mode, node

    def get_unfiltered_data(self):
        gateways_unfiltered = r.get(f"{self.api_url}/status/gateways/detailed-unfiltered").json()
        mixnodes_unfiltered = r.get(f"{self.api_url}/status/mixnodes/detailed-unfiltered").json()
        return gateways_unfiltered, mixnodes_unfiltered

    def get_mixnode_data(self, node_series, id_key):
        mix_id = int(node_series["mixnode_details.bond_information.mix_id"])


    def get_node_data(self,mode, node_series, id_key):
        #endpoint_json = self.get_api_endpoints()
        identity = id_key
        endpoint_json = self.api_endpoints_json
        with open(endpoint_json, "r") as f:
            dicts = json.load(f)
            enpoints = dicts[mode]
        node_data = {}
        if mode == "gateway":
            for key in enpoints:
                url = f"{self.api_url}{key}"
                value = r.get(url)
                node_data[key] = value
        elif mode == "mixnode":
            mix_id = int(node_series["mixnode_details.bond_information.mix_id"])
        else:
            print(f"The mode type {mode} is not recognized!")
            sys.exit(-1)
        return node_data


#    def get_api_endpoints(self):
#        endpoint_json = r.get(self.api_existing_endpoints_url).json()
#        return endpoint_json
#
    def _json_to_dataframe(self,json):
        df = pd.json_normalize(json)
        return df

class ArgParser:

    def __init__(self):
        """init for parser"""
        self.functions = MainFunctions()

    def parser_main(self):
        """Main function initializing ArgumentParser, storing arguments and executing commands."""
        # Top level parser
        parser = argparse.ArgumentParser(
                prog= "Nym-node API check",
                description='''Run through all endpoints and print results.'''
            )
        parser.add_argument("-V","--version", action="version", version='%(prog)s 0.1.0')

        # sub-command parsers
        subparsers = parser.add_subparsers(help="{subcommand}[-h] shows all the options")
        parser_check = subparsers.add_parser('check',help='Run with node identity key', aliases=['c','C'])

        # check - arguments
        parser_check.add_argument("id", help="supply nym-node identity key")
        parser_check.set_defaults(func=self.functions.display_results)

        args = parser.parse_args()

        try:
            args.func(args)
        except AttributeError as e:
            msg = f"{e}.\nPlease run __file__ --help"
            self.panic(msg)


    def panic(self,msg):
        """Error message print"""
        print(f"error: {msg}", file=sys.stderr)
        sys.exit(-1)

if __name__ == '__main__':
    node_check = ArgParser()
    node_check.parser_main()
