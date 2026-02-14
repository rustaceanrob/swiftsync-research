use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;

use kernel::BlockTreeEntry;

const REFERENCE_HEIGHT: i32 = 930_000;

pub mod ef;

pub fn is_reference_height(entry: BlockTreeEntry) -> bool {
    entry.height() == REFERENCE_HEIGHT
}

#[inline]
pub const fn compact_size(value: u64) -> usize {
    match value {
        0..=0xFC => 1,
        0xFD..=0xFFFF => 3,
        0x10000..=0xFFFF_FFFF => 5,
        _ => 9,
    }
}

pub fn compress_amount(mut n: u64) -> u64 {
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

#[inline]
pub fn size_varint(n: u64) -> usize {
    ser_varint(n).len()
}

type BlockHeight = u32;
type FilePos = u64;

#[derive(Debug)]
pub struct BitmapHints {
    map: BTreeMap<BlockHeight, FilePos>,
    file: File,
    stop_height: BlockHeight,
}

impl BitmapHints {
    // # Panics
    //
    // Panics when expected data is not present, or the hintfile overflows the maximum blockheight
    pub fn from_file(mut file: File) -> Self {
        let mut map = BTreeMap::new();
        let mut magic = [0; 4];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(magic, [0x55, 0x54, 0x58, 0x4f]);
        let mut ver = [0; 1];
        file.read_exact(&mut ver).unwrap();
        if u8::from_le_bytes(ver) != 0x00 {
            panic!("Unsupported file version.");
        }
        let mut stop_height = [0; 4];
        file.read_exact(&mut stop_height).expect("empty file");
        let stop_height = BlockHeight::from_le_bytes(stop_height);
        for _ in 1..=stop_height {
            let mut height = [0; 4];
            file.read_exact(&mut height)
                .expect("expected kv pair does not exist.");
            let height = BlockHeight::from_le_bytes(height);
            let mut file_pos = [0; 8];
            file.read_exact(&mut file_pos)
                .expect("expected kv pair does not exist.");
            let file_pos = FilePos::from_le_bytes(file_pos);
            map.insert(height, file_pos);
        }
        Self {
            map,
            file,
            stop_height,
        }
    }

    /// Get the stop height of the hint file.
    pub fn stop_height(&self) -> BlockHeight {
        self.stop_height
    }

    /// # Panics
    ///
    /// If there are no offset present at that height, aka an overflow, or the entry has already
    /// been fetched.
    pub fn get_indexes(&mut self, height: BlockHeight) -> Vec<u16> {
        let file_pos = self
            .map
            .get(&height)
            .cloned()
            .expect("block height overflow");
        self.file
            .seek(SeekFrom::Start(file_pos))
            .expect("missing file position.");
        let mut bits_arr = [0; 4];
        self.file.read_exact(&mut bits_arr).unwrap();
        let mut unspents = Vec::new();
        let num_bits = u32::from_le_bytes(bits_arr);
        let mut curr_byte: u8 = 0;
        for bit_pos in 0..num_bits {
            let leftovers = bit_pos % 8;
            if leftovers == 0 {
                let mut single_byte_arr = [0; 1];
                self.file.read_exact(&mut single_byte_arr).unwrap();
                curr_byte = u8::from_le_bytes(single_byte_arr);
            }
            if ((curr_byte >> leftovers) & 0x01) == 0x01 {
                unspents.push(bit_pos as u16);
            }
        }
        unspents
    }
}
