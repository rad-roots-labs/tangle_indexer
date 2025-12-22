pub fn truncate_log(s: &str, max: usize) -> &str {
    if s.len() > max {
        let mut idx = max;
        while idx > 0 && !s.is_char_boundary(idx) {
            idx -= 1;
        }
        &s[..idx]
    } else {
        s
    }
}
