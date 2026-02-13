use kernel::{
    ChainType, ChainstateManager, ContextBuilder,
    core::{BlockSpentOutputsExt, CoinExt, TransactionSpentOutputsExt, TxOutExt},
};
use std::path::PathBuf;

fn compress_amount(mut n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut e: u64 = 0;
    while (n % 10) == 0 && e < 9 {
        n /= 10;
        e += 1;
    }
    if e < 9 {
        let d = n % 10;
        assert!((1..=9).contains(&d));
        n /= 10;
        1 + (n * 9 + d - 1) * 10 + e
    } else {
        1 + (n - 1) * 10 + 9
    }
}

fn ser_varint(n: u64) -> Vec<u8> {
    let mut tmp = Vec::new();
    let mut l = n;
    loop {
        let has_more = !tmp.is_empty();
        let byte = (l & 0x7f) as u8 | if has_more { 0x80 } else { 0x00 };
        tmp.push(byte);
        if l <= 0x7f {
            break;
        }
        l = (l >> 7) - 1;
    }
    tmp.reverse();
    tmp
}

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
                let compressed = compress_amount(amount);
                let size_bytes = ser_varint(compressed);
                if size_bytes.len() < 8 {
                    compressed_amount_savings += 8 - size_bytes.len() as u128;
                }
            }
        }
        if swiftsync_research::is_reference_height(entry) {
            break;
        }
    }
    println!("Total potential compressed amount savings {compressed_amount_savings}");
}
