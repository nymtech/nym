# Troubleshooting

## Lots of `duplicate fragment received` messages
You might see a lot of `WARN` level logs about duplicate fragments in your logs, depending on the log level you're using. This occurs when a packet is retransmitted somewhere in the Mixnet, but then the original makes it to the destination client as well. This is not something to do with your client logic, but instead the state of the Mixnet.
