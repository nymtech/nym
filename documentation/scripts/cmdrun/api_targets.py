import argparse
import os
import requests
import json
import sys
import pandas as pd
from collections import namedtuple

def get_url(args):
    config_file = "./api_targets_config.json"
    with open(config_file, "r") as f:
        config = json.load(f)
    env = args.env
    endpoint = args.endpoint
    url = f"{config[env]}/api/v1/{endpoint}"
    return url

def subparser_read(args):
    url = get_url(args)
    r = requests.get(url)
    response = r.json()
    return response

def convert_u_nym(unym):
    unym = int(unym)
    nym = unym / 1000000
    nym = int(nym)
    return nym

def thousand_separator(n):
    n = f'{n:_}'
    return n

def remove_underscore(arg):
    string = arg.replace("_", " ")
    string = string.title()
    return string

def display_supply_table(response, args):
    df = pd.DataFrame(response)
    df = df.T
    del df['denom']
#    df.set_axis(['**Item**', '**Amount in NYM**'], axis=1, inplace=True)
    df = df.rename_axis('index1').reset_index()
    df = df.rename(columns={'index1': '**Item**', 'amount': '**Amount in NYM**'})
    df['**Item**'] = df['**Item**'].apply(remove_underscore)
    df['**Amount in NYM**'] = df['**Amount in NYM**'].apply(convert_u_nym)
    df['**Amount in NYM**'] = df['**Amount in NYM**'].apply(thousand_separator)
    table = df.to_markdown(index=False)
    print(table)

def read_supply(args):
    response = subparser_read(args)
    if args.endpoint == "circulating-supply":
        if args.format:
            display_supply_table(response, args)
        else:
            print(response)
    elif args.endpoint == "foo":
        # placeholder for other endpoint args
        pass
    else:
        # placeholder for other endpoint args
        pass


def parser_main():
    parser = argparse.ArgumentParser(
            prog="Nym API scraper",
            description='''Get any live data from Nyx validator''',
            epilog=''
            )
    subparsers = parser.add_subparsers(help="")
    parser_supply = subparsers.add_parser('supply',
            help='reads API on supply',
            aliases=['s','S']
            )

    parser_supply.add_argument(
            "-v","--env",
            type=str,
            default="mainnet",
            help="choose: mainnet, perf, sandbox"
            )
    parser_supply.add_argument(
            "-e","--endpoint",
            type=str,
            help="choose from: https://validator.nymtech.net/api/swagger/index.html"
            )
    parser_supply.add_argument(
            "-f","--format",
            action="store_true",
            help="format the output for documentation purpose (.md) - default: False (raw output)",
            )

    parser_supply.set_defaults(func=read_supply)


    args = parser.parse_args()
    try:
        args.func(args)
    except AttributeError as e:
        print("Error on argparser")
        sys.exit(-1)




if __name__ == "__main__":
    parser_main()
