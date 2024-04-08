pub fn chunk_string(s: &str, chunk_size: usize) -> Vec<String> {
    log::trace!("In chunk_string");

    s.chars()
        .collect::<Vec<char>>()
        .chunks(chunk_size)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

pub fn trim_quotes(s: String) -> Option<String> {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 2 {
        None
    } else {
        Some(chars[1..chars.len() - 1].iter().collect())
    }
}
