use kernel::{
    ChainType, ChainstateManager, ContextBuilder,
    core::{BlockSpentOutputsExt, CoinExt, ScriptPubkeyExt, TransactionSpentOutputsExt, TxOutExt},
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
    let mut uncompressed_p2pk_count: u64 = 0;
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
                let spk = tx_out.script_pubkey();
                let bytes = spk.to_bytes();
                if bytes.len() != 67 {
                    continue;
                }
                if bytes[0] == 65 && bytes[66] == 0xAC {
                    uncompressed_p2pk_count += 1;
                }
            }
        }
        if swiftsync_research::is_reference_height(entry) {
            break;
        }
    }
    println!("Total uncompressed P2PK {uncompressed_p2pk_count}");
}
