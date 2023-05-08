use rand::distributions::Alphanumeric;
use rand::Rng;

/// Generate a random string of a given size
pub fn rand_str(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
