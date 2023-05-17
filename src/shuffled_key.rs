use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use crate::{key::Key, shuffle::SharedShuffle};

pub type SharedKey = Rc<RefCell<Key>>;
/// ShuffledKey is a key with a shuffle applied to it.
/// ShuffledKey clones shares the same shuffle and key.
/// Not thread safe.

#[derive(Clone, Debug)]
pub struct ShuffledKey {
    pub correct_key: Rc<Key>, //TODO: test only, remove this
    key: SharedKey,
    shuffle: SharedShuffle,
}

impl ShuffledKey {
    pub fn new(correct_key: Rc<Key>, noise_key: SharedKey, shuffle: SharedShuffle) -> Self {
        Self {
            correct_key,
            key: noise_key,
            shuffle,
        }
    }
    pub fn get_estimated_ber(&self) -> f32 {
        self.key.borrow().get_estimated_ber()
    }
    pub fn get_shuffle(&self) -> SharedShuffle {
        Rc::clone(&self.shuffle)
    }

    pub fn get_nr_bits(&self) -> u32 {
        self.key.borrow().get_nr_bits()
    }

    /// get bit in the original key
    pub fn get_bit(&self, bit_nr: u32) -> u8 {
        let orig_bit_nr = self.shuffle.shuffle_to_orig(bit_nr);
        self.key.borrow().get_bit(orig_bit_nr)
    }

    pub fn shuffle_to_orig_bit_nr(&self, shuffle_bit_nr: u32) -> u32 {
        self.shuffle.shuffle_to_orig(shuffle_bit_nr)
    }
    pub fn orig_to_shuffle_bit_nr(&self, orig_bit_nr: u32) -> u32 {
        self.shuffle.orig_to_shuffle(orig_bit_nr)
    }

    /// set bit in the original key
    pub fn set_bit(&self, bit_nr: u32, value: u8) {
        let orig_bit_nr = self.shuffle.shuffle_to_orig(bit_nr);
        self.key.borrow_mut().set_bit(orig_bit_nr, value);
    }

    /// flip bit in the original key
    pub fn flip_bit(&self, bit_nr: u32) {
        let orig_bit_nr = self.shuffle.shuffle_to_orig(bit_nr);
        self.key.borrow_mut().flip_bit(orig_bit_nr);
    }

    pub fn compute_range_parity(&self, start_bit_nr: u32, end_bit_nr: u32) -> u8 {
        let mut parity = 0;
        // have to get the index of the bit in the original key first
        // so we can not use original key's compute_range_parity method
        for bit_nr in start_bit_nr..=end_bit_nr {
            let orig_bit_nr = self.shuffle.shuffle_to_orig(bit_nr);
            if self.key.borrow().get_bit(orig_bit_nr) == 1 {
                parity = 1 - parity;
            }
        }
        parity
    }

    // TODO: this is for testing only,
    pub(crate) fn ask_correct_range_parity(&self, start_bit_nr: u32, end_bit_nr: u32) -> u8 {
        let mut parity = 0;
        // have to get the index of the bit in the original key first
        // so we can not use original key's compute_range_parity method
        for bit_nr in start_bit_nr..=end_bit_nr {
            let orig_bit_nr = self.shuffle.shuffle_to_orig(bit_nr);
            if self.correct_key.get_bit(orig_bit_nr) == 1 {
                parity = 1 - parity;
            }
        }
        parity
    }
    //TODO: testing only
    pub(crate) fn get_noise_key(&self) -> Ref<'_, Key> {
        self.key.borrow()
    }
    pub(crate) fn get_correct_key(&self) -> &Rc<Key> {
        &self.correct_key
    }
}

impl std::fmt::Display for ShuffledKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        // Print key bits in natural order from LSB to MSB
        for bit_nr in 0..self.get_nr_bits() {
            // '0' ascii code is 48
            let zero = '0' as u8;
            s.push(char::from(zero + self.get_bit(bit_nr)));
        }
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Ref, RefCell},
        rc::Rc,
    };

    use crate::{key::Key, random, shuffle::Shuffle, shuffled_key::ShuffledKey};
    #[test]
    fn test_compute_parity() {
        const SEED: u64 = 12345678;
        const ORIGINAL_KEY: &str =
            "1011000010101111010010001001000011001100110001011010100001010111";
        const KEY_SIZE: u32 = ORIGINAL_KEY.len() as u32;

        // random key
        random::set_random_uint32_seed(SEED as u32);
        let correct_key = Key::from(ORIGINAL_KEY);
        let key: Rc<RefCell<_>> = Rc::new(RefCell::new(correct_key.clone()));

        // random shuffle
        let shuffle = Shuffle::new_shuffle_from_seed(2, KEY_SIZE, SEED, true);
        let shuffled_key =
            ShuffledKey::new(Rc::new(correct_key), Rc::clone(&key), Rc::clone(&shuffle));

        let ori_parity: u8 = key.borrow().compute_range_parity(0, KEY_SIZE - 1);
        let shuffled_parity = shuffled_key.compute_range_parity(0, KEY_SIZE - 1);
        assert_eq!(ori_parity, shuffled_parity);

        const BIT_NR: u32 = 2;
        shuffled_key.flip_bit(BIT_NR);
        let ori_parity = key.borrow().compute_range_parity(0, KEY_SIZE - 1);
        let shuffled_parity = shuffled_key.compute_range_parity(0, KEY_SIZE - 1);
        assert_eq!(ori_parity, shuffled_parity);
    }
}
