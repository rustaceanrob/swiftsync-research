# Bitcoin Coin Age

This crate contains tools for analzying the creation and spending patterns of coins in the bitcoin blockchain.

## Background

The lifetime of a coin, as in the number of blocks between when a coin is created and when it is spent, does not follow a uniform distribution. Most coins are spent within a handful of blocks of when they were created. This pattern, as long as it exists, can be used for database caching.

## Quick Start

To build the `csv` table of coin age to number of occurances, set the `BITCOIN_DIR` to the absolute path of your bitcoin data directory.

`export BITCOIN_DIR=/path/to/bitcoin/datadir && cargo run --bin generate --release`

Plot the results to `plot.png` with an optional upper bound on age.

`cargo run --bin generate --release 10000 #filter coins with ages older than 10000`
