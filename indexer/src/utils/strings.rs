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

#[cfg(test)]
mod tests {
    use super::truncate_log;

    #[test]
    fn truncate_log_no_change_when_under_limit() {
        let value = "alpha";
        assert_eq!(truncate_log(value, 10), value);
    }

    #[test]
    fn truncate_log_respects_char_boundary() {
        let value = "a✓b";
        assert_eq!(truncate_log(value, 2), "a");
        assert_eq!(truncate_log(value, 3), "a");
        assert_eq!(truncate_log(value, 4), "a✓");
    }
}
