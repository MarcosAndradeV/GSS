//! Unified diff generation for GSS formatting.
//!
//! Inspired by diff implementations in `rustfmt`, `gofmt -d`, and `git diff`.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiffOp {
    Equal,
    Delete,
    Insert,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine<'a> {
    pub op: DiffOp,
    pub text: &'a str,
}

/// Computes the diff between two sets of lines using LCS.
pub fn compute_line_diff<'a>(orig: &[&'a str], fmt: &[&'a str]) -> Vec<DiffLine<'a>> {
    let n = orig.len();
    let m = fmt.len();

    // Fast path: common prefix
    let mut prefix = 0;
    while prefix < n && prefix < m && orig[prefix] == fmt[prefix] {
        prefix += 1;
    }

    // Fast path: common suffix
    let mut suffix = 0;
    while suffix < (n - prefix) && suffix < (m - prefix) && orig[n - 1 - suffix] == fmt[m - 1 - suffix] {
        suffix += 1;
    }

    let mut result = Vec::with_capacity(n + m);
    for &line in &orig[..prefix] {
        result.push(DiffLine {
            op: DiffOp::Equal,
            text: line,
        });
    }

    let mid_orig = &orig[prefix..n - suffix];
    let mid_fmt = &fmt[prefix..m - suffix];
    let mid_n = mid_orig.len();
    let mid_m = mid_fmt.len();

    if mid_n > 0 && mid_m > 0 {
        // DP table for LCS
        let mut dp = vec![vec![0u32; mid_m + 1]; mid_n + 1];
        for i in 0..mid_n {
            for j in 0..mid_m {
                if mid_orig[i] == mid_fmt[j] {
                    dp[i + 1][j + 1] = dp[i][j] + 1;
                } else {
                    dp[i + 1][j + 1] = dp[i + 1][j].max(dp[i][j + 1]);
                }
            }
        }

        let mut i = mid_n;
        let mut j = mid_m;
        let mut mid_diff = Vec::new();

        while i > 0 || j > 0 {
            if i > 0 && j > 0 && mid_orig[i - 1] == mid_fmt[j - 1] {
                mid_diff.push(DiffLine {
                    op: DiffOp::Equal,
                    text: mid_orig[i - 1],
                });
                i -= 1;
                j -= 1;
            } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
                mid_diff.push(DiffLine {
                    op: DiffOp::Insert,
                    text: mid_fmt[j - 1],
                });
                j -= 1;
            } else if i > 0 {
                mid_diff.push(DiffLine {
                    op: DiffOp::Delete,
                    text: mid_orig[i - 1],
                });
                i -= 1;
            }
        }

        mid_diff.reverse();
        result.extend(mid_diff);
    } else {
        for &line in mid_orig {
            result.push(DiffLine {
                op: DiffOp::Delete,
                text: line,
            });
        }
        for &line in mid_fmt {
            result.push(DiffLine {
                op: DiffOp::Insert,
                text: line,
            });
        }
    }

    for &line in &orig[n - suffix..] {
        result.push(DiffLine {
            op: DiffOp::Equal,
            text: line,
        });
    }

    result
}

struct Hunk<'a> {
    orig_start: usize,
    orig_count: usize,
    fmt_start: usize,
    fmt_count: usize,
    lines: Vec<DiffLine<'a>>,
}

