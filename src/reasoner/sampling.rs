use std::sync::Arc;
use rand::seq::SliceRandom;
use rand::rng;

use crate::context::Context;

const CAP: usize = 50;

pub(super) fn pre_sample_context_group(mut group: Vec<Arc<Context>>) -> Vec<Arc<Context>> {
    if group.len() <= CAP {
        return group;
    }

    group.shuffle(&mut rng());
    group.truncate(CAP);
    group
}

pub(super) fn sample_most_different(candidates: Vec<String>, embeddings: &[Vec<f32>]) -> Vec<String> {
    let n = candidates.len();

    // *************************
    let min_samples = 5;
    let threshold = 0.2;
    let max_samples = 20;
    // *************************

    if n < min_samples {
        return candidates;
    }

    let mut selected = vec![0usize];
    let mut min_dists: Vec<f32> = embeddings.iter()
        .map(|e| cosine_distance(e, &embeddings[0]))
        .collect();
    min_dists[0] = 0.0;
    
    loop {
        let (next, &dist) = min_dists.iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        if dist < threshold && selected.len() >= min_samples {
            break;
        }

        selected.push(next);

        if selected.len() >= max_samples {
            break;
        }

        for (i, d) in min_dists.iter_mut().enumerate() {
            let new_d = cosine_distance(&embeddings[i], &embeddings[next]);
            if new_d < *d {
                *d = new_d;
            }
        }
        min_dists[next] = 0.0;
    }

    selected.iter().map(|index| candidates[*index].clone()).collect()
}

fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    1.0 - dot
}

