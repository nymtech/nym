# Nyxd Scraper

## Pruning

Similarly to cosmos-sdk, we incorporate pruning into our (scraped) chain data. We attempt to follow their strategies as
closely as possible for convenience's sake. Therefore, the following are available:

### Strategies

The strategies are configured in `config.toml`, with the format `pruning = "<strategy>"` where the options are:

* `default`: only the last 362,880 states(approximately 3.5 weeks worth of state) are kept; pruning at 10 block
  intervals
* `nothing`: all historic states will be saved, nothing will be deleted (i.e. archiving node)
* `everything`: 2 latest states will be kept; pruning at 10 block intervals.
* `custom`: allow pruning options to be manually specified through `pruning.keep_recent`, and `pruning.interval`

### Custom Pruning

These are applied if and only if the pruning strategy is `custom`:

* `pruning.keep_recent`: N means to keep all of the last N blocks
* `pruning.interval`: N means to delete old block data from disk every Nth block.
