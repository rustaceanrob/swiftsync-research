// src/lib.rs

use std::io::{self, Read, Write};

/// Elias-Fano encoded representation of a sorted sequence of unique u16 values.
/// Uses u8-packed bitvectors to minimize wasted trailing bits.
#[derive(Debug, Clone)]
pub struct EliasFano {
    n: u16,
    max_value: u16,
    lower_bits: u8,
    lower: Vec<u8>,
    upper: Vec<u8>,
}

impl EliasFano {
    pub fn new(values: &[u16]) -> Self {
        assert!(
            values.len() <= u16::MAX as usize + 1,
            "Cannot encode more than 65536 u16 values"
        );

        let n = values.len() as u16;

        if n == 0 {
            return EliasFano {
                n: 0,
                max_value: 0,
                lower_bits: 0,
                lower: Vec::new(),
                upper: Vec::new(),
            };
        }

        for i in 1..values.len() {
            assert!(
                values[i] > values[i - 1],
                "Input must be strictly sorted (no duplicates)"
            );
        }

        let max_value = values[values.len() - 1];
        let universe = max_value as u32 + 1;
        let count = n as u32;

        let lower_bits = if universe <= count {
            0
        } else {
            (universe / count).ilog2() as u8
        };

        let lower = Self::build_lower(values, lower_bits);
        let upper = Self::build_upper(values, lower_bits, count);

        EliasFano {
            n,
            max_value,
            lower_bits,
            lower,
            upper,
        }
    }

    fn build_lower(values: &[u16], lower_bits: u8) -> Vec<u8> {
        if lower_bits == 0 {
            return Vec::new();
        }

        let total_bits = values.len() as u64 * lower_bits as u64;
        let num_bytes = ((total_bits + 7) / 8) as usize;
        let mut lower = vec![0u8; num_bytes];
        let mask: u16 = (1u16 << lower_bits) - 1;

        for (i, &val) in values.iter().enumerate() {
            let low = val & mask;
            let bit_pos = i as u64 * lower_bits as u64;
            set_bits_u8(&mut lower, bit_pos, low as u32, lower_bits);
        }

        lower
    }

    fn build_upper(values: &[u16], lower_bits: u8, n: u32) -> Vec<u8> {
        let max_upper = values[values.len() - 1] as u64 >> lower_bits;
        let total_bits = n as u64 + max_upper + 1;
        let num_bytes = ((total_bits + 7) / 8) as usize;
        let mut upper = vec![0u8; num_bytes];

        for (i, &val) in values.iter().enumerate() {
            let high = (val as u64) >> lower_bits;
            let pos = high + i as u64;
            let byte_idx = (pos / 8) as usize;
            let bit_idx = (pos % 8) as u32;
            upper[byte_idx] |= 1u8 << bit_idx;
        }

        upper
    }

    pub fn len(&self) -> u16 {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    pub fn contains(&self, value: u16) -> bool {
        if self.n == 0 || value > self.max_value {
            return false;
        }

        let high = (value as u32) >> self.lower_bits;
        let low = if self.lower_bits == 0 {
            0u32
        } else {
            value as u32 & ((1u32 << self.lower_bits) - 1)
        };

        let start = if high == 0 {
            0u32
        } else {
            self.select0_upper(high - 1) + 1
        };

        let end_pos = self.upper.len() as u32 * 8;
        let mut pos = start;

        loop {
            if pos >= end_pos {
                return false;
            }

            let byte_idx = (pos / 8) as usize;
            let bit_idx = pos % 8;
            let bit = (self.upper[byte_idx] >> bit_idx) & 1;

            if bit == 0 {
                return false;
            }

            let elem_idx = (pos - high) as u16;
            let elem_low = self.get_lower(elem_idx);

            if elem_low == low {
                return true;
            }
            if elem_low > low {
                return false;
            }

            pos += 1;
        }
    }

    pub fn get(&self, index: u16) -> Option<u16> {
        if index >= self.n {
            return None;
        }

        let low = self.get_lower(index);
        let high = self.get_upper(index);

        Some(((high << self.lower_bits) | low) as u16)
    }

    fn get_lower(&self, index: u16) -> u32 {
        if self.lower_bits == 0 {
            return 0;
        }

        let bit_pos = index as u64 * self.lower_bits as u64;
        get_bits_u8(&self.lower, bit_pos, self.lower_bits)
    }

    fn get_upper(&self, index: u16) -> u32 {
        let pos = self.select1_upper(index as u32);
        pos - index as u32
    }

    fn select1_upper(&self, rank: u32) -> u32 {
        let mut remaining = rank;
        let mut bit_pos: u32 = 0;

        for &byte in &self.upper {
            let popcount = byte.count_ones();
            if remaining < popcount {
                return bit_pos + select_in_byte(byte, remaining);
            }
            remaining -= popcount;
            bit_pos += 8;
        }

        panic!("select1_upper: rank {} out of bounds", rank);
    }

    fn select0_upper(&self, rank: u32) -> u32 {
        let mut remaining = rank;
        let mut bit_pos: u32 = 0;

        for &byte in &self.upper {
            let zeros = (!byte).count_ones();
            if remaining < zeros {
                return bit_pos + select_in_byte(!byte, remaining);
            }
            remaining -= zeros;
            bit_pos += 8;
        }

        panic!("select0_upper: rank {} out of bounds", rank);
    }

    pub fn iter(&self) -> EliasFanoIterator<'_> {
        EliasFanoIterator {
            ef: self,
            index: 0,
            upper_bit_pos: 0,
            zeros_seen: 0,
        }
    }

