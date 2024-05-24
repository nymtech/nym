#!/usr/bin/python3

import requests as r
import argparse
import sys
import pandas as pd
import json
from json import JSONDecodeError

class MainFunctions:

    def __init__(self):
        self.api_url = "https://validator.nymtech.net/api/v1"
        #self.api_existing_endpoints_url = "https://validator.nymtech.net/api/v1/openapi.json"
        self.api_endpoints_json = "api_endpoints.json"

    def display_results(self, args):
        mode, host, version, mix_id, role, node_df, node_dict, api_data, swagger_data, routing_history = self.collect_all_results(args)
        print("\nNYM NODE INFO\n")
        print(f"Type = {mode}")
        if role:
            print(f"Mode = {role}")
        print(f"Identity Key = {args.id}")
        print(f"Host = {host}")
        print(f"Version = {version}")
        if mix_id:
            print(f"Mix ID = {mix_id}")
        print("\n\nNODE RESULTS FROM UNFILTERED QUERY\n")
        if args.markdown:
            node_markdown = self._dataframe_to_markdown(node_df, ["RESULT"], ["API EDNPOINT"])
            print(node_markdown, "\n")
        else:
            self.print_neat_dict(node_dict)
        print(f"\n\nNODE RESULTS FROM {self.api_url.upper()}\n")
        if args.markdown:
            api_df = self._json_to_dataframe(api_data)
            node_markdown = self._dataframe_to_markdown(api_df, ["RESULT"], ["API EDNPOINT"])
            print(node_markdown, "\n")
        else:
            self.print_neat_dict(api_data)
        if swagger_data:
            print(f"\n\nNODE RESULTS FROM SWAGGER PAGE\n")
            if args.markdown:
                swagger_df = self._json_to_dataframe(swagger_data)
                node_markdown = self._dataframe_to_markdown(swagger_df, ["RESULT"], ["API EDNPOINT"])
                print(node_markdown, "\n")
            else:
                self.print_neat_dict(swagger_data)
        if routing_history:
            print(f"\n\nNODE UPTIME HISTORY\n")
            if args.markdown:
                routing_history_df = self._json_to_dataframe(routing_history)
                print(routing_history_df.to_markdown(index = False))
