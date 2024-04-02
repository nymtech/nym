import argparse
import os
import requests
import json
import sys
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
    print(response)


def parser_main():
    parser = argparse.ArgumentParser(
            prog="Nym API scraper",
            description='''Get any live data from Nyx validator''',
            epilog=''
            )
    subparsers = parser.add_subparsers(help="prints this message")
    parser_api_read = subparsers.add_parser('read',
            help='reads API endpoint value',
            aliases=['r','R']
            )

    parser_api_read.add_argument(
            "-v","--env",
            type=str,
            default="mainnet",
            help="choose: mainnet, perf, sandbox"
            )
    parser_api_read.add_argument(
            "-e","--endpoint",
            type=str,
            help="choose from: https://validator.nymtech.net/api/swagger/index.html"
            )
    parser_api_read.set_defaults(func=subparser_read)


    args = parser.parse_args()
    try:
        args.func(args)
    except AttributeError as e:
        print("Error on argparser")
        sys.exit(-1)




if __name__ == "__main__":
    parser_main()