    /// Serialize to a writer.
    ///
    /// Layout (all little-endian):
    /// - `n`:          2 bytes (u16)
    /// - `max_value`:  2 bytes (u16)  — omitted if n == 0
    /// - `lower_bits`: 1 byte  (u8)   — omitted if n == 0
    /// - `lower`:      raw bytes, length = ceil(n * lower_bits / 8)
    /// - `upper`:      raw bytes, length = ceil((n + (max_value >> lower_bits) + 1) / 8)
    pub fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.n.to_le_bytes())?;

        if self.n == 0 {
            return Ok(());
        }

        writer.write_all(&[self.lower_bits])?;
        writer.write_all(&self.lower)?;
        writer.write_all(&self.upper)?;

        Ok(())
    }

    pub fn deserialize<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut buf2 = [0u8; 2];
        reader.read_exact(&mut buf2)?;
        let n = u16::from_le_bytes(buf2);

        if n == 0 {
            return Ok(EliasFano {
                n: 0,
                max_value: 0,
                lower_bits: 0,
                lower: Vec::new(),
                upper: Vec::new(),
            });
        }

        reader.read_exact(&mut buf2)?;
        let max_value = u16::from_le_bytes(buf2);

        let mut buf1 = [0u8; 1];
        reader.read_exact(&mut buf1)?;
        let lower_bits = buf1[0];

        let lower_total_bits = n as u64 * lower_bits as u64;
        let lower_len = ((lower_total_bits + 7) / 8) as usize;

        let max_upper = (max_value as u64) >> lower_bits;
        let upper_total_bits = n as u64 + max_upper + 1;
        let upper_len = ((upper_total_bits + 7) / 8) as usize;

        let mut lower = vec![0u8; lower_len];
        reader.read_exact(&mut lower)?;

        let mut upper = vec![0u8; upper_len];
        reader.read_exact(&mut upper)?;

        Ok(EliasFano {
            n,
            max_value,
            lower_bits,
            lower,
            upper,
        })
    }

    pub fn save_to_file(&self, path: &str) -> io::Result<()> {
        let mut file = std::fs::File::create(path)?;
        self.serialize(&mut file)
    }

    pub fn load_from_file(path: &str) -> io::Result<Self> {
        let mut file = std::fs::File::open(path)?;
        Self::deserialize(&mut file)
    }

    pub fn serialized_size(&self) -> usize {
        if self.n == 0 {
            return 2;
        }
        2 + 2 + 1 + self.lower.len() + self.upper.len()
    }

    pub fn size_in_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.lower.len() + self.upper.len()
    }
}

/// Write `width` bits from `value` into a u8 slice at the given bit position.
fn set_bits_u8(buf: &mut [u8], bit_pos: u64, value: u32, width: u8) {
    let mut remaining = width;
    let mut val = value;
    let mut pos = bit_pos;

    while remaining > 0 {
        let byte_idx = (pos / 8) as usize;
        let bit_idx = (pos % 8) as u8;
        let fits = (8 - bit_idx).min(remaining);
        let mask = ((1u32 << fits) - 1) as u8;

        buf[byte_idx] |= ((val as u8) & mask) << bit_idx;

        val >>= fits;
        pos += fits as u64;
        remaining -= fits;
    }
}

