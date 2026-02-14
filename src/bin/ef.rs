use std::fs::File;

use swiftsync_research::{BitmapHints, ef::EliasFano};

fn main() {
    let hints_file = std::env::var("HINTS_FILE").unwrap();
    println!("Using hintsfile {hints_file}");
    let hints_file = hints_file.parse::<std::path::PathBuf>().unwrap();
    let file = File::open(hints_file).unwrap();
    let mut hints = BitmapHints::from_file(file);
    let mut ef_file = File::create("ef.bin").unwrap();
    let stop = hints.stop_height();
    for height in 1..stop {
        let indices = hints.get_indexes(height);
        let enc = EliasFano::new(&indices);
        enc.serialize(&mut ef_file).unwrap();
    }
}
