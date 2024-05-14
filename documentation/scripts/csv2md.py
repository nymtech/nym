#!/usr/bin/python3

"""CLI to display .csv files as markdown"""

import argparse
import pandas as pd
import sys
import pathlib
import csv

def create_table(args):
    """Imports csv and creates a table"""
    file = args.file
    csv = pd.read_csv(file)
    if args.sort != None:
        csv = csv.sort_values(csv.columns[args.sort])
        print(csv.columns[args.sort])
    if args.table:
        table = csv.to_markdown(tablefmt="grid", index=args.index)
    else:
        table = csv.to_markdown(index=args.index)
    return table

def display_file(args):
    """Display csv file as a table"""
    table = create_table(args)
    print(table)

def panic(msg):
    """Error message print"""
    print(f"error: {msg}", file=sys.stderr)
    sys.exit(-1)

def parser_main():
    """Main function initializing ArgumentParser, storing arguments and executing commands."""
    # Top level parser
    parser = argparse.ArgumentParser(
            prog='CSV2MD',
            description='''Displays .csv files in markdown''',
            epilog='''Code is power!'''
        )

    # Parser arguments
    parser.add_argument("-V","--version", action="version", version='%(prog)s 1.1.0')
    parser.add_argument("file", help="path/to/file.csv")
    parser.add_argument("-t","--table", default=False, action="store_true", help="output with a tabulate option for terminal reading - does not render in mdbook")
    parser.add_argument("-i","--index", default=False, action="store_true", help="output with an index column")
    parser.add_argument("-s","--sort", type=int, help="supply with column index to sort your output accordingly (ascending way)")

    parser.set_defaults(func=display_file)
    args = parser.parse_args()

    try:
        args.func(args)
    except AttributeError as e:
        msg = f"{e}.\nPlease run with --help or read the error message in case your .csv file is corrupted."
        panic(msg)


if __name__ == '__main__':
    parser_main()
