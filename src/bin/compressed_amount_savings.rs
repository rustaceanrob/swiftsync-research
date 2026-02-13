use kernel::{
    ChainType, ChainstateManager, ContextBuilder,
    core::{BlockSpentOutputsExt, CoinExt, TransactionSpentOutputsExt, TxOutExt},
};
use std::path::PathBuf;

fn main() {
    let bitcoin_dir = std::env::var("BITCOIN_DIR").unwrap();
    println!("Using directory {bitcoin_dir}");
    let data_dir = bitcoin_dir.parse::<PathBuf>().unwrap();
    let blocks_dir = data_dir.join("blocks");
    let context = ContextBuilder::new()
        .chain_type(ChainType::Mainnet)
        .build()
        .unwrap();
    let chainman = ChainstateManager::new(
        &context,
        data_dir.to_str().unwrap(),
        blocks_dir.to_str().unwrap(),
    )
    .unwrap();
    chainman.import_blocks().unwrap();
    let mut compressed_amount_savings: u128 = 0;
    let chain = chainman.active_chain();
    for entry in chain.iter() {
        println!(
            "Block hash {} at height {}",
            entry.block_hash(),
            entry.height()
        );
        let block = chainman.read_spent_outputs(&entry).unwrap();
        for output in block.iter() {
            for coin in output.coins() {
                let tx_out = coin.output();
                let amount = tx_out.value() as u64;
                let compressed = swiftsync_research::compress_amount(amount);
                let size_bytes = swiftsync_research::size_varint(compressed);
                if size_bytes < 8 {
                    compressed_amount_savings += 8 - size_bytes as u128;
                }
            }
        }
        if swiftsync_research::is_reference_height(entry) {
            break;
        }
    }
    println!(
        "Total potential compressed amount savings {}GB",
        compressed_amount_savings / 1_000_000_000
    );
}
