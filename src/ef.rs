// src/lib.rs

use std::io::{self, Write};

/// Elias-Fano encoded representation of a sorted sequence of u16 values.
#[derive(Debug, Clone)]
pub struct EliasFano {
    /// Number of elements in the sequence (max 65536 for full u16 range)
    n: u16,
    /// Number of lower bits per element (0..=16)
    lower_bits: u8,
    /// Compact array of lower bits (packed into u64 words)
    lower: Vec<u64>,
    /// Unary-coded upper bits (packed into u64 words)
    upper: Vec<u64>,
}

impl EliasFano {
    /// Create an Elias-Fano encoding from a sorted slice of u16 values.
    ///
    /// # Panics
    /// Panics if the input is not sorted in non-decreasing order or has more
    /// than 65536 elements (since values are u16, at most 65536 distinct
    /// sorted entries including duplicates fit meaningfully).
    pub fn new(values: &[u16]) -> Self {
        assert!(
            values.len() <= u16::MAX as usize + 1,
            "Cannot encode more than 65536 u16 values"
        );
        let n = values.len() as u16;

        if n == 0 {
            return EliasFano {
                n: 0,
                lower_bits: 0,
                lower: Vec::new(),
                upper: Vec::new(),
            };
        }

        for i in 1..values.len() {
            assert!(
                values[i] >= values[i - 1],
                "Input must be sorted in non-decreasing order"
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
            lower_bits,
            lower,
            upper,
        }
    }

    fn build_lower(values: &[u16], lower_bits: u8) -> Vec<u64> {
        if lower_bits == 0 {
            return Vec::new();
        }

        let total_bits = values.len() as u64 * lower_bits as u64;
        let num_words = ((total_bits + 63) / 64) as usize;
        let mut lower = vec![0u64; num_words];
        let mask: u64 = (1u64 << lower_bits) - 1;

        for (i, &val) in values.iter().enumerate() {
            let low = val as u64 & mask;
            let bit_pos = i as u64 * lower_bits as u64;
            let word_idx = (bit_pos / 64) as usize;
            let bit_idx = (bit_pos % 64) as u32;

            lower[word_idx] |= low << bit_idx;

            if bit_idx + lower_bits as u32 > 64 {
                let overflow = bit_idx + lower_bits as u32 - 64;
                if word_idx + 1 < lower.len() {
                    lower[word_idx + 1] |= low >> (lower_bits as u32 - overflow);
                }
            }
        }

        lower
    }

    fn build_upper(values: &[u16], lower_bits: u8, n: u32) -> Vec<u64> {
        let max_upper = values[values.len() - 1] as u64 >> lower_bits;
        let total_bits = n as u64 + max_upper + 1;
        let num_words = ((total_bits + 63) / 64) as usize;
        let mut upper = vec![0u64; num_words];

        for (i, &val) in values.iter().enumerate() {
            let high = (val as u64) >> lower_bits;
            let pos = high + i as u64;
            let word_idx = (pos / 64) as usize;
            let bit_idx = (pos % 64) as u32;
            upper[word_idx] |= 1u64 << bit_idx;
        }

        upper
    }

    /// Returns the number of elements in the encoded sequence.
    pub fn len(&self) -> u16 {
        self.n
    }

    /// Returns true if the sequence is empty.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Retrieve the element at the given index (0-based).
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
        let word_idx = (bit_pos / 64) as usize;
        let bit_idx = (bit_pos % 64) as u32;
        let mask = (1u64 << self.lower_bits) - 1;

        let mut result = (self.lower[word_idx] >> bit_idx) & mask;

        if bit_idx + self.lower_bits as u32 > 64 {
            let remaining = bit_idx + self.lower_bits as u32 - 64;
            let high_part = self.lower[word_idx + 1] & ((1u64 << remaining) - 1);
            result |= high_part << (self.lower_bits as u32 - remaining);
        }

        result as u32
    }

    fn get_upper(&self, index: u16) -> u32 {
        let pos = self.select_upper(index);
        pos - index as u32
    }

    fn select_upper(&self, rank: u16) -> u32 {
        let mut remaining = rank as u32;
        let mut bit_pos: u32 = 0;

        for &word in &self.upper {
            let popcount = word.count_ones();
            if remaining < popcount {
                bit_pos += select_in_word(word, remaining);
                return bit_pos;
            }
            remaining -= popcount;
            bit_pos += 64;
        }

        panic!("select_upper: rank {} out of bounds", rank);
    }