/// Read `width` bits from a u8 slice at the given bit position.
fn get_bits_u8(buf: &[u8], bit_pos: u64, width: u8) -> u32 {
    let mut remaining = width;
    let mut result: u32 = 0;
    let mut pos = bit_pos;
    let mut shift = 0u8;

    while remaining > 0 {
        let byte_idx = (pos / 8) as usize;
        let bit_idx = (pos % 8) as u8;
        let fits = (8 - bit_idx).min(remaining);
        let mask = ((1u32 << fits) - 1) as u8;

        let bits = (buf[byte_idx] >> bit_idx) & mask;
        result |= (bits as u32) << shift;

        shift += fits;
        pos += fits as u64;
        remaining -= fits;
    }

    result
}

/// Select the `rank`-th (0-based) set bit within a byte.
fn select_in_byte(byte: u8, rank: u32) -> u32 {
    let mut remaining = rank;
    let mut b = byte;
    let mut pos = 0u32;

    loop {
        debug_assert!(b != 0, "select_in_byte: not enough bits set");
        let tz = b.trailing_zeros();
        if remaining == 0 {
            return pos + tz;
        }
        remaining -= 1;
        let skip = tz + 1;
        b >>= skip;
        pos += skip;
    }
}

pub struct EliasFanoIterator<'a> {
    ef: &'a EliasFano,
    index: u16,
    upper_bit_pos: u32,
    zeros_seen: u32,
}

impl<'a> Iterator for EliasFanoIterator<'a> {
    type Item = u16;

