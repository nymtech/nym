#!/usr/bin/python3

import requests as r
import argparse
import sys
import os
import pandas as pd
import json
import urllib3
import time
from json import JSONDecodeError
from tabulate import tabulate


class MainFunctions:

    def __init__(self):
        self.api_url = "https://validator.nymtech.net/api/v1"
        #TODO: Pull endpoints from: https://validator.nymtech.net/api/v1/openapi.json
        self.api_endpoints_json = "api_endpoints.json"
        self.output = Output()

    def display_results(self, args):
        id_key = args.id
        mode, host, version, mix_id, role, node_df, node_dict, api_data, swagger_data, routing_history = self.collect_all_results(args)
        print("\n============================================================")
        print("\nNYM NODE INFO\n")
        print(f"Type = {mode}")
        if role:
            print(f"Mode = {role}")
        print(f"Identity Key = {id_key}")
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
                swagger_data = self._json_neat_format(swagger_data)
                print(swagger_data)
        else:
            swagger_data = f"\nSwagger API endpoints of node {id_key} hosted on IP: {host} are not responding. Maybe you querying a deprecated version of nym-mixnode or the VPS ports are not open correctly."
        if routing_history:
            print(f"\n\nNODE UPTIME HISTORY\n")
            if args.markdown:
                routing_history_df = self._json_to_dataframe(routing_history)
                print(routing_history_df.to_markdown(index = False))
            else:
                self.print_neat_dict(routing_history)
                routing_history = self._json_neat_format(routing_history)
        else:
            routing_history = " "
        if args.output or args.output == "":
            node_dict = self._json_neat_format(node_dict)
            api_data = self._json_neat_format(api_data)
            data_list = [f"Id. Key = {id_key}", f"Host = {host}", f"Type = {mode}", node_dict, api_data, swagger_data, routing_history]
            self.output.concat_to_file(args, data_list)

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
        print("INFO: Starting to query /detailed-unfiltered endpoint...")
        gateways_unfiltered = r.get(f"{self.api_url}/status/gateways/detailed-unfiltered").json()
        mixnodes_unfiltered = r.get(f"{self.api_url}/status/mixnodes/detailed-unfiltered").json()
        return gateways_unfiltered, mixnodes_unfiltered

    def get_node_data(self,mode, node_dict, id_key, args):
        print("INFO: Sorting out data from the unfiltered endpoint...")
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
                print(f"Querying {url}")
                value = r.get(url).json()
                api_data[endpoint] = value
            routing_history = api_data[f"/status/gateway/{identity}/history"]["history"]
            del api_data[f"/status/gateway/{identity}/history"]["history"]
            swagger_data = self.get_swagger_data(host,swagger,swagger_data)
            if swagger_data["/roles"]["network_requester_enabled"]== True and swagger_data["/roles"]["ip_packet_router_enabled"] == True:
                role = "exit-gateway"
            else:
                role = "entry-gateway"
        elif mode == "mixnode":
            mix_id = str(node_dict["mixnode_details"]["bond_information"]["mix_id"])
            for key in endpoints:
                endpoint = key.replace("{mix_id}", mix_id)
                url = f"{self.api_url}{endpoint}"
                print(f"Querying {url}")
                try:
                    value = r.get(url).json()
                    api_data[endpoint] = value
                except (JSONDecodeError, json.JSONDecodeError, r.exceptions.JSONDecodeError):
                    print(f"Error: Endpoint {url} results in 404: Not Found!")
            host = node_dict["mixnode_details"]["bond_information"]["mix_node"]["host"]
            version = node_dict["mixnode_details"]["bond_information"]["mix_node"]["version"]
            routing_history = api_data[f"/status/mixnode/{mix_id}/history"]["history"]
            del api_data[f"/status/mixnode/{mix_id}/history"]["history"]
            #TODO: try this https://stackoverflow.com/questions/15431044/can-i-set-max-retries-for-requests-request/35504626#35504626
            swagger_data = self.get_swagger_data(host,swagger,swagger_data)
        else:
            print(f"The mode type {mode} is not recognized!")
            sys.exit(-1)
        host = str(host)

        if args.no_routing_history == True:
            routing_history = None
        else:
            routing_history = routing_history

        return host, version, mix_id, role, api_data, swagger_data, routing_history

    def get_swagger_data(self,host,swagger,swagger_data):
        print("INFO: Starting to query SWAGGER API page...")
        for key in swagger:
            try:
                url = f"http://{host}:8080/api/v1{key}"
                print(f"Querying {url}")
                value = r.get(url, timeout=3).json()
                swagger_data[key] = value
            except r.exceptions.ConnectionError:
                url = f"https://{host}/api/v1{key}"
                value = r.get(url, timeout=3).json()
                swagger_data[key] = value
            except r.exceptions.ConnectionError:
                url = f"http://{host}/api/v1{key}"
                value = r.get(url,timeout=3).json()
                swagger_data[key] = value
            except r.exceptions.ConnectionError as e:
                print(f"Error: The request to pull data from /api/v1/{key} returns {e}!")
            except urllib3.exceptions.ProtocolError as e:
                print(f"Error: The request to pull data from /api/v1/{key} returns {e}!")
            except (JSONDecodeError, json.JSONDecodeError, r.exceptions.JSONDecodeError, ConnectionResetError, r.exceptions.ConnectionError) as e:
                print(f"Error: Swagger endpoint {url} results in 404: Not Found! {e}")
            except r.exceptions.ConnectTimeout as e:
                print(f"Error: The request to pull data from /api/v1/{key} returns {e}! We are likely quering a deprecated version of nym-mixnode.")
            except Exception as e:
                print(f"Error: {e}: {url} not responding. Maybe you querying a deprecated version of nym-mixnode?")
        return swagger_data

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
        df.index.names = index_names
        df.columns  = col_names
        markdown = df.to_markdown()
        return markdown

    def format_dataframe(self, df):
        df = self._json_to_dataframe(df)
        df = df.T
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


