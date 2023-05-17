use std::{cell::RefCell, rc::Rc};

use prototype::{
    algorithm::OriginalAlgorithm,
    block::{Block, BlockType},
    iteration::Iteration,
    key::Key,
    reconciliation::Reconciliation,
    shuffle::Shuffle,
    shuffled_key::{SharedKey, ShuffledKey},
};

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
    assert_eq!(final_bit_err, 0);
}

fn main() {
    test_reconciliation_large();
}
