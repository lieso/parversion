pub fn chunk_string(s: &str, chunk_size: usize) -> Vec<String> {
    log::trace!("In chunk_string");

    s.chars()
        .collect::<Vec<char>>()
        .chunks(chunk_size)
        .map(|chunk| chunk.iter().collect())
        .collect()
}
