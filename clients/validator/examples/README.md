This is a bunch of scripts for trying out CosmWasm contracts on a (local) blockchain node. 

They work with CosmWasm v0.14.x. To set up a local node, you'll first need to check out the Nym validator source code, build it, and install it into your Go bin directory. Let's put it on your Desktop and install it:

```
cd ~/Desktop
git clone git@github.com:CosmWasm/wasmd.git
cd wasmd
git checkout v0.14.1
go build -o nymd -mod=readonly -tags "netgo,ledger" -ldflags '-X github.com/cosmos/cosmos-sdk/version.Name=nymd -X github.com/cosmos/cosmos-sdk/version.AppName=nymd -X github.com/CosmWasm/wasmd/app.NodeDir=.nymd -X github.com/cosmos/cosmos-sdk/version.Version=0.14.1 -X github.com/cosmos/cosmos-sdk/version.Commit=5828347c67a16754a43608d00746df55e416a5ec -X github.com/CosmWasm/wasmd/app.Bech32Prefix=nym -X "github.com/cosmos/cosmos-sdk/version.BuildTags=netgo,ledger"' -trimpath ./cmd/wasmd
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