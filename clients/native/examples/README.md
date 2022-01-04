# Nym Desktop Client Code Examples 

This directory contains example code for javascript, python, and rust. Please note that **none of these examples are production-ready**, and are included really just to show how the client interacts with the Nym system. 

> Make sure that you have an instance of the `nym-client` set up and running locally on port 1977 before trying to run any of these examples. 

## Go
There are two examples here: 
* `binarysend`, which (as the name suggests) sends a binary file, and 
* `textsend` which sends a raw text file. 

Both examples send these files over the Nym mixnet to the address of your running Nym client, logging information in your terminal window. 

## Javascript 
The example included here starts a websocket server on port 8888, the ui of which can be used to send strings over the Nym mixnet to the address of your running Nym client. 

### Prerequisites
* Reasonably up to date `NodeJS` & `npm` (>= ~v12)

### Running it
Run the following commands:

```
# install dependencies
npm install
# start a webserver on port 8888 
npm start 
```

Then open your browser to `localhost:8888`. 

## Python 
There are two examples here: 
* `binarysend`, which (as the name suggests) sends a binary file, and 
* `textsend` which sends a raw text file. 

Both examples send these files over the Nym mixnet to the address of your running Nym client, logging information in your terminal window. 

Make sure that you have an instance of the `nym-client` set up and running locally on port 1977 before trying to run either of the example scripts. 

## Rust 
There are two examples here: 
* `binarysend`, which (as the name suggests) sends a binary file, and 
* `textsend` which sends a raw text file. 

Both examples send these files over the Nym mixnet to the address of your running Nym client, logging information in your terminal window. 

Make sure that you have an instance of the `nym-client` set up and running locally on port 1977 before trying to run either of the example scripts. 