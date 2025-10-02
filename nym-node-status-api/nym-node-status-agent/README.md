# Node Status Agent

An agent to run tests and report results back to the Node Status API.

Environment variables that can be set individually are:

- `NYM_NODE_MNEMONICS` - mnemonic to get tickets for tests 
- `NODE_STATUS_AGENT_SERVER_PORT` - Node Status API port
- `NODE_STATUS_AGENT_SERVER_ADDRESS` - Node Status API address

Or use `NODE_STATUS_AGENT_ARGS` to pass your own arguments:

```
NODE_STATUS_AGENT_ARGS="run-probe --server localhost:8000 --mnemonic foo bar baz"
```