    fn next(&mut self) -> Option<u16> {
        if self.index >= self.ef.n {
            return None;
        }

        loop {
            let byte_idx = (self.upper_bit_pos / 8) as usize;
            let bit_idx = self.upper_bit_pos % 8;

            if byte_idx >= self.ef.upper.len() {
                return None;
            }

            let bit = (self.ef.upper[byte_idx] >> bit_idx) & 1;
            self.upper_bit_pos += 1;

            if bit == 0 {
                self.zeros_seen += 1;
            } else {
                let high = self.zeros_seen;
                let low = self.ef.get_lower(self.index);
                let value = ((high << self.ef.lower_bits) | low) as u16;
                self.index += 1;
                return Some(value);
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.ef.n - self.index) as usize;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for EliasFanoIterator<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let ef = EliasFano::new(&[]);
        assert!(ef.is_empty());
        assert_eq!(ef.len(), 0);
        assert_eq!(ef.get(0), None);
        assert!(!ef.contains(0));
        assert_eq!(ef.iter().collect::<Vec<_>>(), Vec::<u16>::new());
        assert_eq!(ef.serialized_size(), 2);
    }

    #[test]
    fn test_single_element() {
        let ef = EliasFano::new(&[42]);
        assert_eq!(ef.len(), 1);
        assert_eq!(ef.get(0), Some(42));
        assert!(ef.contains(42));
        assert!(!ef.contains(41));
    }

    #[test]
    fn test_basic_sequence() {
        let values: Vec<u16> = vec![2, 3, 5, 7, 11, 13, 24];
        let ef = EliasFano::new(&values);

        for (i, &v) in values.iter().enumerate() {
            assert_eq!(ef.get(i as u16), Some(v));
            assert!(ef.contains(v));
        }

        assert!(!ef.contains(0));
        assert!(!ef.contains(4));
        assert!(!ef.contains(25));
    }

    #[test]
    fn test_iterator() {
        let values: Vec<u16> = vec![0, 1, 5, 10, 100, 1000, 10000];
        let ef = EliasFano::new(&values);
        let collected: Vec<u16> = ef.iter().collect();
        assert_eq!(collected, values);
    }

    #[test]
    fn test_max_u16() {
        let values: Vec<u16> = vec![0, 1000, 30000, 65534, 65535];
        let ef = EliasFano::new(&values);

        for (i, &v) in values.iter().enumerate() {
            assert_eq!(ef.get(i as u16), Some(v));
            assert!(ef.contains(v));
        }

        assert!(!ef.contains(1));
        assert!(!ef.contains(65533));

        let collected: Vec<u16> = ef.iter().collect();
        assert_eq!(collected, values);
    }

    #[test]
    fn test_consecutive() {
        let values: Vec<u16> = (0..100).collect();
        let ef = EliasFano::new(&values);

        for &v in &values {
            assert!(ef.contains(v));
        }
        assert!(!ef.contains(100));

        let collected: Vec<u16> = ef.iter().collect();
        assert_eq!(collected, values);
    }

    #[test]
    fn test_sparse() {
        let values: Vec<u16> = vec![100, 20000, 40000, 60000, 65535];
        let ef = EliasFano::new(&values);

        for (i, &v) in values.iter().enumerate() {
            assert_eq!(ef.get(i as u16), Some(v));
            assert!(ef.contains(v));
        }

        assert!(!ef.contains(0));
        assert!(!ef.contains(30000));
    }

    #[test]
    fn test_contains_exhaustive_small() {
        let values: Vec<u16> = vec![3, 7, 15, 20];
        let ef = EliasFano::new(&values);

        for v in 0..=25u16 {
            assert_eq!(ef.contains(v), values.contains(&v));
        }
    }

    #[test]
    fn test_serialize_deserialize() {
        let values: Vec<u16> = vec![2, 3, 5, 7, 11, 13, 24, 100, 9999];
        let ef = EliasFano::new(&values);

        let mut buffer = Vec::new();
        ef.serialize(&mut buffer).unwrap();
        assert_eq!(buffer.len(), ef.serialized_size());

        let mut cursor = io::Cursor::new(buffer);
        let ef2 = EliasFano::deserialize(&mut cursor).unwrap();

        assert_eq!(ef2.len(), ef.len());
        for i in 0..ef.len() {
            assert_eq!(ef2.get(i), ef.get(i));
        }
    }

    #[test]
    fn test_serialize_empty() {
        let ef = EliasFano::new(&[]);
        let mut buffer = Vec::new();
        ef.serialize(&mut buffer).unwrap();
        assert_eq!(buffer.len(), 2);

        let mut cursor = io::Cursor::new(buffer);
        let ef2 = EliasFano::deserialize(&mut cursor).unwrap();
        assert!(ef2.is_empty());
    }

    #[test]
    fn test_file_round_trip() {
        let values: Vec<u16> = vec![10, 20, 30, 40, 50, 1000, 2000, 50000];
        let ef = EliasFano::new(&values);

        let path = "/tmp/test_elias_fano_u8_words.bin";
        ef.save_to_file(path).unwrap();

        let ef2 = EliasFano::load_from_file(path).unwrap();
        let collected: Vec<u16> = ef2.iter().collect();
        assert_eq!(collected, values);

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_u8_saves_space_vs_u64() {
        // With u64 words: each array rounds up to 8-byte boundary
        // With u8 words: each array rounds up to 1-byte boundary
        let values: Vec<u16> = vec![1, 5, 10];
        let ef = EliasFano::new(&values);

        let lower_bits_total = 3u64 * ef.lower_bits as u64;
        let max_upper = (10u64) >> ef.lower_bits;
        let upper_bits_total = 3u64 + max_upper + 1;

        let u8_bytes = ((lower_bits_total + 7) / 8) + ((upper_bits_total + 7) / 8);
        let u64_bytes = ((lower_bits_total + 63) / 64) * 8 + ((upper_bits_total + 63) / 64) * 8;

        println!(
            "u8 words: {} data bytes, u64 words: {} data bytes, saved: {}",
            u8_bytes,
            u64_bytes,
            u64_bytes - u8_bytes
        );

        assert!(u8_bytes <= u64_bytes);
    }

    #[test]
    fn test_compression_ratio() {
        let values: Vec<u16> = (0..1000).map(|i| i * 65).collect();
        let ef = EliasFano::new(&values);

        let raw_bytes = values.len() * 2;
        let disk_bytes = ef.serialized_size();

        println!(
            "Raw: {} bytes, Serialized: {} bytes ({:.1}%)",
            raw_bytes,
            disk_bytes,
            disk_bytes as f64 / raw_bytes as f64 * 100.0
        );

        let collected: Vec<u16> = ef.iter().collect();
        assert_eq!(collected, values);
    }

    #[test]
    #[should_panic(expected = "strictly sorted")]
    fn test_unsorted_panics() {
        EliasFano::new(&[5, 3, 1]);
    }

    #[test]
    #[should_panic(expected = "strictly sorted")]
    fn test_duplicates_panic() {
        EliasFano::new(&[1, 1, 2, 3]);
    }
}
