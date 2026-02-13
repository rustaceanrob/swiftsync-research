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
    let mut total_savings_bytes: u128 = 0;
    let mut total_extra: u128 = 0;
    let mut total_p2tr: u128 = 0;
    let mut total_p2wpkh: u128 = 0;
    let mut total_p2wsh: u128 = 0;
    let mut total_p2pk: u128 = 0;
    let mut total_p2pkh: u128 = 0;
    let mut total_p2sh: u128 = 0;
    let mut total_p2pk_uncompressed: u128 = 0;
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
                // P2TR
                if bytes.len() == 34 && bytes[0] == 0x51 && bytes[1] == 0x20 {
                    total_savings_bytes += 1;
                    total_p2tr += 1;
                    continue;
                }
                // P2WPKH
                if bytes.len() == 22 && bytes[0] == 0x00 && bytes[1] == 0x14 {
                    total_savings_bytes += 1;
                    total_p2wpkh += 1;
                    continue;
                }
                // P2WSH
                if bytes.len() == 34 && bytes[0] == 0x00 && bytes[1] == 0x20 {
                    total_savings_bytes += 1;
                    total_p2wsh += 1;
                    continue;
                }
                // P2PK
                if bytes.len() == 35 && bytes[0] == 33 && bytes[34] == 0xAC {
                    total_savings_bytes += 2;
                    total_p2pk += 2;
                    continue;
                }
                // P2PKH
                if bytes.len() == 25
                    && bytes[0] == 0x76
                    && bytes[1] == 0xA9
                    && bytes[2] == 20
                    && bytes[23] == 0x88
                    && bytes[24] == 0xAC
                {
                    total_savings_bytes += 4;
                    total_p2pkh += 4;
                    continue;
                }
                // P2SH
                if bytes.len() == 23 && bytes[0] == 0xA9 && bytes[1] == 0x14 && bytes[22] == 0x87 {
                    total_savings_bytes += 2;
                    total_p2sh += 2;
                    continue;
                }
                // P2PK Uncompressed
                if bytes.len() == 67 && bytes[0] == 65 && bytes[66] == 0xAC {
                    total_savings_bytes += 2;
                    total_p2pk_uncompressed += 2;
                    continue;
                }
                total_extra += 1
            }
        }
        if swiftsync_research::is_reference_height(entry) {
            break;
        }
    }
    println!("Savings: {}MB", total_savings_bytes / 1_000_000);
    println!("P2TR: {}MB", total_p2tr / 1_000_000);
    println!("P2WPKH: {}MB", total_p2wpkh / 1_000_000);
    println!("P2WSH: {}MB", total_p2wsh / 1_000_000);
    println!("P2SH: {}MB", total_p2sh / 1_000_000);
    println!("P2PKH: {}MB", total_p2pkh / 1_000_000);
    println!("P2PK: {}MB", total_p2pk / 1_000_000);
    println!(
        "P2PK Uncompressed: {}MB",
        total_p2pk_uncompressed / 1_000_000
    );
    println!("Count unknown scripts: {}", total_extra);
}
