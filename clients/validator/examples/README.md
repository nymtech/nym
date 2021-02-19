This is a bunch of scripts for trying out CosmWasm contracts on a (local) blockchain node. 

They work with CosmWasm v0.14.x. To set up a local node, you'll first need to check out the Nym validator source code, build it, and install it into your Go bin directory. Let's put it on your Desktop and install it:

```
cd ~/Desktop
git clone git@github.com:nymtech/nymd.git
cd nymd
git checkout v0.14.1
go build -o nymd -mod=readonly -tags "netgo,ledger" -ldflags '-X github.com/cosmos/cosmos-sdk/version.Name=nymd -X github.com/cosmos/cosmos-sdk/version.AppName=nymd -X github.com/CosmWasm/wasmd/app.NodeDir=.nymd -X github.com/cosmos/cosmos-sdk/version.Version=0.14.1 -X github.com/cosmos/cosmos-sdk/version.Commit=1920f80d181adbeaedac1eeea1c1c6e1704d3e49 -X github.com/CosmWasm/wasmd/app.Bech32Prefix=nym -X "github.com/cosmos/cosmos-sdk/version.BuildTags=netgo,ledger"' -trimpath ./cmd/wasmd
mv nymd $GOBIN
```

That will build and install the `nymd` binary in Go's bin directory; assuming that's in your $PATH, the commands below will work. 

There's an examples folder inside the validator client, which will set up Cosmos-based accounts, then initialize and start a local Cosmos blockchain for development purposes. First, from the top-level `nym` directory:

```
cd clients/validator/examples
```

(Re)generate accounts by running the following. NOTE: it's destructive, it'll wipe your previous wasm accounts!

```
./reset_accounts.sh
```

Init the blockchain running with: 

```
./init.sh
```

Start it with: 

```
./start.sh
```

Congratulations! You now have a running CosmWasm blockchain that you can upload contract code into. 

## Using it in TypeScript

From the `clients/validators/examples` directory:

```
cd nym-driver-example
npm install
npx nodemon --exec ts-node ./index.ts  --watch . --ext .ts
```

That will give you a running daemon that watches for changes in TypeScript files. It will execute a series of actions in the blockchain and explain what it's doing in the console, so you can see how things work.

Running `ts-node` on the index file would do basically the same thing, without a watcher. 


## Hitting the CosmWasm REST API directly:

Let's assume for the moment that the contract address you want to work on is `nym10pyejy66429refv3g35g2t7am0was7ya69su6d`.

Basic information about the contract can be retrieved from the Cosmos REST API's `cosmwasm` module.

The following routes are available:

http://localhost:1317/wasm/contract/nym10pyejy66429refv3g35g2t7am0was7ya69su6d
http://localhost:1317/wasm/contract/nym10pyejy66429refv3g35g2t7am0was7ya69su6d/state
http://localhost:1317/wasm/contract/nym10pyejy66429refv3g35g2t7am0was7ya69su6d/history

### Querying active contracts

There are also two query routes, one for smart queries and one for raw queries based on a key:

http://localhost:1317/wasm/contract/nym10pyejy66429refv3g35g2t7am0was7ya69su6d/smart/{query}
http://localhost:1317/wasm/contract/nym10pyejy66429refv3g35g2t7am0was7ya69su6d/raw/{key}

Let's try using a smart query to retrieve our topology. We know the above URL structure for a smart query. What do we put in the `{query}` parameter? 

Our contract has a `GetTopology {}` query struct (which is annotated as `snake_case` in the contract code, and takes an empty struct as a value), so we can construct the following smart query param:

```
{ "get_topology": {} }
```

HOWEVER

We can't just send this: 

http://localhost:1317/wasm/contract/nym10pyejy66429refv3g35g2t7am0was7ya69su6d/smart/{ "get_topology": {} }

Instead, we need to encode the param `{ "get_topology": {} }` as Base64, and tell the REST API that this is how it's encoded by adding a query string `encoding` parameter, e.g. `?encoding=base64`.

`{ "get_topology": {} }` in Base64 is `eyAiZ2V0X3RvcG9sb2d5Ijoge30gfQ==`. You can get this without making a driver program by going here: 

https://onlineasciitools.com/convert-ascii-to-base64

