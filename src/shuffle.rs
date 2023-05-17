use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use rand::rngs::StdRng;
use rand::{seq::SliceRandom, Rng, SeedableRng};

#[derive(PartialEq, Eq, Hash)]
pub struct ShuffleIndex {
    iteration_nr: u32,
    nr_bits: u32,
    has_seed: bool,
}

impl ShuffleIndex {
    pub fn new(iteration_nr: u32, nr_bits: u32, has_seed: bool) -> Self {
        Self {
            iteration_nr,
            nr_bits,
            has_seed,
        }
    }
}

/// Shuffle is a permutation of the bit index of a key.
#[derive(Debug)]
pub struct Shuffle {
    iteration_nr: u32,
    nr_bits: u32,
    has_seed: bool,
    seed: u64,
    // the value is the index of the shuffled bit
    orig_to_shuffled_map: Vec<u32>,
    // the value is the index of the original bit
    shuffled_to_orig_map: Vec<u32>,
}

// Both ShuffledKey and CATCHE hold a reference to Shuffle.
pub type SharedShuffle = Rc<Shuffle>;

thread_local! {
    static CACHE: RefCell<HashMap<ShuffleIndex, SharedShuffle>> = RefCell::new(HashMap::new());

}

impl Shuffle {
    pub fn new_random_shuffle(
        iteration_nr: u32,
        nr_bits: u32,
        assign_seed: bool,
        cache: bool,
    ) -> SharedShuffle {
        assert!(iteration_nr > 0);
        let index = ShuffleIndex::new(iteration_nr, nr_bits, assign_seed);
        let shuffle = if cache {
            CACHE.with(|c| {
                if let Some(shuffle) = c.borrow().get(&index) {
                    return Rc::clone(shuffle);
                }

                let shuffle = Rc::new(Shuffle::new(iteration_nr, nr_bits, assign_seed));
                c.borrow_mut().insert(index, Rc::clone(&shuffle));
                shuffle
            })
        } else {
            Rc::new(Shuffle::new(iteration_nr, nr_bits, assign_seed))
        };

        shuffle
    }

    pub fn new_shuffle_from_seed(
        iteration_nr: u32,
        nr_bits: u32,
        seed: u64,
        cache: bool,
    ) -> SharedShuffle {
        assert!(iteration_nr > 0);
        let index = ShuffleIndex::new(iteration_nr, nr_bits, true);
        let shuffle = if cache {
            CACHE.with(|c| {
                if let Some(shuffle) = c.borrow().get(&index) {
                    return Rc::clone(shuffle);
                }

                let shuffle = Rc::new(Shuffle::from_seed(iteration_nr, nr_bits, seed));
                c.borrow_mut().insert(index, Rc::clone(&shuffle));
                shuffle
            })
        } else {
            Rc::new(Shuffle::from_seed(iteration_nr, nr_bits, seed))
        };

        shuffle
    }

    fn new(iteration_nr: u32, nr_bits: u32, assign_seed: bool) -> Self {
        let mut shuffle = Self {
            iteration_nr,
            nr_bits,
            has_seed: false,
            seed: 0,
            orig_to_shuffled_map: vec![0; nr_bits as usize],
            shuffled_to_orig_map: Vec::with_capacity(nr_bits as usize),
        };
        shuffle.initialize(assign_seed);
        shuffle
    }

    fn from_seed(iteration_nr: u32, nr_bits: u32, seed: u64) -> Self {
        let mut shuffle = Self {
            iteration_nr,
            nr_bits,
            has_seed: true,
            seed,
            orig_to_shuffled_map: vec![0; nr_bits as usize],
            shuffled_to_orig_map: Vec::with_capacity(nr_bits as usize),
        };
        shuffle.initialize(false);
        shuffle
    }

    fn initialize(&mut self, assign_seed: bool) {
        for bit_nr in 0..self.nr_bits {
            self.shuffled_to_orig_map.push(bit_nr);
        }

        if self.iteration_nr != 1 {
            if assign_seed {
                assert!(!self.has_seed);
                self.has_seed = true;
                self.seed = rand::thread_rng().gen();
            }
            if self.has_seed {
                let mut rng = StdRng::seed_from_u64(self.seed);
                self.shuffled_to_orig_map.shuffle(&mut rng);
            } else {
                // lazily-initialized thread local RNG, avoids the cost of constructing a new one
                self.shuffled_to_orig_map.shuffle(&mut rand::thread_rng());
            }
        }
        // Compute the reverse mapping of original key bits to shuffled key bits.
        for (shuffled_bit_nr, &orig_bit_nr) in self.shuffled_to_orig_map.iter().enumerate() {
            self.orig_to_shuffled_map[orig_bit_nr as usize] = shuffled_bit_nr as u32;
        }
    }

    pub fn get_seed(&self) -> u64 {
        self.seed
    }

    pub fn get_nr_bits(&self) -> u32 {
        self.nr_bits
    }

    pub fn orig_to_shuffle(&self, orig_bit_nr: u32) -> u32 {
        self.orig_to_shuffled_map[orig_bit_nr as usize]
    }

    pub fn shuffle_to_orig(&self, shuffle_bit_nr: u32) -> u32 {
        self.shuffled_to_orig_map[shuffle_bit_nr as usize]
    }
}

#[cfg(test)]
mod tests {
    use crate::shuffle::{self, CACHE};

    use super::Shuffle;

    #[test]
    fn test_random_shuffle() {
        // no shuffle at iteration 1
        let shuffle = Shuffle::new_random_shuffle(1, 10, true, false);
        assert_eq!(shuffle.orig_to_shuffled_map, shuffle.shuffled_to_orig_map);
        assert_eq!(0, shuffle.get_seed());
        // shuffle at iteration 2
        let shuffle = Shuffle::new_random_shuffle(2, 10, true, false);
        assert_ne!(shuffle.orig_to_shuffled_map, shuffle.shuffled_to_orig_map);
        let shuffled_bit_nr = 5;
        let ori_bit_nr = shuffle.shuffle_to_orig(shuffled_bit_nr);
        assert_eq!(shuffled_bit_nr, shuffle.orig_to_shuffle(ori_bit_nr));
    }

    #[test]
    fn test_random_shuffle_from_seed() {
        const SEED: u64 = 123456789;
        let shuffle = Shuffle::new_shuffle_from_seed(2, 10, SEED, false);
        assert_eq!(SEED, shuffle.get_seed());
        assert_ne!(shuffle.orig_to_shuffled_map, shuffle.shuffled_to_orig_map);
        let shuffled_bit_nr = 5;
        let ori_bit_nr = shuffle.shuffle_to_orig(shuffled_bit_nr);
        assert_eq!(shuffled_bit_nr, shuffle.orig_to_shuffle(ori_bit_nr));
    }

    #[test]
    fn test_shuffle_cache() {
        const SEED: u64 = 123456789;
        const NUM_BITS: u32 = 4;
        let max_nr = 2u32.pow(4);
        // fill the cache
        for i in 1..=max_nr {
            let _ = Shuffle::new_shuffle_from_seed(i, NUM_BITS, SEED, true);
        }
        CACHE.with(|c| {
            assert_eq!(max_nr as usize, c.borrow().len());
        });
    }
}
