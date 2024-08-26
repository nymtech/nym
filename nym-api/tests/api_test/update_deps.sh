#!/bin/bash

set -e

source venv/bin/activate

pip-compile
pip-sync
