#!/bin/bash

curl -s http://localhost:8000/api-docs/openapi.json | jq . > formatted-openapi.json
npx @redocly/cli@latest lint --config .redocly.yaml \
    # --generate-ignore-file
