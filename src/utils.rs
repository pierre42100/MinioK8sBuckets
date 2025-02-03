use rand::distr::{Alphanumeric, SampleString};

/// Generate a random string of a given size
pub fn rand_str(len: usize) -> String {
    Alphanumeric.sample_string(&mut rand::rng(), len)
}
