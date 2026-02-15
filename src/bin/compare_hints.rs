use std::fs::File;

use hintsfile::EliasFano;
use swiftsync_research::{BitmapHints, compact_size};

fn size_run_lengths_compact_size(elements: &[u16]) -> usize {
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

fn size_run_lengths_varint(elements: &[u16]) -> usize {
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
    let hints_file = hints_file.parse::<std::path::PathBuf>().unwrap();
    let file = File::open(hints_file).unwrap();
    let mut hints = BitmapHints::from_file(file);
    let stop = hints.stop_height();
    let mut size_ef = 0;
    let mut size_literal_indices = 0;
    let mut size_rle_compact_size = 0;
    let mut size_rle_varint = 0;
    for height in 1..stop {
        let indices = hints.get_indexes(height);
        let ef = EliasFano::compress(&indices);
        size_ef += ef.size();
        size_literal_indices += compact_size(indices.len() as u64) + 2 * indices.len();
        size_rle_compact_size += size_run_lengths_compact_size(&indices);
        size_rle_varint += size_run_lengths_varint(&indices);
        if height % 10_000 == 0 {
            println!("({height}/{stop})");
        }
    }
    println!(
        "Size of Elias-Fano encoding {:<4}MB",
        size_ef as f64 / 1_000_000.
    );
    println!(
        "Size of CompactSize encoded run-lengths {:<4}MB",
        size_rle_compact_size as f64 / 1_000_000.
    );
    println!(
        "Size of VarInt encoded run-lengths {:<4}MB",
        size_rle_varint as f64 / 1_000_000.
    );
    println!(
        "Size of bitmap encoded positions {:<4}MB",
        hints.size() as f64 / 1_000_000.
    );
    println!(
        "Size of encoding indices literally {:<4}MB",
        size_literal_indices as f64 / 1_000_000.
    );
}
