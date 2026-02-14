# SwiftSync Protocol Research

Collection of statistics to drive _SwiftSync_ protocol decisions.

## Background

_SwiftSync_ requires the inputs of a block be served by peers. To minimize the bandwidth required to for clients to perform _SwiftSync_, compression techniques are used on each field of the input coins. Further reduction can be achieved with caching strategies.

A concise representation of the UTXO set must also be shared to use _SwiftSync_. Encoding of this file depends largely on the distribution of these coins within historical blocks.

## Binaries

Set your `BITCOIN_DIR` environment variable to an absolute path to your Bitcoin data directory.

- Report the savings due to the `ReconstructableScript` format:

```
export BITCOIN_DIR=/path/to/bitcoin/datadir && cargo run --bin reconstructable_script_savings --release
```

- Report the savings due to the amount compression format:

```
export BITCOIN_DIR=/path/to/bitcoin/datadir && cargo run --bin compressed_amount_savings --release
```

- Count P2PK outputs that are uncompressed:

```
export BITCOIN_DIR=/path/to/bitcoin/datadir && cargo run --bin count_p2pk --release
```

- Analyze the liftime (age), of a coin follows an empirical distribution. To build the `csv` table of coin age to number of occurrences:

```
export BITCOIN_DIR=/path/to/bitcoin/datadir && cargo run --bin compute_coin_ages --release
```

- Plot the results to `plot.png` with an optional upper bound on age.

```
cargo run --bin plot --release 10000 #filter coins with ages older than 10000
```

- Analyze the distribution of coins within blocks, which requires a bitmap to UTXOs in blocks:

```
curl -o bitcoin.hints https://utxohints.store/hints/bitcoin
```

```
cargo run --bin hints --release
```
