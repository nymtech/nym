#!/bin/bash

clear
curl -s http://localhost:8000/api-docs/openapi.json | jq . > formatted-openapi.json
npx @redocly/cli@latest lint --config .redocly.yaml \
    --skip-rule=operation-2xx-response \


    # --generate-ignore-file