class Output():

    def __init__(self):
        self.home = os.path.expanduser('~')
        self.pwd = os.path.dirname(os.path.realpath(__file__))

    def concat_to_file(self,args, data_list):
        filename = self.init_output_file(args)
        with open(f"{filename}", "w") as output_file:
            for name in data_list:
                output_file.write(name)
                output_file.write("\n")

        print(f"\nResults were exported to {filename}.")

    def init_output_file(self,args):
        filename = self.get_filename(args)
        os.system(f"touch {filename}")
        return filename

    def get_filename(self,args):
        path = args.output
        id_key = args.id
        file = f"api_output_{id_key}.txt"
        if path == "":
            filename = file
        else:
            if path[-1] != "/":
                path = path + "/"
            if path[0] == "~":
                path = self.home + path[1:]
            filename = f"{path}{file}"
        return filename


class VersionCount():

    def __init__(self):
        self.functions = MainFunctions()
        self.mixnodes_version_column = 'mixnode_details.bond_information.mix_node.version'
        self.gateways_version_column = 'gateway_bond.gateway.version'

    def display_results(self, args):
        df_final = self.fetch_results(args)
        if args.markdown:
            table = df_final.to_markdown(index=False)
        else:
            table = tabulate(df_final)
        print(table)

    def fetch_results(self, args):
        gateways_unfiltered, mixnodes_unfiltered = self.functions.get_unfiltered_data()
        df_gateways = self.functions._json_to_dataframe(gateways_unfiltered)
        df_mixnodes = self.functions._json_to_dataframe(mixnodes_unfiltered)
        versions = list(args.version)
        mixnodes_version_column = self.mixnodes_version_column
        gateways_version_column = self.gateways_version_column
        mixnodes_sum = self.version_count(df_mixnodes, mixnodes_version_column, versions, "mixnode")
        gateways_sum = self.version_count(df_gateways, gateways_version_column, versions, "gateway")
        df_final = self.final_summary(mixnodes_sum, gateways_sum, versions)
        return df_final

    def version_count(self, df, column, versions, mode):
        count_all = []
        for version in versions:
            version_sum = df[f'{column}'].value_counts()[f'{version}']
            result = {"Node type": mode, "Version": version, "Summary":version_sum}
            count_all.append(result)
        return count_all

    def final_summary(self, mixnodes_sum, gateways_sum, versions):
        list_final = mixnodes_sum + gateways_sum
        df_final = pd.DataFrame(list_final)
        col_names = df_final.columns
        total_summary = df_final['Summary'].sum()
        if len(versions) > 1:
            mixnodes_total = df_final.loc[df_final['Node type'] == 'mixnode', 'Summary'].sum()
            gateways_total = df_final.loc[df_final['Node type'] == 'gateway', 'Summary'].sum()
            df_append = pd.DataFrame([["mixnodes",f"versions: {versions}", f"{mixnodes_total}"],["gateways",f"versions: {versions}",f"{gateways_total}"]],columns=col_names)
            df_final = pd.concat([df_final, df_append], ignore_index=True)
            for version in versions:
                version_total = df_final.loc[df_final['Version'] == f'{version}', 'Summary'].sum()
                df_append = pd.DataFrame([["all nodes",f"{version}", f"{version_total}"]],columns=col_names)
                df_final = pd.concat([df_final, df_append], ignore_index=True)
        df_append = pd.DataFrame([["TOTAL SUMMARY",f"{versions}", f"{total_summary}"]],columns=col_names)
        df_final = pd.concat([df_final, df_append], ignore_index=True)
        return df_final


