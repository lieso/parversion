
pub fn remove_duplicate_sequences(vec: Vec<String>) -> Vec<String> {
    if vec.is_empty() {
        return vec;
    }

    let mut result = Vec::new();
    let mut iter = vec.into_iter().peekable();

    while let Some(current) = iter.next() {
        result.push(current.clone());

        while let Some(next) = iter.peek() {
            if next == &current {
                iter.next();
            } else {
                break;
            }
        }
    }

    result
}

