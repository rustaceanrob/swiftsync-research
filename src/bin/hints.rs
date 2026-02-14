use std::fs::File;

use swiftsync_research::BitmapHints;

fn main() {
    let file = File::open("/home/rob/swiftsync/node/bitcoin.hints").unwrap();
    let mut hints = BitmapHints::from_file(file);
    let mut max_rle = 0;
    let stop = hints.stop_height();
    let mut all_rle = Vec::new();
    let mut maxes = Vec::new();
    for height in 1..stop {
        let indices = hints.get_indexes(height);
        let rle = indices
            .iter()
            .zip(indices.iter().skip(1))
            .map(|(first, second)| second - first)
            .collect::<Vec<u64>>();
        all_rle.extend_from_slice(&rle);
        let max_diff = *rle.iter().max().unwrap_or(&0);
        if max_diff > max_rle {
            max_rle = max_diff;
        }
        maxes.push(max_rle);
    }
    let avg_rle: u64 = all_rle.iter().sum::<u64>() / all_rle.len() as u64;
    let avg_max: u64 = maxes.iter().sum::<u64>() / all_rle.len() as u64;
    maxes.sort_unstable();
    let median_max = maxes[maxes.len() / 2];
    println!("Avg run length encoded {avg_rle}");
    println!("Max run length encoded {max_rle}");
    println!("Avg block max run length length encoded {avg_max}");
    println!("Median block max run length length encoded {median_max}");
}
