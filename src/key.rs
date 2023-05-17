use crate::random::{random_bit_nr, random_uint32};
use std::{collections::HashSet, fmt};

/// Calculate the parity of a sigle word.
///
/// This only need 8 XOR and 8 lookups operations.
///
/// a naive approach of counting 1s in the word would require iterating
/// through all 64 bits, checking if each bit is set to 1,
/// and incrementing a counter if it is.
/// This would involve more operations and conditional statements,
/// making it less efficient than the precomputed table approach.
///
///
fn word_parity(word: u64) -> u8 {
    // precomputed parity values for all possible 256 byte values (from 0 to 255).
    static BYTE_PARITY: [u8; 256] = [
        0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0,
        0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1,
        0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 1, 0,
        0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0,
        0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0,
        0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1,
        0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0,
        0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1,
        0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0,
    ];

    let mut parity = 0;
    parity ^= BYTE_PARITY[(word & 0xff) as usize];
    parity ^= BYTE_PARITY[((word >> 8) & 0xff) as usize];
    parity ^= BYTE_PARITY[((word >> 16) & 0xff) as usize];
    parity ^= BYTE_PARITY[((word >> 24) & 0xff) as usize];
    parity ^= BYTE_PARITY[((word >> 32) & 0xff) as usize];
    parity ^= BYTE_PARITY[((word >> 40) & 0xff) as usize];
    parity ^= BYTE_PARITY[((word >> 48) & 0xff) as usize];
    parity ^= BYTE_PARITY[((word >> 56) & 0xff) as usize];
    parity
}

/// Key is per thread data structure
/// Key bits are stored into 64-bit int "words" as follows:
///
/// +----+----+----+-//-+----+----+----+   +----+----+----+-//-+----+----+----+   +---...
/// | 63 | 62 | 61 |    |  2 |  1 |  0 |   | 127| 126| 125|    | 66 | 65 | 64 |   |
/// +----+----+----+-//-+----+----+----+   +----+----+----+-//-+----+----+----+   +---...
///   63   62   61         2    1    0       63   62   61         2    1    0       
///   MSB                          LSB       MSB                          LSB
///                Word 0                                 Word 1
///
/// Note: we use the term "word" instead of te more natural term "block" to avoid confusion with
///       cascade blocks.
#[derive(Clone, Debug)]
pub struct Key {
    nr_bits: u32,
    nr_words: u32,
    words: Vec<u64>,
    estimated_ber: f32,
}

impl From<&str> for Key {
    fn from(value: &str) -> Self {
        assert!(!value.chars().any(|c| c != '0' && c != '1'));
        let nr_bits_param = u32::try_from(value.len()).expect("should be able to convert to u32");
        let nr_words = (nr_bits_param - 1) / 64 + 1;
        let mut words = Vec::with_capacity(nr_words as usize);
        for word in value.as_bytes().chunks(64) {
            let mut word_value = 0;
            for (i, &byte) in word.iter().enumerate() {
                word_value |= u64::from(byte - b'0') << i;
            }
            words.push(word_value);
        }
        words[nr_words as usize - 1] &= Self::end_word_mask(nr_bits_param - 1);

        Key {
            nr_bits: nr_bits_param,
            nr_words,
            words,
            estimated_ber: Self::ESTIMATED_QBER,
        }
    }
}

impl Key {
    const ESTIMATED_QBER: f32 = 0.02; // TODO: read from config file

    pub fn set_estimated_ber(&mut self, estimated_ber: f32) {
        self.estimated_ber = estimated_ber;
    }
    pub fn get_estimated_ber(&self) -> f32 {
        self.estimated_ber
    }
    /// Apply bit errors to the key, for prototype purposes.
    pub fn apply_noise(&mut self) {
        let nr_bit_errors = (self.estimated_ber * self.nr_bits as f32).round() as u32;
        let mut error_bits = HashSet::new();
        println!("nr_bit_errors: {}", nr_bit_errors);
        for d in (self.nr_bits - nr_bit_errors)..self.nr_bits {
            let t = random_bit_nr(0, d);
            if !error_bits.contains(&t) {
                error_bits.insert(t);
            } else {
                error_bits.insert(d);
            }
        }

        for bit in error_bits {
            self.flip_bit(bit);
        }
    }
    // Get work mask for the start word
    //
    // # Example
    //
    //   start_bit_nr(e.g., 4)  -->| |<-- unused bits -->|
    // +----+----+----+-//-+----+----+----+----+----+----+   +---...
    // | 1  | 1  | 1  |    |  1 |  1 |  0 |  0 |  0 |  0 |   |
    // +----+----+----+-//-+----+----+----+----+----+----+   +---...
    //   63   62   61                   3    2    1    0
    // |<------------------ start word ----------------->|
    //
    fn start_word_mask(start_bit_nr: u32) -> u64 {
        let nr_unused_bits = start_bit_nr % 64;
        0xffffffffffffffffu64 << nr_unused_bits
    }

    // Get word mask for the end word
    //
    // # Example
    //
    //          |<-- unused bits -->|  |<- (end_bit_nr + 1) % 64 (e.g., 59)
    // ...---+  +----+----+----+----+----+----+-//-+----+----+----+
    //       |  | 0  | 0  | 0  |  0 |  1 |  1 |    |  1 |  1 |  1 |
    // ...---+  +----+----+----+----+----+----+-//-+----+----+----+
    //            63   62   61                   3    2    1    0
    //          |<------------------ end word ----------------->|
    //
    fn end_word_mask(end_bit_nr: u32) -> u64 {
        let nr_unused_bits = 64 - ((end_bit_nr + 1) % 64);
        let mut mask = 0xffffffffffffffffu64;
        if nr_unused_bits != 64 {
            mask >>= nr_unused_bits;
        }
        mask
    }

