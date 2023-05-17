//! This is the top module to implement cascade protocol to reconcile two keys.
//!

use std::rc::Rc;

use crate::{
    algorithm::OriginalAlgorithm, iteration::Iteration, key::Key, shuffled_key::SharedKey,
};

pub struct Reconciliation {
    iterations: Vec<Iteration<OriginalAlgorithm>>,
}

impl Reconciliation {
    pub fn new(num_iterations: u32, correct_key: Rc<Key>, noise_key: SharedKey) -> Self {
        let mut iterations = Vec::with_capacity(num_iterations as usize);

        for iteration_nr in 0..num_iterations {
            let iteration = Iteration::new(
                iteration_nr + 1,
                correct_key.clone(),
                noise_key.clone(),
                OriginalAlgorithm::default(),
            );
            iterations.push(iteration);
        }

        Self { iterations }
    }

    pub fn start_iterations(&self) {
        for iter_nr in 0..self.iterations.len() {
            let iteration = &self.iterations[iter_nr];
            println!(
                "--------- ITERATION {} ---------",
                iteration.get_iteration_nr()
            );
            iteration.schedule_top_block_ask_correct_parity_task();
            let corrected_orig_bits_nr = iteration.schedule_top_block_correct_task();

            self.cascade(iteration.get_iteration_nr(), corrected_orig_bits_nr);
        }
    }

    pub fn cascade(&self, trigger_iteration_nr: u32, corrected_orig_bits_nr: Vec<u32>) {
        // cascade to other iterations
        let cascade_iterations = self.iterations.iter().filter(|cascade_iteration| {
            cascade_iteration.get_iteration_nr() < trigger_iteration_nr
            // && cascade_iteration.is_started()
        });
        let other_iterations = cascade_iterations
            .clone()
            .map(|it| it.get_iteration_nr().to_string())
            .reduce(|a, b| format!("{}, {},", a, b));
        match other_iterations {
            Some(other_iterations) => {
                println!("cascade to Iteration {}", other_iterations,);
            }
            None => {
                println!("no other iterations to cascade");
            }
        }

        for orig_bit_nr in corrected_orig_bits_nr {
            cascade_iterations.clone().for_each(|cascade_iteration| {
                println!(
                    "cascade to Iteration {}, orig bit nr: {}",
                    cascade_iteration.get_iteration_nr(),
                    orig_bit_nr
                );
                for top_block in cascade_iteration.get_top_blocks() {
                    let bit_nr = cascade_iteration
                        .get_shuffled_key()
                        .orig_to_shuffle_bit_nr(orig_bit_nr);

                    if top_block.contains_bit(bit_nr) {
                        println!(
                            "cascade to Iteration {}, trigger Iteration {},
                                        block: {},
                                        flip parity downstream, shuffle bit nr: {}",
                            cascade_iteration.get_iteration_nr(),
                            trigger_iteration_nr,
                            top_block,
                            bit_nr
                        );
                        // to reduce re-computation
                        cascade_iteration.flip_parity_downstream(top_block, bit_nr);
                        // rely on parity flip is correctly done
                        let more_bit_nrs = cascade_iteration.schedule_top_block_correct_task();
                        self.cascade(cascade_iteration.get_iteration_nr(), more_bit_nrs);
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        algorithm::OriginalAlgorithm,
        block::{Block, BlockType},
        key::Key,
        reconciliation::Reconciliation,
        shuffle::Shuffle,
        shuffled_key::{SharedKey, ShuffledKey},
    };

    use super::Iteration;
    fn create_test_shuffled_key(key_str: &str) -> (Rc<Key>, SharedKey) {
        const SEED: u64 = 0x1234567890ABCDEF;
        // correct key
        let correct_key = Key::from(key_str);
        // noise key from file
        let mut noise_key = correct_key.clone();
        noise_key.set_estimated_ber(0.1); // 10% BER, about 3 errors
        noise_key.apply_noise();

        assert_ne!(correct_key.to_string(), noise_key.to_string());
        (Rc::new(correct_key), Rc::new(RefCell::new(noise_key)))
    }

    fn print_keys(correct_key: &Rc<Key>, noise_key: &SharedKey) {
        println!("correct key: {}", correct_key.to_string());
        println!("noise key:   {}", noise_key.borrow().to_string());
    }

    #[test]
    fn test_reconciliation() {
        const NUM_ITERATIONS: u32 = 4;
        const KEY_STR: &str = "10010001100100011001000110010001";
        assert_eq!(KEY_STR.len(), 32);

        let (correct_key, noise_key) = create_test_shuffled_key(KEY_STR);
        let reconciliation =
            Reconciliation::new(NUM_ITERATIONS, correct_key.clone(), noise_key.clone());
        print_keys(&correct_key, &noise_key);
        reconciliation.start_iterations();
        print_keys(&correct_key, &noise_key);

        println!(
            "bit differences: {}",
            correct_key.nr_bits_different(&*noise_key.borrow())
        );
        assert_eq!(correct_key.to_string(), noise_key.borrow().to_string());
    }

    #[test]
    fn test_reconciliation_large() {
        const NUM_ITERATIONS: u32 = 9;
        let key_str =
            "100100011001000110010100011001000101000110010001010001100100011100010001".repeat(200);
        assert_eq!(key_str.len(), 14400);

        let (correct_key, noise_key) = create_test_shuffled_key(&key_str);

        let initial_bit_err = correct_key.nr_bits_different(&*noise_key.borrow());
        let reconciliation =
            Reconciliation::new(NUM_ITERATIONS, correct_key.clone(), noise_key.clone());
        // print_keys(&correct_key, &noise_key);
        reconciliation.start_iterations();
        // print_keys(&correct_key, &noise_key);

        let final_bit_err = correct_key.nr_bits_different(&*noise_key.borrow());

        println!(
            "bit differences: initial: {}, final: {}",
            initial_bit_err, final_bit_err
        );
        assert_eq!(correct_key.to_string(), noise_key.borrow().to_string());
    }
}
