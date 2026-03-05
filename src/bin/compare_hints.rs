use std::fs::File;

use hintsfile::{EliasFano, Hintsfile};
use statrs::function::gamma::ln_gamma;
use swiftsync_research::compact_size;

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

fn size_run_lengths_compact_size(elements: &[u32]) -> usize {
    let mut size = compact_size(elements.len() as u64);
    assert!(elements.is_sorted());
    let mut prev = 0;
    for &element in elements {
        let diff = element - prev;
        size += compact_size(diff as u64);
        prev = element;
    }
    size
}

fn size_run_lengths_varint(elements: &[u32]) -> usize {
    let mut size = swiftsync_research::size_varint(elements.len() as u64);
    assert!(elements.is_sorted());
    let mut prev = 0;
    for &element in elements {
        let diff = element - prev;
        size += swiftsync_research::size_varint(diff as u64);
        prev = element;
    }
    size
}

fn main() {
    let hints_file = std::env::var("HINTS_FILE").unwrap();
    println!("Using hintsfile {hints_file}");
    print!("Generating statistics");
    let hints_file = hints_file.parse::<std::path::PathBuf>().unwrap();
    let mut file = File::open(hints_file).unwrap();
    let hints = Hintsfile::from_reader(&mut file).unwrap();
    let stop = hints.stop_height();
    let mut min_bytes_req = 0.00;
    let mut size_ef = 0;
    let mut size_literal_indices = 0;
    let mut size_rle_compact_size = 0;
    let mut size_rle_varint = 0;
    for height in 1..=stop {
        let indices = hints.indices_at_height(height).unwrap();
        let ef = EliasFano::compress(&indices);
        size_ef += ef.approximate_size();
        size_literal_indices += compact_size(indices.len() as u64) + 2 * indices.len();
        size_rle_compact_size += size_run_lengths_compact_size(&indices);
        size_rle_varint += size_run_lengths_varint(&indices);
        let n = indices.len() as u32;
        let m = indices.iter().max().copied().unwrap_or_default() + 1;
        min_bytes_req += min_bits_permutation(m, n) / 8.0;
        if height % 10_000 == 0 {
            println!("({height}/{stop})");
        }
    }
    println!(">>>");
    println!(
        "Theoretic minimum encoding {:<4} MB",
         min_bytes_req / 1_000_000.
    );
    println!(
        "Size of Elias-Fano encoding {:<4} MB",
        size_ef as f64 / 1_000_000.
    );
    println!(
        "Size of CompactSize encoded run-lengths {:<4} MB",
        size_rle_compact_size as f64 / 1_000_000.
    );
    println!(
        "Size of VarInt encoded run-lengths {:<4} MB",
        size_rle_varint as f64 / 1_000_000.
    );
    println!(
        "Size of encoding indices literally {:<4} MB",
        size_literal_indices as f64 / 1_000_000.
    );
}
