use kernel::{
    ChainType, ChainstateManager, ContextBuilder,
    core::{BlockSpentOutputsExt, CoinExt, TransactionSpentOutputsExt},
};
use std::{collections::BTreeMap, path::PathBuf};

type Age = u32;
type Count = u64;

fn main() {
    let bitcoin_dir = std::env::var("BITCOIN_DIR").unwrap();
    println!("Using directory {bitcoin_dir}");
    let mut args = std::env::args();
    let _ = args.next().unwrap();
    let mut include_coinbase = false;
    if args.next().is_some() {
        include_coinbase = true;
    }
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
    let mut block_input_ages: BTreeMap<u32, Vec<Age>> = BTreeMap::new();
    let chain = chainman.active_chain();
    for entry in chain.iter() {
        println!(
            "Block hash {} at height {}",
            entry.block_hash(),
            entry.height()
        );
        let curr_height: u32 = entry.height().try_into().unwrap();
        let undo = chainman.read_spent_outputs(&entry).unwrap();
        let mut block_ages = Vec::new();
        for transaction in undo.iter() {
            for coin in transaction.coins() {
                let creation_height = coin.confirmation_height();
                let is_coinbase = coin.is_coinbase();
                if !is_coinbase || include_coinbase {
                    let age = curr_height - creation_height;
                    *ages.entry(age).or_insert(0) += 1;
                    block_ages.push(age);
                }
            }
        }
        block_input_ages.insert(curr_height, block_ages);
        if swiftsync_research::is_reference_height(entry) {
            break;
        }
    }
    println!("Writing coin age counts to CSV");
    write_ages_to_csv(ages);
    println!("Writing block input ages to CSV");
    block_input_ages_to_csv(block_input_ages);
}

fn write_ages_to_csv(ages: BTreeMap<Age, Count>) {
    let mut wtr = csv::Writer::from_path("counts.csv").unwrap();
    wtr.write_record(["age", "count"]).unwrap();
    for (k, v) in ages {
        wtr.write_record([k.to_string(), v.to_string()]).unwrap();
    }
    wtr.flush().unwrap();
}

fn block_input_ages_to_csv(ages: BTreeMap<u32, Vec<Age>>) {
    let mut wtr = csv::Writer::from_path("input_ages.csv").unwrap();
    wtr.write_record(["block", "ages"]).unwrap();
    for (k, v) in ages {
        let str_map = v
            .into_iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("|");
        wtr.write_record([k.to_string(), str_map]).unwrap();
    }
    wtr.flush().unwrap();
}
