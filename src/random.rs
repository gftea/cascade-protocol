use std::cell::RefCell;

use rand::distributions::Uniform;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

// We can use lazy_static! to create a global RNG, but that would require
// us to use a Mutex to make it thread-safe. Instead, we use thread_local!
thread_local! {
    static RNG: RefCell<StdRng> = RefCell::new(StdRng::from_entropy());
}

pub(crate) fn set_random_uint32_seed(seed: u32) {
    RNG.with(|rng| {
        let new_rng = StdRng::seed_from_u64(seed as u64);
        rng.replace(new_rng);
    });
}

pub(crate) fn random_uint32() -> u32 {
    RNG.with(|rng| rng.borrow_mut().gen())
}

pub(crate) fn random_bit_nr(start_bit_nr: u32, end_bit_nr: u32) -> u32 {
    let distribution = Uniform::new_inclusive(start_bit_nr, end_bit_nr);
    RNG.with(|rng| rng.borrow_mut().sample(distribution))
}
