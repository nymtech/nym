# Chitchat test

Runs simple chitchat servers, mostly copied over from https://github.com/quickwit-oss/chitchat

## Example

```bash
# Starts 5 servers and joins them into a cluster on localhost ports 10000-10004
# All servers print cluster state on `/` ie 127.0.0.1:10000
# `/docs` endpoint has an open api with a key value setter, set it on one node and observe how the state propagates to the other nodes
# NodeState is a regular BTreeMap
./run-servers.sh

# run killall chitchat-test after you're done, as the servers will continue to run forever in the background
```
