use std::ops::Deref;

pub trait Algorithm {
    const MIN_ESTIMATED_BIT_ERR_RATE: f32 = 1e-5;
    fn block_size(iteration_nr: u32, estimated_bit_error_rate: f32, key_size: u32) -> u32;
}

pub struct InnerConfig {
    name: String,
    nr_cascade_iterations: u32,
    nr_biconf_iterations: u32,
    biconf_error_free_streak: bool,
    biconf_correct_complement: bool,
    biconf_cascade: bool,
    ask_correct_parity_using_shuffle_seed: bool,
    cache_shuffles: bool,
}

pub struct OriginalAlgorithm(InnerConfig);

struct BiconfAlgorithm(InnerConfig);

impl Deref for OriginalAlgorithm {
    type Target = InnerConfig;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl OriginalAlgorithm {
    pub fn new(
        name: &str,
        nr_cascade_iterations: u32,
        nr_biconf_iterations: u32,
        biconf_error_free_streak: bool,
        biconf_correct_complement: bool,
        biconf_cascade: bool,
        ask_correct_parity_using_shuffle_seed: bool,
        cache_shuffles: bool,
    ) -> Self {
        Self(InnerConfig {
            name: name.to_string(),
            nr_cascade_iterations,
            nr_biconf_iterations,
            biconf_error_free_streak,
            biconf_correct_complement,
            biconf_cascade,
            ask_correct_parity_using_shuffle_seed,
            cache_shuffles,
        })
    }
}

impl Default for OriginalAlgorithm {
    fn default() -> Self {
        Self::new("original", 4, 0, false, false, false, true, true)
    }
}
impl Algorithm for OriginalAlgorithm {
    fn block_size(iteration_nr: u32, estimated_bit_error_rate: f32, key_size: u32) -> u32 {
        let estimated_bit_error_rate =
            estimated_bit_error_rate.max(Self::MIN_ESTIMATED_BIT_ERR_RATE);
        if iteration_nr == 1 {
            // Casting from a float to an integer will round the float towards zero
            // NaN will return 0
            // Values larger than the maximum integer value, including INFINITY, will saturate to the maximum value of the integer type.
            // Values smaller than the minimum integer value, including NEG_INFINITY, will saturate to the minimum value of the integer type.
            return (0.73 / estimated_bit_error_rate).ceil() as u32;
        }
        2 * Self::block_size(iteration_nr - 1, estimated_bit_error_rate, key_size)
    }
}

#[cfg(test)]
mod tests {
    use crate::algorithm::{Algorithm, OriginalAlgorithm};

    #[test]
    fn test_original_algorithm() {
        let alg = OriginalAlgorithm::default();
        assert_eq!("original", alg.name);
        assert_eq!(4, alg.nr_cascade_iterations);
        assert_eq!(
            73000,
            <OriginalAlgorithm as Algorithm>::block_size(1, 0.0, 10000)
        );
        assert_eq!(
            8,
            <OriginalAlgorithm as Algorithm>::block_size(1, 0.1, 10000)
        );
        assert_eq!(
            73,
            <OriginalAlgorithm as Algorithm>::block_size(1, 0.01, 10000)
        );
        assert_eq!(
            146,
            <OriginalAlgorithm as Algorithm>::block_size(2, 0.01, 10000)
        );
        assert_eq!(
            292,
            <OriginalAlgorithm as Algorithm>::block_size(3, 0.01, 10000)
        );
        assert_eq!(
            730,
            <OriginalAlgorithm as Algorithm>::block_size(1, 0.001, 10000)
        );
        assert_eq!(0, alg.nr_biconf_iterations);
        assert!(!alg.biconf_error_free_streak);
        assert!(!alg.biconf_correct_complement);
        assert!(!alg.biconf_cascade);
        assert!(alg.ask_correct_parity_using_shuffle_seed);
        assert!(alg.cache_shuffles);
    }
}
