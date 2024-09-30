- what do i need? clients on both sides
  - take a traditional client / server app setup: we need to put the mixnet betwen them to get the privacy properties, and each of them need a client in order to connect and authenticate with the mixnet to send and receive messages: since a lot of the functionality happens client-side and the sphinx of it all

- what dont i need? to run your own infra
  - you can basically use the mixnet as a black box
  - maybe if you're expecting a load of traffic / dont want to rely on other people's gateways, you could run your own and then connect to them specifically (qu: can we configure the client to connect to a particular gateway?)
