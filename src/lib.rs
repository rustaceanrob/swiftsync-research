use kernel::BlockTreeEntry;

const REFERENCE_HEIGHT: i32 = 930_000;

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

pub fn size_varint(n: u64) -> usize {
    ser_varint(n).len()
}
