/// Literal, case-sensitive search helpers (char-index based, unicode safe).
/// Positions are (row, col_char) where col counts chars, not bytes.

/// All occurrences of `query` in `lines`.
pub fn find_all(lines: &[String], query: &str) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (row, line) in lines.iter().enumerate() {
        let mut search_from = 0usize; // byte offset
        while let Some(rel) = line[search_from..].find(query) {
            let byte_idx = search_from + rel;
            let col_char = line[..byte_idx].chars().count();
            out.push((row, col_char));
            // Advance by at least one char to allow overlapping? Move past first char of match.
            let next_char_len = query.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            search_from = byte_idx + next_char_len;
            if search_from > line.len() {
                break;
            }
        }
    }
    out
}

/// Next occurrence at or after (start_row, start_col_char), wrapping if `wrap`.
pub fn find_next(
    lines: &[String],
    query: &str,
    start_row: usize,
    start_col: usize,
    wrap: bool,
) -> Option<(usize, usize)> {
    let all = find_all(lines, query);
    if all.is_empty() {
        return None;
    }
    for pos in &all {
        if pos.0 > start_row || (pos.0 == start_row && pos.1 >= start_col) {
            return Some(*pos);
        }
    }
    if wrap { all.into_iter().next() } else { None }
}

/// Replace all literal occurrences. Returns (new_lines, replacement_count).
pub fn replace_all(
    lines: &[String],
    query: &str,
    replacement: &str,
) -> (Vec<String>, usize) {
    if query.is_empty() {
        return (lines.to_vec(), 0);
    }
    let mut count = 0;
    let out: Vec<String> = lines
        .iter()
        .map(|l| {
            let c = l.matches(query).count();
            count += c;
            l.replace(query, replacement)
        })
        .collect();
    (out, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_literal_positions() {
        let lines = vec!["hello hello".to_string(), "world".to_string()];
        assert_eq!(find_all(&lines, "hello"), vec![(0, 0), (0, 6)]);
        assert_eq!(find_all(&lines, "z"), Vec::new());
        assert_eq!(find_all(&lines, ""), Vec::new());
    }

    #[test]
    fn finds_unicode_char_cols() {
        let lines = vec!["a宽b宽c".to_string()];
        // 宽 is one char each
        assert_eq!(find_all(&lines, "宽"), vec![(0, 1), (0, 3)]);
    }

    #[test]
    fn next_wraps() {
        let lines = vec!["aa".to_string(), "aa".to_string()];
        assert_eq!(find_next(&lines, "aa", 0, 0, false), Some((0, 0)));
        assert_eq!(find_next(&lines, "aa", 0, 1, false), Some((1, 0)));
        assert_eq!(find_next(&lines, "aa", 1, 1, false), None);
        assert_eq!(find_next(&lines, "aa", 1, 1, true), Some((0, 0)));
    }

    #[test]
    fn replace_counts() {
        let lines = vec!["foo foo".to_string(), "bar".to_string()];
        let (out, n) = replace_all(&lines, "foo", "x");
        assert_eq!(n, 2);
        assert_eq!(out, vec!["x x".to_string(), "bar".to_string()]);
    }
}