(and in case you ever want to go the other way and decode some Base64 to verify what you've entered, go here: https://onlineasciitools.com/convert-base64-to-ascii)

The final query we end up with is:

http://localhost:1317/wasm/contract/nym10pyejy66429refv3g35g2t7am0was7ya69su6d/smart/eyAiZ2V0X3RvcG9sb2d5Ijoge30gfQ==?encoding=base64

This works if the topology is small. 

It fails past a certain number of nodes (I'm not sure yet how many).

Note that output is once again in Base64, if you want to decode the output you'll need to do it. 

I got back:

```
{"height":"14036","result":{"smart":"eyJtaXhfbm9kZV9ib25kcyI6W3siYW1vdW50IjpbeyJkZW5vbSI6InVueW0iLCJhbW91bnQiOiIxMDAwMDAwMDAwIn1dLCJvd25lciI6Im55bTFkcWRyZGplOTdmMjZ4aG5xbjk3c2FyMndyNWdqOWQ5NDZ2a3NqayIsIm1peF9ub2RlIjp7Imhvc3QiOiIxOTIuMTY4LjEuMSIsImxheWVyIjoxLCJsb2NhdGlvbiI6InRoZSBpbnRlcm5ldCIsInNwaGlueF9rZXkiOiJteXNwaGlueGtleSIsInZlcnNpb24iOiIwLjkuMiJ9fSx7ImFtb3VudCI6W3siZGVub20iOiJ1bnltIiwiYW1vdW50IjoiMTAwMDAwMDAwMCJ9XSwib3duZXIiOiJueW0xa2tqYTQ5N2NwYzc5ZGwyNDJ3bjdxbHBhOWU4cGxuNGw2dnpyeGMiLCJtaXhfbm9kZSI6eyJob3N0IjoiMTkyLjE2OC4xLjEiLCJsYXllciI6MSwibG9jYXRpb24iOiJ0aGUgaW50ZXJuZXQiLCJzcGhpbnhfa2V5IjoibXlzcGhpbnhrZXkiLCJ2ZXJzaW9uIjoiMC45LjIifX0seyJhbW91bnQiOlt7ImRlbm9tIjoidW55bSIsImFtb3VudCI6IjEwMDAwMDAwMDAifV0sIm93bmVyIjoibnltMWxxaGtwbDlzcHJ3c3A2M2RjNDZldTlrdm11Y2Nna243MzBqOXRsIiwibWl4X25vZGUiOnsiaG9zdCI6IjE5Mi4xNjguMS4xIiwibGF5ZXIiOjEsImxvY2F0aW9uIjoidGhlIGludGVybmV0Iiwic3BoaW54X2tleSI6Im15c3BoaW54a2V5IiwidmVyc2lvbiI6IjAuOS4yIn19LHsiYW1vdW50IjpbeyJkZW5vbSI6InVueW0iLCJhbW91bnQiOiIxMDAwMDAwMDAwIn1dLCJvd25lciI6Im55bTFueTlzY3J6OTJuYWM1bGEzcTc3bXRuZWd6Z2V6YzY1cnZrcGdhYSIsIm1peF9ub2RlIjp7Imhvc3QiOiIxOTIuMTY4LjEuMSIsImxheWVyIjoxLCJsb2NhdGlvbiI6InRoZSBpbnRlcm5ldCIsInNwaGlueF9rZXkiOiJteXNwaGlueGtleSIsInZlcnNpb24iOiIwLjkuMiJ9fV19"}}```

Decoding the big Base64 response string from `smart` gives: 

```
{"mix_node_bonds":[{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1dqdrdje97f26xhnqn97sar2wr5gj9d946vksjk","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1kkja497cpc79dl242wn7qlpa9e8pln4l6vzrxc","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1lqhkpl9sprwsp63dc46eu9kvmuccgkn730j9tl","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1ny9scrz92nac5la3q77mtnegzgezc65rvkpgaa","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}}]}
```

Pretty cool!

```
{"mix_node_bonds":[{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1054nh6a049r2nd73zjyfya6svrrwtwuwplr782","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1332slmq7uqvwrmxvt73zltz0tpdr0pmpge7jnu","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1a4utjp6ysqygt0hdstv9697a5y0ujsg9vvg7m4","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1dqdrdje97f26xhnqn97sar2wr5gj9d946vksjk","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1gem5crjkput2lg7wzpr3jdpver0spazf34fggu","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1kkja497cpc79dl242wn7qlpa9e8pln4l6vzrxc","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1l8yf0qaq39d5xvcmpyx0mhxckn77x8gz4phmjz","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1lctclx3jnskam7llxqdyw0mmfk24znn9lelram","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1lqhkpl9sprwsp63dc46eu9kvmuccgkn730j9tl","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1m2nl7z7adm8ualkkhmpv24fx5ny8c69c38fkjl","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1ny9scrz92nac5la3q77mtnegzgezc65rvkpgaa","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1q5jzekzm8p925hvmwpz0jasd3nzy4tz8h8l32m","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1tauxc4ezv55metq05706actwkptdhsh7qenej7","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1v7a3udzuq7tw3mt926rjtv6yc8pjr2pnmep450","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1vfmv8repca7tl39teqevrg4x0a6ed83t35g7vc","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1xjgyc24v7y7w3gk2kekdngj6zu4dm6ghsc32tu","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1y8dgzduv3lrepvaft44ek6mynjr2fytacs52wh","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1zrlutwux422ks3k06khd3ljtuhfvf9jwzpqgm9","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}},{"amount":[{"denom":"unym","amount":"1000000000"}],"owner":"nym1zw9pdjtpdljhsjmghnz0dp3memcjgapmc3pmvh","mix_node":{"host":"192.168.1.1","layer":1,"location":"the internet","sphinx_key":"mysphinxkey","version":"0.9.2"}}]}```


