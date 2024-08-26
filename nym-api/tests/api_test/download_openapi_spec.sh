#!/bin/bash

# TODO dz only for testing, remove:

curl -s http://localhost:8081/api-docs/openapi.json | jq '.' > openapi_axum.json
curl http://localhost:8000/v1/openapi.json -o openapi_rocket.json
