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

def get_url(args, **kwargs):
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
    unym = float(unym)
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
    desc_column = _get_desc_column()
    df.insert(1, '**Description**', desc_column, True)
    stake_saturation = _get_stake_saturation()
    df.loc[len(df.index)] = ['Stake Saturation', 'Optimal size of node self-bond + delegation', stake_saturation]
    df['**Amount in NYM**'] = df['**Amount in NYM**'].apply(thousand_separator, args=(separator, ))
    table = df.to_markdown(index=False,colalign=("left","left","right"))
    print(table)

def _get_stake_saturation():
    reward_params = get_api("https://validator.nymtech.net/api/v1/epoch/reward_params")
    stake_saturation = get_dict_value(reward_params,["interval","stake_saturation_point"])
    stake_saturation = convert_u_nym(stake_saturation)
    return stake_saturation

def _get_desc_column():
    supply = "Maximum amount of NYM token in existence"
    reserve = "Tokens releasing for operators rewards"
    vesting = "Tokens locked outside of cicrulation for future claim"
    circulating = "Amount of unlocked tokens"
    desc_column = [supply, reserve, vesting, circulating]
    return desc_column

def read_supply(args):
    separator = args.separator
    response = subparser_read(args)
    if args.endpoint == "circulating-supply":
        if args.value:
            value = get_nested_value(response, args)
            value = convert_u_nym(value)
            value = thousand_separator(value, separator)
            print(value)
        elif args.format == "markdown":
            display_supply_table(response, args)
        else:
            value = response
            print(value)
    elif args.endpoint == "epoch/reward_params":
        value = get_reward_params(response, args, separator)
        print(value)

def get_reward_params(response, args, separator):
    value = get_nested_value(response, args)
    if args.format == "percent":
        value = _return_percent_annotation(value)
    else:
        value = convert_u_nym(value)
        value = thousand_separator(value, separator)
    return value

def get_nested_value(response, args):
    value = response
    for key in args.value:
        value = value[key]
    return value

def _return_percent_annotation(value):
    value = float(value) * 100
    value = f"{value}%"
    return value

###########################################
############# CALCULATE FNs ###############
###########################################

def calculate(args):
    separator = args.separator
    reward_params = get_api("https://validator.nymtech.net/api/v1/epoch/reward_params")
    circulating_supply = get_api("https://validator.nymtech.net/api/v1/circulating-supply")
    if args.staking_target:
        display_staking_target(args, reward_params, circulating_supply, separator)

def get_api(url):
    r = requests.get(url)
    response = r.json()
    return response


def display_staking_target(args, reward_params, circulating_supply, separator):
    keys = ["interval", "staking_supply_scale_factor"]
    staking_supply_scale_factor = get_dict_value(reward_params, keys)
    keys = ["circulating_supply", "amount"]
    circulating_supply = get_dict_value(circulating_supply, keys)
    staking_target = float(staking_supply_scale_factor) * float(circulating_supply)
    staking_target = convert_u_nym(staking_target)
    if args.separator:
        staking_target = thousand_separator(staking_target, separator)
    print(staking_target)

def get_dict_value(json, keys):
    value = json
    for key in keys:
        value = value[key]
    return value


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
    parser_supply = subparsers.add_parser('validator',
            help='Reads validaor API enpoints',
            aliases=['v']
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
            "-v","--value",
            type=str,
            help="dictionary keys to get needed value separated by a space",
            nargs = '+'
            )

    parser_supply.add_argument(
            "-f","--format",
            type=str,
            help="'markdown' formats the output for documentation purpose; 'percent' returns a number with % annotation",
            )

    parser_supply.add_argument(
            "-s", "--separator",
            type=str,
            default=" ",
            help="Add custom thousand separator to --format flag (default is none)"
            )

    parser_supply.set_defaults(func=read_supply)




    parser_calculate = subparsers.add_parser('calculate',
            help='Calculate and print the values of optional args',
            aliases=['c']
            )

    parser_calculate.add_argument(
            "--staking_target",
            action="store_true",
            help="A multiplier of staking supply scale factor and circulating supply"
            )

    parser_calculate.add_argument(
            "-s", "--separator",
            type=str,
            default=" ",
            help="Add custom thousand separator to --format flag (default is none)"
            )
#    parser_calculate.add_argument(
#            "--api",
#            default="mainnet",
#            )

    parser_calculate.set_defaults(func=calculate)

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
