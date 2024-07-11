# Validators Troubleshooting

### Common reasons for your validator being jailed

The most common reason for your validator being jailed is that your validator is out of memory because of bloated syslogs.

Running the command `df -H` will return the size of the various partitions of your VPS.

If the `/dev/sda` partition is almost full, try pruning some of the `.gz` syslog archives and restart your validator process.


## Where can I get more help?

The fastest way to reach one of us or get a help from the community, visit our [Telegram Node Setup Help Chat](https://t.me/nymchan_help_chat) or head to our [Discord](https://discord.gg/nymproject).

For more tech heavy question join our [Matrix core community channel](https://matrix.to/#/#general:nymtech.chat), where you can meet other builders and Nym core team members.
