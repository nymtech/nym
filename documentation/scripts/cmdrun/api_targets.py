#!/usr/bin/python3

import argparse
import os
import requests
import json
import sys
import pandas as pd
from collections import namedtuple
from time import gmtime, strftime
import time

############################################
############## GENERAL FNs #################
############################################

def get_url(args):
    config_file = "./api_targets_config.json"
    with open(config_file, "r") as f:
        config = json.load(f)
    env = args.api
    endpoint = args.endpoint
    if env == "github":
        url = f"{config[env]}/{endpoint}"
    else:
        url = f"{config[env]}/api/v1/{endpoint}"
    return url

def subparser_read(args):
    url = get_url(args)
    r = requests.get(url)
    response = r.json()
    return response

def print_time_now(args):
    #now = datetime.now().strftime('%Y-%m-%d %H:%M:%S')
    #now = time.ctime()
    day = strftime("%d", gmtime())
    if day[0] == "0":
        day = day[1]
    if day == "1" or day == "21" or day == "31":
        suffix = "st"
    elif day == "2" or day == "22":
        suffix = "nd"
    elif day == "3" or day == "23":
        suffix = "rd"
    else:
        suffix = "th"
    now = strftime(f"%A, %B {day}{suffix} %Y, %X UTC", gmtime())
    print(now)

############################################
########### NYX RELATED FNs ################
############################################

def convert_u_nym(unym):
    unym = int(unym)
    nym = unym / 1000000
    nym = int(nym)
    return nym

def thousand_separator(n, separator):
    if separator == " ":
        n = f'{n:_}'
        n = remove_underscore(n)
    else:
        n = f'{n:{separator}}'
    return n

def remove_underscore(arg):
    string = arg.replace("_", " ")
    string = string.title()
    return string

def display_supply_table(response, args):
    separator = args.separator
    df = pd.DataFrame(response)
    df = df.T
    del df['denom']
#    df.set_axis(['**Item**', '**Amount in NYM**'], axis=1, inplace=True)
    df = df.rename_axis('index1').reset_index()
    df = df.rename(columns={'index1': '**Item**', 'amount': '**Amount in NYM**'})
    df['**Item**'] = df['**Item**'].apply(remove_underscore)
    df['**Amount in NYM**'] = df['**Amount in NYM**'].apply(convert_u_nym)
    df['**Amount in NYM**'] = df['**Amount in NYM**'].apply(thousand_separator, args=(separator, ))
    desc_column = _get_desc_column()
    df.insert(1, '**Description**', desc_column, True)
    table = df.to_markdown(index=False,colalign=("left","left","right"))
    print(table)

def _get_desc_column():
    supply = "Maximum amount of NYM token in existence"
    reserve = "Tokens releasing for operators rewards"
    vesting = "Tokens locked outside of cicrulation for future claim"
    circulating = "Amount of unlocked tokens"
    desc_column = [supply, reserve, vesting, circulating]
    return desc_column

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

###########################################
############ GH RELATED FNs ###############
###########################################

def get_nym_vpn_version(args):
    response = subparser_read(args)
    if args.client == "desktop":
        version = current_desktop_version(args, response)
    elif args.client == "cli":
        version = current_cli_version(args, response)
    else:
        print("Incorrect argument for -c, --client")
        sys.exit(-1)

def current_cli_version(args, response):
    df = pd.DataFrame(response)
    print(df)

    # NEEDS THIS IN PYTHON:
    # current_cli_version=$(curl -s $release_url | jq -r '.[].tag_name' | grep '^nym-vpn-cli-v' | sort -Vr | head -n 1 | awk -F'-v' '{print $NF}')


def current_desktop_version(args, response):
    # NEEDS THIS IN PYTHON:
    df = pd.DataFrame(response)
    print(df)
    # version=$(curl -s $release_url | jq -r '.[].tag_name' | grep '^nym-vpn-desktop-v' | sort -Vr | head -n 1 | awk -F'-v' '{print $NF}')


###########################################
############### MAIN PARSER ###############
###########################################

def parser_main():
    parser = argparse.ArgumentParser(
            prog="Nym API scraper",
            description='''Get any live data from Nyx validator''',
            epilog=''
            )
    subparsers = parser.add_subparsers(help="")
    parser_supply = subparsers.add_parser('supply',
            help='reads API on supply',
            aliases=['s',]
            )

    parser_supply.add_argument(
            "-a","--api",
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

    parser_supply.add_argument(
            "-s", "--separator",
            type=str,
            default=" ",
            help="Add custom thousand separator to --format flag (default is none)"
            )

    parser_supply.set_defaults(func=read_supply)

    parser_time_now = subparsers.add_parser('time_now',
            help='Prints UTC time now',
            aliases=['time', 't']
            )

    parser_time_now.set_defaults(func=print_time_now)


    parser_nym_vpn = subparsers.add_parser('nym_vpn',
            help='reads NymVPN latest version',
            aliases=['n']
            )

    parser_nym_vpn.add_argument(
            "-c","--client",
            type=str,
            default="desktop",
            help="choose: desktop, cli - default: desktop"
            )

    parser_nym_vpn.add_argument(
            "-a","--api",
            type=str,
            default="github",
            help="choose: mainnet, perf, sandbox"
            )

    parser_nym_vpn.add_argument(
            "-e","--endpoint",
            type=str,
            help="add the url suffix",
            default="repos/nymtech/nym-vpn-client/releases"
            )


    parser_nym_vpn.set_defaults(func=get_nym_vpn_version)




    args = parser.parse_args()
    try:
        args.func(args)
    except AttributeError as e:
        print("Error on argparser")
        sys.exit(-1)




if __name__ == "__main__":
    parser_main()
