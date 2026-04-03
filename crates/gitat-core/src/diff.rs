use crate::GitError;
use crate::runner::CommandRunner;

#[derive(Debug, Clone, PartialEq)]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
    pub old_line_no: Option<u32>,
    pub new_line_no: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffFile {
    pub old_path: String,
    pub new_path: String,
    pub hunks: Vec<DiffHunk>,
}

fn parse_range(s: &str) -> (u32, u32) {
    if let Some(comma_pos) = s.find(',') {
        let start = s[..comma_pos].parse::<u32>().unwrap_or(0);
        let count = s[comma_pos + 1..].parse::<u32>().unwrap_or(1);
        (start, count)
    } else {
        let start = s.parse::<u32>().unwrap_or(0);
        (start, 1)
    }
}

pub fn parse_diff(output: &str) -> Result<Vec<DiffFile>, GitError> {
    if output.is_empty() {
        return Ok(Vec::new());
    }

    let mut files: Vec<DiffFile> = Vec::new();

    // Current file being built
    let mut cur_old_path: Option<String> = None;
    let mut cur_new_path: Option<String> = None;
    let mut cur_hunks: Vec<DiffHunk> = Vec::new();

    // Current hunk being built
    let mut cur_hunk: Option<DiffHunk> = None;
    let mut old_line: u32 = 0;
    let mut new_line: u32 = 0;

    let flush_file = |files: &mut Vec<DiffFile>,
                      old_path: &mut Option<String>,
                      new_path: &mut Option<String>,
                      hunks: &mut Vec<DiffHunk>| {
        if let (Some(op), Some(np)) = (old_path.take(), new_path.take()) {
            files.push(DiffFile {
                old_path: op,
                new_path: np,
                hunks: std::mem::take(hunks),
            });
        }
    };

    for line in output.lines() {
        if line.starts_with("diff --git ") {
            // Flush current hunk into hunks vec
            if let Some(h) = cur_hunk.take() {
                cur_hunks.push(h);
            }
            // Flush current file
            flush_file(&mut files, &mut cur_old_path, &mut cur_new_path, &mut cur_hunks);
            // A new file entry begins; paths will be set by --- and +++ lines
        } else if line.starts_with("--- ") {
            let path_str = &line[4..];
            let path = if path_str == "/dev/null" {
                "/dev/null".to_string()
            } else if let Some(stripped) = path_str.strip_prefix("a/") {
                stripped.to_string()
            } else {
                path_str.to_string()
            };
            cur_old_path = Some(path);
        } else if line.starts_with("+++ ") {
            let path_str = &line[4..];
            let path = if path_str == "/dev/null" {
                "/dev/null".to_string()
            } else if let Some(stripped) = path_str.strip_prefix("b/") {
                stripped.to_string()
            } else {
                path_str.to_string()
            };
            cur_new_path = Some(path);
        } else if line.starts_with("@@ ") {
            // Flush previous hunk
            if let Some(h) = cur_hunk.take() {
                cur_hunks.push(h);
            }
            // Parse @@ -old_start,old_count +new_start,new_count @@
            // Example: "@@ -1,3 +1,4 @@"
            let inner = line.trim_start_matches('@').trim_start_matches(' ');
            // Find the closing @@ (may have trailing context text)
            let parts: Vec<&str> = inner.splitn(3, ' ').collect();
            // parts[0] = "-1,3", parts[1] = "+1,4"
            let old_part = parts.first().unwrap_or(&"").trim_start_matches('-');
            let new_part = parts.get(1).unwrap_or(&"").trim_start_matches('+');
            let (old_start, old_count) = parse_range(old_part);
            let (new_start, new_count) = parse_range(new_part);

            old_line = old_start;
            new_line = new_start;

            cur_hunk = Some(DiffHunk {
                old_start,
                old_count,
                new_start,
                new_count,
                lines: Vec::new(),
            });
        } else if let Some(ref mut hunk) = cur_hunk {
            if let Some(rest) = line.strip_prefix('+') {
                hunk.lines.push(DiffLine {
                    kind: DiffLineKind::Added,
                    content: rest.to_string(),
                    old_line_no: None,
                    new_line_no: Some(new_line),
                });
                new_line += 1;
            } else if let Some(rest) = line.strip_prefix('-') {
                hunk.lines.push(DiffLine {
                    kind: DiffLineKind::Removed,
                    content: rest.to_string(),
                    old_line_no: Some(old_line),
                    new_line_no: None,
                });
                old_line += 1;
            } else if let Some(rest) = line.strip_prefix(' ') {
                hunk.lines.push(DiffLine {
                    kind: DiffLineKind::Context,
                    content: rest.to_string(),
                    old_line_no: Some(old_line),
                    new_line_no: Some(new_line),
                });
                old_line += 1;
                new_line += 1;
            }
            // Otherwise skip (e.g. "\ No newline at end of file")
        }
        // Skip other header lines (index, new file mode, etc.)
    }

    // Flush last hunk and file
    if let Some(h) = cur_hunk.take() {
        cur_hunks.push(h);
    }
    flush_file(&mut files, &mut cur_old_path, &mut cur_new_path, &mut cur_hunks);

    Ok(files)
}

