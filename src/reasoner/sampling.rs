use rand::rng;
use rand::seq::SliceRandom;

pub(super) fn sample_random<T>(mut candidates: Vec<T>, size: usize) -> Vec<T> {
    candidates.shuffle(&mut rng());
    candidates.truncate(size);
    candidates
}
