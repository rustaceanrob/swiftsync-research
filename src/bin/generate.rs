use kernel::{
    ChainType, ChainstateManager, ContextBuilder,
    core::{TransactionExt, TxInExt, TxOutPointExt, TxidExt},
};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

type Age = u32;
type Count = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct OutPoint {
    txid: [u8; 32],
    vout: usize,
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
    let mut ages: BTreeMap<Age, Count> = BTreeMap::new();
    let mut births: HashMap<OutPoint, Age> = HashMap::new();
    let mut total_outputs: u128 = 0;
    let chain = chainman.active_chain();
    for entry in chain.iter() {
        println!(
            "Block hash {} at height {}",
            entry.block_hash(),
            entry.height()
        );
        let curr_height: u32 = entry.height().try_into().unwrap();
        let block = chainman.read_block_data(&entry).unwrap();
        for transaction in block.transactions().skip(1) {
            let txid = transaction.txid().to_bytes();
            for (vout, _) in transaction.outputs().enumerate() {
                total_outputs += 1;
                births.insert(OutPoint { txid, vout }, curr_height);
            }
            for input in transaction.inputs() {
                let outpoint = input.outpoint();
                let txid = outpoint.txid().to_bytes();
                let vout = outpoint.index() as usize;
                if let Some(non_coinbase_birth) = births.remove(&OutPoint { txid, vout }) {
                    *ages.entry(curr_height - non_coinbase_birth).or_insert(0) += 1;
                }
            }
        }
    }
    println!(
        "{}/{} remaining outpoints remain unspent.",
        births.len(),
        total_outputs
    );
    println!("Writing coin ages to CSV");
    write_ages_to_csv(ages);
}

fn write_ages_to_csv(ages: BTreeMap<Age, Count>) {
    let mut wtr = csv::Writer::from_path("ages.csv").unwrap();
    wtr.write_record(["age", "count"]).unwrap();
    for (k, v) in ages {
        wtr.write_record([k.to_string(), v.to_string()]).unwrap();
    }
    wtr.flush().unwrap();
}
