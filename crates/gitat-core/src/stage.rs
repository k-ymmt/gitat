use crate::GitError;
use crate::diff::{DiffFile, DiffLineKind};
use crate::runner::CommandRunner;

pub fn stage_file(runner: &dyn CommandRunner, path: &str) -> Result<(), GitError> {
    runner.run(&["add", "--", path])?;
    Ok(())
}

pub fn unstage_file(runner: &dyn CommandRunner, path: &str) -> Result<(), GitError> {
    runner.run(&["reset", "HEAD", "--", path])?;
    Ok(())
}

pub fn stage_hunk(
    runner: &dyn CommandRunner,
    diff_file: &DiffFile,
    hunk_index: usize,
) -> Result<(), GitError> {
    let patch = format_hunk_patch(diff_file, hunk_index)?;
    runner.run_with_stdin(&["apply", "--cached"], &patch)?;
    Ok(())
}

pub fn unstage_hunk(
    runner: &dyn CommandRunner,
    diff_file: &DiffFile,
    hunk_index: usize,
) -> Result<(), GitError> {
    let patch = format_hunk_patch(diff_file, hunk_index)?;
    runner.run_with_stdin(&["apply", "--cached", "--reverse"], &patch)?;
    Ok(())
}

fn format_hunk_patch(diff_file: &DiffFile, hunk_index: usize) -> Result<String, GitError> {
    let hunk = diff_file.hunks.get(hunk_index).ok_or_else(|| {
        GitError::ParseError(format!(
            "hunk index {} out of bounds (file has {} hunks)",
            hunk_index,
            diff_file.hunks.len()
        ))
    })?;

    let mut patch = String::new();

    // File header
    let old_header = if diff_file.old_path == "/dev/null" {
        "/dev/null".to_string()
    } else {
        format!("a/{}", diff_file.old_path)
    };
    let new_header = if diff_file.new_path == "/dev/null" {
        "/dev/null".to_string()
    } else {
        format!("b/{}", diff_file.new_path)
    };

    let diff_old = if diff_file.old_path == "/dev/null" {
        &diff_file.new_path
    } else {
        &diff_file.old_path
    };
    let diff_new = if diff_file.new_path == "/dev/null" {
        &diff_file.old_path
    } else {
        &diff_file.new_path
    };
    patch.push_str(&format!("diff --git a/{} b/{}\n", diff_old, diff_new));
    patch.push_str(&format!("--- {}\n", old_header));
    patch.push_str(&format!("+++ {}\n", new_header));

    // Hunk header
    patch.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
    ));

    // Hunk lines
    for line in &hunk.lines {
        match line.kind {
            DiffLineKind::Context => {
                patch.push(' ');
                patch.push_str(&line.content);
                patch.push('\n');
            }
            DiffLineKind::Added => {
                patch.push('+');
                patch.push_str(&line.content);
                patch.push('\n');
            }
            DiffLineKind::Removed => {
                patch.push('-');
                patch.push_str(&line.content);
                patch.push('\n');
            }
        }
    }

    Ok(patch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{DiffFile, DiffHunk, DiffLine, DiffLineKind};

    fn make_simple_diff_file() -> DiffFile {
        DiffFile {
            old_path: "src/main.rs".to_string(),
            new_path: "src/main.rs".to_string(),
            hunks: vec![DiffHunk {
                old_start: 1,
                old_count: 3,
                new_start: 1,
                new_count: 4,
                lines: vec![
                    DiffLine {
                        kind: DiffLineKind::Context,
                        content: "fn main() {".to_string(),
                        old_line_no: Some(1),
                        new_line_no: Some(1),
                    },
                    DiffLine {
                        kind: DiffLineKind::Removed,
                        content: "    println!(\"hello\");".to_string(),
                        old_line_no: Some(2),
                        new_line_no: None,
                    },
                    DiffLine {
                        kind: DiffLineKind::Added,
                        content: "    let msg = \"hello\";".to_string(),
                        old_line_no: None,
                        new_line_no: Some(2),
                    },
                    DiffLine {
                        kind: DiffLineKind::Added,
                        content: "    println!(\"{msg}\");".to_string(),
                        old_line_no: None,
                        new_line_no: Some(3),
                    },
                    DiffLine {
                        kind: DiffLineKind::Context,
                        content: "}".to_string(),
                        old_line_no: Some(3),
                        new_line_no: Some(4),
                    },
                ],
            }],
        }
    }

    #[test]
    fn test_format_hunk_patch_normal() {
        let diff_file = make_simple_diff_file();
        let patch = format_hunk_patch(&diff_file, 0).unwrap();
        let expected = "\
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
-    println!(\"hello\");
+    let msg = \"hello\";
+    println!(\"{msg}\");
 }
";
        assert_eq!(patch, expected);
    }

    #[test]
    fn test_format_hunk_patch_new_file() {
        let diff_file = DiffFile {
            old_path: "/dev/null".to_string(),
            new_path: "new.rs".to_string(),
            hunks: vec![DiffHunk {
                old_start: 0,
                old_count: 0,
                new_start: 1,
                new_count: 2,
                lines: vec![
                    DiffLine {
                        kind: DiffLineKind::Added,
                        content: "fn hello() {}".to_string(),
                        old_line_no: None,
                        new_line_no: Some(1),
                    },
                    DiffLine {
                        kind: DiffLineKind::Added,
                        content: "fn world() {}".to_string(),
                        old_line_no: None,
                        new_line_no: Some(2),
                    },
                ],
            }],
        };
        let patch = format_hunk_patch(&diff_file, 0).unwrap();
        assert!(patch.starts_with("diff --git a/new.rs b/new.rs\n"));
        assert!(patch.contains("--- /dev/null\n"));
        assert!(patch.contains("+++ b/new.rs\n"));
    }

    #[test]
    fn test_format_hunk_patch_out_of_bounds() {
        let diff_file = make_simple_diff_file();
        let result = format_hunk_patch(&diff_file, 5);
        assert!(result.is_err());
    }

    #[test]
    fn snapshot_format_hunk_patch() {
        let diff_file = make_simple_diff_file();
        let patch = format_hunk_patch(&diff_file, 0).unwrap();
        insta::assert_snapshot!(patch);
    }

    use crate::runner::MockRunner;

    #[test]
    fn test_stage_hunk() {
        let diff_file = make_simple_diff_file();
        let runner = MockRunner::new().with_response("apply --cached", "");
        let result = stage_hunk(&runner, &diff_file, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unstage_hunk() {
        let diff_file = make_simple_diff_file();
        let runner = MockRunner::new().with_response("apply --cached --reverse", "");
        let result = unstage_hunk(&runner, &diff_file, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_stage_hunk_out_of_bounds() {
        let diff_file = make_simple_diff_file();
        let runner = MockRunner::new();
        let result = stage_hunk(&runner, &diff_file, 99);
        assert!(result.is_err());
    }
}