    pub fn compute_range_parity(&self, start_bit_nr: u32, end_bit_nr: u32) -> u8 {
        assert!(start_bit_nr < self.nr_bits);
        assert!(end_bit_nr < self.nr_bits);

        let start_word_nr = start_bit_nr / 64;
        let end_word_nr = end_bit_nr / 64;

        let mut xor_words = 0u64;
        for word_nr in start_word_nr..=end_word_nr {
            xor_words ^= self.words[word_nr as usize];
        }
        // Undo bits that we did not want to include in first word.
        let unwanted_mask = !Self::start_word_mask(start_bit_nr);
        let unwanted_bits = self.words[start_word_nr as usize] & unwanted_mask;
        xor_words ^= unwanted_bits;

        // Undo bits that we did not want to include in first word.
        let unwanted_mask = !Self::end_word_mask(end_bit_nr);
        let unwanted_bits = self.words[end_word_nr as usize] & unwanted_mask;
        xor_words ^= unwanted_bits;

        word_parity(xor_words)
    }

    pub(crate) fn get_nr_bits(&self) -> u32 {
        self.nr_bits
    }

    pub fn nr_bits_different(&self, other_key: &Key) -> u32 {
        assert!(self.nr_bits == other_key.nr_bits);

        let mut difference = 0;
        for word_nr in 0..self.nr_words {
            let word_nr = word_nr as usize;
            let xor_word = self.words[word_nr] ^ other_key.words[word_nr];
            difference += xor_word.count_ones() as u32;
        }

        difference
    }

    pub(crate) fn get_bit(&self, bit_nr: u32) -> u8 {
        assert!(bit_nr < self.nr_bits);
        let word_nr = (bit_nr / 64) as usize;
        let bit_nr_in_word = bit_nr % 64;
        let mask = 1u64 << bit_nr_in_word;

        (self.words[word_nr] & mask != 0) as u8
    }

    pub(crate) fn set_bit(&mut self, bit_nr: u32, value: u8) {
        assert!(bit_nr < self.nr_bits);
        let word_nr = (bit_nr / 64) as usize;
        let bit_nr_in_word = bit_nr % 64;
        let mask = 1u64 << bit_nr_in_word;

        match value {
            0 => self.words[word_nr] &= !mask,
            1 => self.words[word_nr] |= mask,
            _ => panic!("Invalid value for setting a bit"),
        }
    }

    pub(crate) fn flip_bit(&mut self, bit_nr: u32) {
        assert!(bit_nr < self.nr_bits);
        let word_nr = (bit_nr / 64) as usize;
        let bit_nr_in_word = bit_nr % 64;
        let mask = 1u64 << bit_nr_in_word;
        self.words[word_nr] ^= mask;
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        // Print key bits in natural order from LSB to MSB
        for bit_nr in 0..self.nr_bits {
            // '0' ascii code is 48
            let zero = '0' as u8;
            s.push(char::from(zero + self.get_bit(bit_nr)));
        }
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        key::Key,
        random::{self, set_random_uint32_seed},
    };

    #[test]
    fn test_str_to_key() {
        let key = Key::from("1011000010101111010010001001000011001100110001011010100001010111");
        assert_eq!(
            "1011000010101111010010001001000011001100110001011010100001010111",
            key.to_string()
        );

        let mut noise_key = key.clone();
        noise_key.set_estimated_ber(0.1);
        noise_key.apply_noise();
        println!("key:       {:>12}", key.to_string());
        println!("noise_key: {:>12}", noise_key.to_string());
        assert_ne!(key.to_string(), noise_key.to_string());
    }

    #[test]
    fn test_compute_parity() {
        let key = Key::from("1011000010101111010010001001000011001100110001011010100001010111");
        assert_eq!(
            "1011000010101111010010001001000011001100110001011010100001010111",
            key.to_string()
        );
        assert_eq!(1, key.compute_range_parity(0, 63));
        assert_eq!(0, key.compute_range_parity(0, 62));
        assert_eq!(0, key.compute_range_parity(1, 63));
        assert_eq!(1, key.compute_range_parity(1, 62));
        assert_eq!(1, key.compute_range_parity(0, 0));
        assert_eq!(1, key.compute_range_parity(63, 63));
    }

    #[test]
    fn test_key_clone() {
        set_random_uint32_seed(1111);

        let mut key = Key::from("1011000010101111010010001001000011001100110001011010100001010111");
        assert_eq!(
            "1011000010101111010010001001000011001100110001011010100001010111",
            key.to_string()
        );
        let mut key_clone = key.clone();
        assert_eq!(
            "1011000010101111010010001001000011001100110001011010100001010111",
            key_clone.to_string()
        );
        // Make sure that changing a bit in the original key does not affect the copied key,
        // and vice versa.
        key.set_bit(60, 1);
        key_clone.set_bit(61, 0);
        assert_eq!(
            "1011000010101111010010001001000011001100110001011010100001011111",
            key.to_string()
        );
        assert_eq!(
            "1011000010101111010010001001000011001100110001011010100001010011",
            key_clone.to_string()
        );
    }
}
