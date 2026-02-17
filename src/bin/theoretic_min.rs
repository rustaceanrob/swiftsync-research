use statrs::function::gamma::ln_gamma;
use std::fs::File;

use swiftsync_research::BitmapHints;

#[allow(clippy::excessive_precision)]
const LN_2: f64 = 0.693147180559945309417232121458176568_f64;

fn min_bits_permutation(m: u32, n: u32) -> f64 {
    assert!(n <= m, "n must be <= m");
    if n == 0 || n == m {
        return 0.0;
    }

    let log2_perm =
        ln_gamma((m + 1) as f64) - ln_gamma((n + 1) as f64) - ln_gamma((m - n + 1) as f64);

    log2_perm / LN_2
}

fn main() {
    let hints_file = std::env::var("HINTS_FILE").unwrap();
    println!("Using hintsfile {hints_file}");
    let hints_file = hints_file.parse::<std::path::PathBuf>().unwrap();
    let file = File::open(hints_file).unwrap();
    let mut hints = BitmapHints::from_file(file);
    let stop = hints.stop_height();
    let mut min_bytes_req = 0.00;
    println!("Computing estimate. This will take a minute...");
    for entry_height in 1..=stop {
        if entry_height == 0 {
            continue;
        }
        let indices = hints.get_indexes(entry_height);
        let n = indices.len() as u32;
        let m = indices.iter().max().copied().unwrap_or_default() + 1;
        min_bytes_req += min_bits_permutation(m, n) / 8.0;
        if entry_height == stop {
            break;
        }
    }
    println!(
        "Computed theoretical minimum size required {}MB",
        min_bytes_req / 1_000_000.
    );
}