class ArgParser:

    def __init__(self):
        """init for parser"""
        self.functions = MainFunctions()
        self.version_count = VersionCount()

    def parser_main(self):
        """Main function initializing ArgumentParser, storing arguments and executing commands."""
        # Top level parser
        parser = argparse.ArgumentParser(
                prog= "Nym-node API check",
                description='''Run through all endpoints and print results.'''
            )
        parser.add_argument("-V","--version", action="version", version='%(prog)s 0.1.1')

        # sub-command parsers
        subparsers = parser.add_subparsers()
        parser_pull_stats = subparsers.add_parser('query_stats',help='Get all nodes API endpoints', aliases=['q','query'])
        parser_version_count = subparsers.add_parser('version_count', help='Sum of nodes in given version(s)', aliases=['v','version'])

        # pull_stats arguments
        parser_pull_stats.add_argument("id", help="supply nym-node identity key")
        parser_pull_stats.add_argument("-n","--no_routing_history", help="Display node stats without routing history", action="store_true")
        parser_pull_stats.add_argument("-m","--markdown",help="Display results in markdown format", action="store_true")
        parser_pull_stats.add_argument("-o","--output",help="Save results to file (in current dir or supply with path without filename)", nargs='?',const="", type=str)
        parser_pull_stats.set_defaults(func=self.functions.display_results)


        # version_count arguments
        parser_version_count.add_argument('version', help="supply node versions separated with space", nargs='+')
        parser_version_count.add_argument("-m","--markdown",help="Display results in markdown format", action="store_true")
        parser_version_count.set_defaults(func=self.version_count.display_results)

        args = parser.parse_args()

        try:
            func = args.func
            try:
                args.func(args)
            except (AttributeError, KeyError) as e:
                msg = f"{e}.\nPlease run python {__file__} --help"
                self.panic(msg)
            except UnboundLocalError as e:
                msg = f"{e}.\nPlease provide a correct node identity key."
                self.panic(msg)
        except FileNotFoundError as e:
            msg = f"{e}.\nMake sure your <PATH> supplied to --output is correct."
            self.panic(msg)
        except AttributeError:
            parser.print_help(sys.stderr)


    def panic(self,msg):
        """Error message print"""
        print(f"error: {msg}", file=sys.stderr)
        sys.exit(-1)

if __name__ == '__main__':
    node_check = ArgParser()
    node_check.parser_main()
