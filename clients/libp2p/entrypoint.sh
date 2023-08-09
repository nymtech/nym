#!/bin/bash
set -e
printenv NYM_ID
if [ "$NYM_ID" = 'default' ]; then
    echo "No NYM_ID value provided. Exiting."
    exit 1
fi

nym-client init --host '0.0.0.0' --id "$NYM_ID" 
nym-client run --host '0.0.0.0' --id "$NYM_ID" 
