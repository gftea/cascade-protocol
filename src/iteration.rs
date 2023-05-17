use std::rc::Rc;

use crate::{
    algorithm::{Algorithm, OriginalAlgorithm},
    block::{Block, BlockRef, BlockType, SubBlockType},
    key::Key,
    shuffle::{self, SharedShuffle, Shuffle},
    shuffled_key::{SharedKey, ShuffledKey},
};

pub struct Iteration<T: Algorithm> {
    iteration_nr: u32,
    top_blocks: Vec<Rc<Block>>,
    nr_key_bits: u32,
    algo: T,
    shuffled_key: ShuffledKey,
}

impl<T: Algorithm> Iteration<T> {
    pub fn new(iteration_nr: u32, correct_key: Rc<Key>, noise_key: SharedKey, algo: T) -> Self {
        // create shuffled key for this iteration
        // for testing purposes, we use a fixed seed
        const SEED: u64 = 0x1234567890ABCDEF;

        let shuffle = Shuffle::new_shuffle_from_seed(
            iteration_nr,
            noise_key.borrow().get_nr_bits(),
            SEED,
            true,
        );
        let shuffled_key = ShuffledKey::new(correct_key, noise_key, shuffle);

        let estimated_ber = shuffled_key.get_estimated_ber();
        let nr_key_bits = shuffled_key.get_nr_bits();
        // create top blocks for this iteration
        let block_size = <T as Algorithm>::block_size(iteration_nr, estimated_ber, nr_key_bits);
        println!("block size: {}", block_size);
        let mut start_bit_nr = 0;
        let mut top_blocks = Vec::new();

        while start_bit_nr < nr_key_bits {
            let end_bit_nr = std::cmp::min(start_bit_nr + block_size, nr_key_bits) - 1;
            let block = Block::new(
                BlockType::TopLevel,
                start_bit_nr,
                end_bit_nr,
                shuffled_key.clone(),
            );
            top_blocks.push(block);
            start_bit_nr += block_size;
        }

        Self {
            iteration_nr,
            top_blocks,
            nr_key_bits,
            algo,
            shuffled_key,
        }
    }

    pub fn schedule_top_block_ask_correct_parity_task(&self) {
        println!(
            "Iteration: {}, schedule top block ask correct parity task",
            self.get_iteration_nr()
        );
        for block in self.top_blocks.iter() {
            // spawn async tasks for concurrent asking
            block.ask_correct_parity();
        }
    }

    pub fn schedule_top_block_correct_task(&self) -> Vec<u32> {
        println!(
            "Iteration: {}, schedule top block correct task",
            self.get_iteration_nr()
        );
        let mut corrected_bits = Vec::new();
        for block in self.top_blocks.iter() {
            if block.get_error_parity() {
                println!(
                    "schedule correct block: {:?}, range: {}..{}",
                    block.get_block_type(),
                    block.get_start_bit_nr(),
                    block.get_end_bit_nr()
                );
                let orig_bit_nr = self.try_correct_block(block);
                println!("corrected bit: {}", orig_bit_nr);

                corrected_bits.push(orig_bit_nr);
            }
        }
        corrected_bits
    }

    // start with top block
    pub fn try_correct_block(&self, block: &BlockRef) -> u32 {
        let mut current_block = block.clone();

        while current_block.get_nr_bits() > 1 {
            let left_sub_block = current_block.create_sub_block(SubBlockType::Left);
            let right_sub_block = current_block.create_sub_block(SubBlockType::Right);
            left_sub_block.ask_correct_parity();
            right_sub_block.try_to_infer_correct_parity();

            let error_parity = left_sub_block.get_error_parity();

            // if odd number of errors, recurse on left sub block
            if error_parity {
                current_block = left_sub_block;
            }
            // if even number of errors, we can infer right sub block
            // and recurse on it
            else {
                right_sub_block.get_error_parity();
                current_block = right_sub_block
            }
        }
        // correct the bit
        let shuffle_bit_nr = current_block.get_start_bit_nr();
        current_block.correct_bit(shuffle_bit_nr);

        self.flip_parity_upstream(&current_block);
        return self.shuffled_key.shuffle_to_orig_bit_nr(shuffle_bit_nr);
    }

    pub fn get_iteration_nr(&self) -> u32 {
        self.iteration_nr
    }

    pub fn get_top_blocks(&self) -> &Vec<Rc<Block>> {
        &self.top_blocks
    }

    pub fn is_started(&self) -> bool {
        self.top_blocks
            .iter()
            .all(|block| block.get_correct_parity().is_some())
    }

    pub fn flip_parity_upstream(&self, leaf_block: &BlockRef) {
        // current block
        leaf_block.flip_current_parity();

        // traverse up to top block
        let mut parent_block = leaf_block.get_parent_block();
        while let Some(block) = parent_block {
            block.flip_current_parity();
            parent_block = block.get_parent_block();
        }
    }

    /// Start with the top block that contains this bit
    pub fn flip_parity_downstream(&self, top_block: &BlockRef, bit_nr: u32) {
        println!("flip parity downstream, bit nr: {}", bit_nr);
        // current top block
        top_block.flip_current_parity();
        // traverse down all blocks containing this bit
        // because it is binary tree, either left or right sub block will contain this bit
        let mut block = top_block.clone();
        while block.has_sub_blocks() {
            // start with left
            // note that we always create both left and right sub blocks, so
            // if any of them is None, we can break
            let left_block = block.get_left_sub_block().unwrap();
            if left_block.contains_bit(bit_nr) {
                left_block.flip_current_parity();
                block = left_block;
                continue;
            }
            // then right
            let right_block = block.get_right_sub_block().unwrap();
            assert!(right_block.contains_bit(bit_nr));
            right_block.flip_current_parity();
            block = right_block;
        }
    }

    pub fn get_shuffled_key(&self) -> &ShuffledKey {
        &self.shuffled_key
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        algorithm::OriginalAlgorithm,
        block::{Block, BlockType},
        key::Key,
        shuffle::Shuffle,
        shuffled_key::{SharedKey, ShuffledKey},
    };

    use super::Iteration;

    fn create_test_shuffled_key() -> (Rc<Key>, SharedKey) {
        const SEED: u64 = 0x1234567890ABCDEF;
        const KEY_STR: &str = "10010001100100011001000110010001";
        assert_eq!(KEY_STR.len(), 32);
        // correct key
        let correct_key = Key::from(KEY_STR);
        // noise key from file
        let mut noise_key = correct_key.clone();
        noise_key.set_estimated_ber(0.1); // 10% BER, about 3 errors
        noise_key.apply_noise();

        assert_ne!(correct_key.to_string(), noise_key.to_string());
        (Rc::new(correct_key), Rc::new(RefCell::new(noise_key)))
    }

    fn print_keys(shuffled_key: &ShuffledKey) {
        println!(
            "correct key: {}",
            shuffled_key.get_correct_key().to_string()
        );
        println!("noise key:   {}", shuffled_key.get_noise_key().to_string());
    }

    #[test]
    fn test_correct_block() {
        const ITERATION_NR: u32 = 2;
        let (correct_key, noise_key) = create_test_shuffled_key();
        let iteration = Iteration::new(
            ITERATION_NR,
            correct_key,
            noise_key,
            OriginalAlgorithm::default(),
        );
        print_keys(&iteration.get_shuffled_key());

        println!("top blocks count: {}", iteration.get_top_blocks().len());

        iteration.schedule_top_block_ask_correct_parity_task();
        iteration.schedule_top_block_correct_task();
        // should correct one bit
        print_keys(&iteration.get_shuffled_key());
    }
}