#                node_markdown = self._dataframe_to_markdown(routing_history_df, ["PERFORMANCE"], ["TIME"])
#                print(node_markdown, "\n")
            else:
                self.print_neat_dict(routing_history)


    def collect_all_results(self,args):
        id_key = args.id
        gateways_unfiltered, mixnodes_unfiltered = self.get_unfiltered_data()
        gateways_df = self._json_to_dataframe(gateways_unfiltered)
        gateways_df = self._set_index_to_empty(gateways_df)
        mixnodes_df = self._json_to_dataframe(mixnodes_unfiltered)
        mixnodes_df = self._set_index_to_empty(mixnodes_df)
        mode, node_df, node_dict = self.get_node_df(id_key, gateways_df, mixnodes_df, gateways_unfiltered, mixnodes_unfiltered)
        host, version, mix_id, role, api_data, swagger_data, routing_history = self.get_node_data(mode, node_dict, id_key, args)
        return mode, host, version, mix_id, role, node_df, node_dict, api_data, swagger_data, routing_history

    def get_node_df(self,id_key, gateways_df, mixnodes_df, gateways_unfiltered,mixnodes_unfiltered):
        if id_key in mixnodes_df['mixnode_details.bond_information.mix_node.identity_key'].values:
            node_df = mixnodes_df.loc[mixnodes_df['mixnode_details.bond_information.mix_node.identity_key'] == id_key]
            node_dict = next((mn for mn in mixnodes_unfiltered if mn['mixnode_details']['bond_information']['mix_node']['identity_key'] == f"{id_key}"), None)
            mode = "mixnode"
        elif id_key in gateways_df["gateway_bond.gateway.identity_key"].values:
            node_df = gateways_df.loc[gateways_df["gateway_bond.gateway.identity_key"] == id_key]
            node_dict = next((gw for gw in gateways_unfiltered if gw['gateway_bond']['gateway']['identity_key'] == f"{id_key}"), None)
            mode = "gateway"
        else:
            print(f"The identity key '{id_key}' does not exist.")
        return mode, node_df, node_dict

    def get_unfiltered_data(self):
        gateways_unfiltered = r.get(f"{self.api_url}/status/gateways/detailed-unfiltered").json()
        mixnodes_unfiltered = r.get(f"{self.api_url}/status/mixnodes/detailed-unfiltered").json()
        return gateways_unfiltered, mixnodes_unfiltered

    def get_mixnode_data(self, node_df, id_key):
        mix_id = int(node_df["mixnode_details.bond_information.mix_id"])


    def get_node_data(self,mode, node_dict, id_key, args):
        #endpoint_json = self.get_api_endpoints()
        identity = id_key
        endpoint_json = self.api_endpoints_json
        with open(endpoint_json, "r") as f:
            dicts = json.load(f)
            endpoints = dicts[mode]
            swagger = dicts["swagger"]
        api_data = {}
        swagger_data = {}
        routing_history = {}
        mix_id = None
        role = None
        if mode == "gateway":
            host = node_dict["gateway_bond"]["gateway"]["host"]
            version = node_dict["gateway_bond"]["gateway"]["version"]
            for key in endpoints:
                endpoint = key.replace("{identity}", identity)
                url = f"{self.api_url}{endpoint}"
                value = r.get(url).json()
                api_data[endpoint] = value
            routing_history = api_data[f"/status/gateway/{identity}/history"]["history"]
            del api_data[f"/status/gateway/{identity}/history"]["history"]
            for key in swagger:
                try:
                    url = f"http://{host}:8080/api/v1{key}"
                    value = r.get(url).json()
                    swagger_data[key] = value
                except r.exceptions.ConnectionError:
                    url = f"https://{host}/api/v1{key}"
                    value = r.get(url).json()
                    swagger_data[key] = value
                except r.exceptions.ConnectionError:
                    url = f"http://{host}/api/v1{key}"
                    value = r.get(url).json()
                    swagger_data[key] = value
                except r.exceptions.ConnectionError as e:
                    print(f"The request to pull data from /api/v1/{key} returns {e}!")
                except (JSONDecodeError, json.JSONDecodeError, r.exceptions.JSONDecodeError):
                    print(f"Endpoint {url} results in 404: Not Found!\n")
            if swagger_data["/roles"]["network_requester_enabled"]== True and swagger_data["/roles"]["ip_packet_router_enabled"] == True:
                role = "exit-gateway"
            else:
                role = "entry-gateway"
        elif mode == "mixnode":
            mix_id = str(node_dict["mixnode_details"]["bond_information"]["mix_id"])
            for key in endpoints:
                endpoint = key.replace("{mix_id}", mix_id)
                url = f"{self.api_url}{endpoint}"
                try:
                    value = r.get(url).json()
                    api_data[endpoint] = value
                except (JSONDecodeError, json.JSONDecodeError, r.exceptions.JSONDecodeError):
                    print(f"Endpoint {url} results in 404: Not Found!\n")
            host = node_dict["mixnode_details"]["bond_information"]["mix_node"]["host"]
            version = node_dict["mixnode_details"]["bond_information"]["mix_node"]["version"]
            routing_history = api_data[f"/status/mixnode/{mix_id}/history"]["history"]
            del api_data[f"/status/mixnode/{mix_id}/history"]["history"]
        else:
            print(f"The mode type {mode} is not recognized!")
            sys.exit(-1)
        host = str(host)

        if args.no_routing_history == True:
            routing_history = None
        else:
            routing_history = routing_history

        return host, version, mix_id, role, api_data, swagger_data, routing_history

#    def get_api_endpoints(self):
#        endpoint_json = r.get(self.api_existing_endpoints_url).json()
#        return endpoint_json

    def _set_index_to_empty(self, df):
        index_len = pd.RangeIndex(len(df.index))
        new_index = []
        for x in index_len:
            x = ""
            new_index.append(x)
        df.index = new_index
        return df

    def _dataframe_to_markdown(self,df,col_names, index_names=""):
        df = df.T
#        print(f"columns = {df.columns}")
#        print(f"index = {df.index}")
        df.index.names = index_names
        df.columns  = col_names
        markdown = df.to_markdown()
        return markdown

    def format_dataframe(self, df):
        #df = pd.DataFrame(df)
        df = self._json_to_dataframe(df)
        df = df.T
        #df.columns = ["API ENDPOINT", "RESULTS"]
        return df

    def print_neat_dict(self, dictionary, indent=4):
        neat_dictionary = self._json_neat_format(dictionary)
        print(neat_dictionary)

    def _json_neat_format(self,dictionary,indent=4):
        dictionary = json.dumps(dictionary, indent = indent)
        return dictionary

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
        parser_pull_stats = subparsers.add_parser('pull_stats',help='Run with node identity key', aliases=['p','P','pull'])

        # pull_stats arguments
        parser_pull_stats.add_argument("id", help="supply nym-node identity key")
        parser_pull_stats.add_argument("-n","--no_routing_history", help="Display node stats without routing history", action="store_true")
        parser_pull_stats.add_argument("-m","--markdown",help="Display node stats in markdown format", action="store_true")
        parser_pull_stats.add_argument("-o","--output",help="Save results to file")


        parser_pull_stats.set_defaults(func=self.functions.display_results)

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