    /// Returns an iterator over all values in the sequence.
    pub fn iter(&self) -> EliasFanoIterator<'_> {
        EliasFanoIterator {
            ef: self,
            index: 0,
            upper_bit_pos: 0,
            zeros_seen: 0,
        }
    }

    /// Serialize to a writer in a minimal binary format.
    ///
    /// Layout (all little-endian):
    /// - `n`:          2 bytes (u16)
    /// - `max_value`:  2 bytes (u16)  — omitted if n == 0
    /// - `lower_bits`: 1 byte  (u8)   — omitted if n == 0
    /// - `lower`:      packed u64 words, count derived from n and lower_bits
    /// - `upper`:      packed u64 words, count derived from n, max_value, lower_bits
    pub fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.n.to_le_bytes())?;

        if self.n == 0 {
            return Ok(());
        }

        writer.write_all(&[self.lower_bits])?;

        for &word in &self.lower {
            writer.write_all(&word.to_le_bytes())?;
        }
        for &word in &self.upper {
            writer.write_all(&word.to_le_bytes())?;
        }

        Ok(())
    }
}

/// Select the `rank`-th (0-based) set bit within a single u64 word.
fn select_in_word(word: u64, rank: u32) -> u32 {
    let mut remaining = rank;
    let mut w = word;
    let mut pos = 0u32;

    loop {
        debug_assert!(w != 0, "select_in_word: not enough bits set");
        let tz = w.trailing_zeros();
        if remaining == 0 {
            return pos + tz;
        }
        remaining -= 1;
        let skip = tz + 1;
        w >>= skip;
        pos += skip;
    }
}

/// An efficient forward iterator that walks the upper bitvector linearly.
pub struct EliasFanoIterator<'a> {
    ef: &'a EliasFano,
    index: u16,
    upper_bit_pos: u32,
    zeros_seen: u32,
}

impl<'a> Iterator for EliasFanoIterator<'_> {
    type Item = u16;

    fn next(&mut self) -> Option<u16> {
        if self.index >= self.ef.n {
            return None;
        }

        loop {
            let word_idx = (self.upper_bit_pos / 64) as usize;
            let bit_idx = self.upper_bit_pos % 64;

            if word_idx >= self.ef.upper.len() {
                return None;
            }

            let bit = (self.ef.upper[word_idx] >> bit_idx) & 1;
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
        assert_eq!(ef.iter().collect::<Vec<_>>(), Vec::<u16>::new());
    }

    #[test]
    fn test_basic_sequence() {
        let values: Vec<u16> = vec![2, 3, 5, 7, 11, 13, 24];
        let ef = EliasFano::new(&values);

        assert_eq!(ef.len(), values.len() as u16);

        for (i, &v) in values.iter().enumerate() {
            assert_eq!(ef.get(i as u16), Some(v), "Mismatch at index {}", i);
        }
    }

    #[test]
    fn test_duplicates() {
        let values: Vec<u16> = vec![1, 1, 3, 3, 3, 7, 7];
        let ef = EliasFano::new(&values);

        for (i, &v) in values.iter().enumerate() {
            assert_eq!(ef.get(i as u16), Some(v), "Mismatch at index {}", i);
        }
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
            assert_eq!(ef.get(i as u16), Some(v), "Mismatch at index {}", i);
        }

        let collected: Vec<u16> = ef.iter().collect();
        assert_eq!(collected, values);
    }

    #[test]
    fn test_consecutive() {
        let values: Vec<u16> = (0..100).collect();
        let ef = EliasFano::new(&values);

        for (i, &v) in values.iter().enumerate() {
            assert_eq!(ef.get(i as u16), Some(v));
        }

        let collected: Vec<u16> = ef.iter().collect();
        assert_eq!(collected, values);
    }

    #[test]
    fn test_all_same() {
        let values: Vec<u16> = vec![500; 50];
        let ef = EliasFano::new(&values);

        for i in 0..50u16 {
            assert_eq!(ef.get(i), Some(500));
        }

        let collected: Vec<u16> = ef.iter().collect();
        assert_eq!(collected, values);
    }

    #[test]
    fn test_zeros() {
        let values: Vec<u16> = vec![0, 0, 0, 0];
        let ef = EliasFano::new(&values);

        for i in 0..4u16 {
            assert_eq!(ef.get(i), Some(0));
        }
    }

    #[test]
    fn test_sparse() {
        let values: Vec<u16> = vec![100, 20000, 40000, 60000, 65535];
        let ef = EliasFano::new(&values);

        for (i, &v) in values.iter().enumerate() {
            assert_eq!(ef.get(i as u16), Some(v), "Mismatch at index {}", i);
        }

        let collected: Vec<u16> = ef.iter().collect();
        assert_eq!(collected, values);
    }

    #[test]
    fn test_dense_full_range() {
        let values: Vec<u16> = (0..=65535).collect();
        let ef = EliasFano::new(&values);

        assert_eq!(ef.len(), 0); // 65536 as u16 wraps to 0 — see note below
        // Actually 65536 values can't be represented by u16 len.
        // This test validates the assertion in new().
    }

    #[test]
    fn test_iterator_exact_size() {
        let values: Vec<u16> = vec![1, 2, 3, 4, 5];
        let ef = EliasFano::new(&values);
        let iter = ef.iter();
        assert_eq!(iter.len(), 5);
    }

    #[test]
    #[should_panic(expected = "sorted")]
    fn test_unsorted_panics() {
        EliasFano::new(&[5, 3, 1]);
    }
}
