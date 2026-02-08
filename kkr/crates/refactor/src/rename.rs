//! Rename refactoring — find and replace symbol names across files.

use std::path::PathBuf;

/// A rename operation to apply to a file.
#[derive(Debug, Clone)]
pub struct RenameChange {
    pub file: PathBuf,
    pub old_name: String,
    pub new_name: String,
    pub locations: Vec<RenameLocation>,
}

/// A specific location where a rename should be applied.
#[derive(Debug, Clone)]
pub struct RenameLocation {
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

impl RenameChange {
    /// Apply this rename to file content.
    /// Returns the modified content.
    pub fn apply(&self, content: &str) -> String {
        // Use word-boundary-aware replacement to avoid partial matches.
        // E.g., renaming "get" should not affect "get_all".
        let mut result = String::new();
        let old = &self.old_name;
        let new = &self.new_name;

        let mut last_end = 0;
        let bytes = content.as_bytes();

        for (i, _) in content.match_indices(old) {
            // Check word boundaries
            let before_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            let after_ok = i + old.len() >= bytes.len() || !is_ident_char(bytes[i + old.len()]);

            if before_ok && after_ok {
                result.push_str(&content[last_end..i]);
                result.push_str(new);
                last_end = i + old.len();
            }
        }

        result.push_str(&content[last_end..]);
        result
    }

    /// Count how many replacements would be made.
    pub fn count_replacements(&self, content: &str) -> usize {
        let old = &self.old_name;
        let bytes = content.as_bytes();
        let mut count = 0;

        for (i, _) in content.match_indices(old) {
            let before_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            let after_ok = i + old.len() >= bytes.len() || !is_ident_char(bytes[i + old.len()]);
            if before_ok && after_ok {
                count += 1;
            }
        }

        count
    }
}

/// Check if a byte is a valid identifier character (alphanumeric or underscore).
fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Create rename changes for all affected files.
pub fn plan_rename(
    old_name: &str,
    new_name: &str,
    files: &[(PathBuf, String)],
) -> Vec<RenameChange> {
    files.iter()
        .filter_map(|(path, content)| {
            let change = RenameChange {
                file: path.clone(),
                old_name: old_name.to_string(),
                new_name: new_name.to_string(),
                locations: Vec::new(),
            };

            let count = change.count_replacements(content);
            if count > 0 {
                Some(change)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_rename() {
        let files = vec![
            (PathBuf::from("a.rs"), "fn process() { process(); }".to_string()),
            (PathBuf::from("b.rs"), "fn other() {}".to_string()),
            (PathBuf::from("c.rs"), "use crate::process;".to_string()),
        ];

        let changes = plan_rename("process", "handle", &files);
        assert_eq!(changes.len(), 2); // a.rs and c.rs
    }
}