pub fn get_diff(runner: &dyn CommandRunner, staged: bool) -> Result<Vec<DiffFile>, GitError> {
    let output = if staged {
        runner.run(&["diff", "--cached"])?
    } else {
        runner.run(&["diff"])?
    };
    parse_diff(&output)
}

pub fn get_diff_for_file(
    runner: &dyn CommandRunner,
    path: &str,
    staged: bool,
) -> Result<Vec<DiffFile>, GitError> {
    let output = if staged {
        runner.run(&["diff", "--cached", "--", path])?
    } else {
        runner.run(&["diff", "--", path])?
    };
    parse_diff(&output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_diff() {
        let result = parse_diff("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_single_file_diff() {
        let input = "\
diff --git a/src/main.rs b/src/main.rs
index abc123..def456 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
-    println!(\"hello\");
+    let msg = \"hello\";
+    println!(\"{msg}\");
 }
";
        let result = parse_diff(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].old_path, "src/main.rs");
        assert_eq!(result[0].new_path, "src/main.rs");
        assert_eq!(result[0].hunks.len(), 1);

        let hunk = &result[0].hunks[0];
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_count, 3);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_count, 4);
        assert_eq!(hunk.lines.len(), 5);
    }

    #[test]
    fn test_diff_line_numbers() {
        let input = "\
diff --git a/f.rs b/f.rs
--- a/f.rs
+++ b/f.rs
@@ -10,3 +10,4 @@
 context
-removed
+added1
+added2
 context2
";
        let result = parse_diff(input).unwrap();
        let lines = &result[0].hunks[0].lines;

        assert_eq!(lines[0].old_line_no, Some(10));
        assert_eq!(lines[0].new_line_no, Some(10));
        assert_eq!(lines[1].old_line_no, Some(11));
        assert_eq!(lines[1].new_line_no, None);
        assert_eq!(lines[2].old_line_no, None);
        assert_eq!(lines[2].new_line_no, Some(11));
        assert_eq!(lines[3].old_line_no, None);
        assert_eq!(lines[3].new_line_no, Some(12));
        assert_eq!(lines[4].old_line_no, Some(12));
        assert_eq!(lines[4].new_line_no, Some(13));
    }

    #[test]
    fn test_parse_new_file() {
        let input = "\
diff --git a/new.rs b/new.rs
new file mode 100644
--- /dev/null
+++ b/new.rs
@@ -0,0 +1,2 @@
+fn hello() {}
+fn world() {}
";
        let result = parse_diff(input).unwrap();
        assert_eq!(result[0].old_path, "/dev/null");
        assert_eq!(result[0].new_path, "new.rs");
    }
}