/// Generates a unified diff string between `original` and `formatted`.
///
/// If `color` is true, ANSI escape sequences will be used for colorizing output:
/// - Cyan for hunk headers (`@@ ... @@`)
/// - Red for deletions (`-`)
/// - Green for additions (`+`)
/// - Bold for file headers (`---`, `+++`)
pub fn unified_diff(
    original: &str,
    formatted: &str,
    orig_label: &str,
    fmt_label: &str,
    color: bool,
) -> String {
    if original == formatted {
        return String::new();
    }

    let orig_lines: Vec<&str> = original.lines().collect();
    let fmt_lines: Vec<&str> = formatted.lines().collect();

    let diff_lines = compute_line_diff(&orig_lines, &fmt_lines);

    // Group into hunks with 3 lines of context
    const CONTEXT: usize = 3;
    let mut hunks: Vec<Hunk> = Vec::new();

    let mut i = 0;
    let mut orig_line_no = 1;
    let mut fmt_line_no = 1;

    while i < diff_lines.len() {
        // Find next change
        while i < diff_lines.len() && diff_lines[i].op == DiffOp::Equal {
            orig_line_no += 1;
            fmt_line_no += 1;
            i += 1;
        }

        if i >= diff_lines.len() {
            break;
        }

        // We found a change at i. Determine context start
        let context_start = i.saturating_sub(CONTEXT);
        let orig_start = orig_line_no - (i - context_start);
        let fmt_start = fmt_line_no - (i - context_start);

        let mut hunk_lines = Vec::new();
        for item in &diff_lines[context_start..i] {
            hunk_lines.push(item.clone());
        }

        let mut trailing_equals = 0;
        let mut cur_orig_count = i - context_start;
        let mut cur_fmt_count = i - context_start;

        while i < diff_lines.len() {
            let line = &diff_lines[i];
            match line.op {
                DiffOp::Equal => {
                    trailing_equals += 1;
                    cur_orig_count += 1;
                    cur_fmt_count += 1;
                    orig_line_no += 1;
                    fmt_line_no += 1;
                    hunk_lines.push(line.clone());
                    if trailing_equals > CONTEXT * 2 && i + 1 < diff_lines.len() {
                        // Lookahead: is there another change soon?
                        let has_change_soon = diff_lines[i + 1..]
                            .iter()
                            .take(CONTEXT)
                            .any(|l| l.op != DiffOp::Equal);
                        if !has_change_soon {
                            // Trim trailing equals down to CONTEXT
                            let to_trim = trailing_equals - CONTEXT;
                            hunk_lines.truncate(hunk_lines.len() - to_trim);
                            cur_orig_count -= to_trim;
                            cur_fmt_count -= to_trim;
                            i += 1;
                            break;
                        }
                    }
                }
                DiffOp::Delete => {
                    trailing_equals = 0;
                    cur_orig_count += 1;
                    orig_line_no += 1;
                    hunk_lines.push(line.clone());
                }
                DiffOp::Insert => {
                    trailing_equals = 0;
                    cur_fmt_count += 1;
                    fmt_line_no += 1;
                    hunk_lines.push(line.clone());
                }
            }
            i += 1;
        }

        if trailing_equals > CONTEXT {
            let to_trim = trailing_equals - CONTEXT;
            hunk_lines.truncate(hunk_lines.len() - to_trim);
            cur_orig_count -= to_trim;
            cur_fmt_count -= to_trim;
        }

        hunks.push(Hunk {
            orig_start,
            orig_count: cur_orig_count,
            fmt_start,
            fmt_count: cur_fmt_count,
            lines: hunk_lines,
        });
    }

    if hunks.is_empty() {
        return String::new();
    }

    let mut out = String::new();

    let (c_bold, c_cyan, c_red, c_green, c_reset) = if color {
        ("\x1b[1m", "\x1b[36m", "\x1b[31m", "\x1b[32m", "\x1b[0m")
    } else {
        ("", "", "", "", "")
    };

    out.push_str(&format!("{c_bold}--- {orig_label}{c_reset}\n"));
    out.push_str(&format!("{c_bold}+++ {fmt_label}{c_reset}\n"));

    for hunk in hunks {
        let orig_count_str = if hunk.orig_count == 1 {
            format!("{}", hunk.orig_start)
        } else {
            format!("{},{}", hunk.orig_start, hunk.orig_count)
        };
        let fmt_count_str = if hunk.fmt_count == 1 {
            format!("{}", hunk.fmt_start)
        } else {
            format!("{},{}", hunk.fmt_start, hunk.fmt_count)
        };

        out.push_str(&format!(
            "{c_cyan}@@ -{orig_count_str} +{fmt_count_str} @@{c_reset}\n"
        ));

        for line in hunk.lines {
            match line.op {
                DiffOp::Equal => {
                    out.push(' ');
                    out.push_str(line.text);
                    out.push('\n');
                }
                DiffOp::Delete => {
                    out.push_str(c_red);
                    out.push('-');
                    out.push_str(line.text);
                    out.push_str(c_reset);
                    out.push('\n');
                }
                DiffOp::Insert => {
                    out.push_str(c_green);
                    out.push('+');
                    out.push_str(line.text);
                    out.push_str(c_reset);
                    out.push('\n');
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_diff_identical() {
        let content = "a = 1,\nb = 2,\n";
        assert_eq!(unified_diff(content, content, "a/test", "b/test", false), "");
    }

    #[test]
    fn test_unified_diff_basic() {
        let orig = "a = 1,\nb = 2,\n";
        let fmt = "a = 1,\nb = 20,\nc = 3,\n";
        let diff = unified_diff(orig, fmt, "a/test.gss", "b/test.gss", false);
        assert!(diff.contains("--- a/test.gss"));
        assert!(diff.contains("+++ b/test.gss"));
        assert!(diff.contains("@@ -1,2 +1,3 @@"));
        assert!(diff.contains("-b = 2"));
        assert!(diff.contains("+b = 20"));
        assert!(diff.contains("+c = 3"));
    }

    #[test]
    fn test_unified_diff_colors() {
        let orig = "old = 1";
        let fmt = "new = 2";
        let diff = unified_diff(orig, fmt, "a/f", "b/f", true);
        assert!(diff.contains("\x1b[31m-old = 1\x1b[0m"));
        assert!(diff.contains("\x1b[32m+new = 2\x1b[0m"));
    }
}
