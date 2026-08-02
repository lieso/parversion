use std::sync::Arc;
use rand::seq::SliceRandom;
use rand::rng;

use crate::context::Context;

const CAP: usize = 100;

pub(super) fn pre_sample_context_group(mut group: Vec<Arc<Context>>) -> Vec<Arc<Context>> {
    if group.len() <= CAP {
        return group;
    }

    group.shuffle(&mut rng());
    group.truncate(CAP);
    group
}
