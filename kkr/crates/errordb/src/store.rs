//! Error store — persists error records to JSON file.
//!
//! The store automatically deduplicates errors by fingerprint.
//! Only errors with 2+ occurrences are returned by `find_solutions()`.

use crate::record::ErrorRecord;
use crate::Result;
use std::path::PathBuf;

/// The error knowledge base.
#[derive(Debug)]
pub struct ErrorDb {
    records: Vec<ErrorRecord>,
    path: Option<PathBuf>,
}

impl ErrorDb {
    /// Create an in-memory error DB (no persistence).
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            path: None,
        }
    }

    /// Create or load an error DB from a JSON file.
    pub fn from_file(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let records = if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            serde_json::from_str(&data)?
        } else {
            Vec::new()
        };

        Ok(Self {
            records,
            path: Some(path),
        })
    }

    /// Record an error. If we've seen this fingerprint before, bump its counter.
    /// Returns true if this is a recurring error (2+ occurrences).
    pub fn record_error(
        &mut self,
        problem: &str,
        solution: &str,
    ) -> Result<bool> {
        let new_record = ErrorRecord::new(problem, solution);

        // Check for existing record with same fingerprint
        if let Some(existing) = self.records.iter_mut().find(|r| r.fingerprint == new_record.fingerprint) {
            existing.bump();
            // Update solution if a better one is provided
            if !solution.is_empty() && solution != existing.solution {
                existing.solution = solution.to_string();
            }
            let recurring = existing.is_recurring();
            self.save()?;
            return Ok(recurring);
        }

        self.records.push(new_record);
        self.save()?;
        Ok(false) // First occurrence — not recurring yet
    }

    /// Record an error with tags and language context.
    pub fn record_error_with_context(
        &mut self,
        problem: &str,
        solution: &str,
        tags: Vec<String>,
        language: Option<&str>,
        project: Option<&str>,
    ) -> Result<bool> {
        let mut new_record = ErrorRecord::new(problem, solution);
        new_record.tags = tags;
        new_record.language = language.map(|s| s.to_string());
        new_record.project = project.map(|s| s.to_string());

        if let Some(existing) = self.records.iter_mut().find(|r| r.fingerprint == new_record.fingerprint) {
            existing.bump();
            if !solution.is_empty() && solution != existing.solution {
                existing.solution = solution.to_string();
            }
            let recurring = existing.is_recurring();
            self.save()?;
            return Ok(recurring);
        }

        self.records.push(new_record);
        self.save()?;
        Ok(false)
    }

    /// Find solutions for a given error text.
    /// Only returns records with 2+ occurrences (recurring errors).
    pub fn find_solutions(&self, error_text: &str) -> Vec<&ErrorRecord> {
        self.records.iter()
            .filter(|r| r.is_recurring() && r.matches(error_text))
            .collect()
    }

    /// Find all records matching a tag (regardless of occurrence count).
    pub fn by_tag(&self, tag: &str) -> Vec<&ErrorRecord> {
        self.records.iter()
            .filter(|r| r.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// Find all records for a language.
    pub fn by_language(&self, language: &str) -> Vec<&ErrorRecord> {
        self.records.iter()
            .filter(|r| r.language.as_deref() == Some(language))
            .collect()
    }

    /// Get all recurring errors (2+ occurrences).
    pub fn recurring_errors(&self) -> Vec<&ErrorRecord> {
        self.records.iter()
            .filter(|r| r.is_recurring())
            .collect()
    }

    /// Total number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Is the DB empty?
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// All records.
    pub fn all(&self) -> &[ErrorRecord] {
        &self.records
    }

    /// Save to disk (if path is set).
    fn save(&self) -> Result<()> {
        if let Some(ref path) = self.path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let data = serde_json::to_string_pretty(&self.records)?;
            std::fs::write(path, data)?;
        }
        Ok(())
    }
}

impl Default for ErrorDb {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_first_time() {
        let mut db = ErrorDb::new();
        let recurring = db.record_error(
            "Parser crash on empty input",
            "Add empty input check at parser entry",
        ).unwrap();
        assert!(!recurring); // First time — not recurring
        assert_eq!(db.len(), 1);
    }

    #[test]
    fn test_record_recurring() {
        let mut db = ErrorDb::new();

        let r1 = db.record_error("Parser crash on empty input", "Add guard").unwrap();
        assert!(!r1);

        let r2 = db.record_error("Parser crash on empty input", "Add guard").unwrap();
        assert!(r2); // Second time — recurring!
    }

    #[test]
    fn test_find_solutions_only_recurring() {
        let mut db = ErrorDb::new();

        // Record once — should not appear in solutions
        db.record_error("Timeout error", "Increase timeout").unwrap();
        assert!(db.find_solutions("Timeout error").is_empty());

        // Record again — now it should appear
        db.record_error("Timeout error", "Increase timeout").unwrap();
        let solutions = db.find_solutions("Timeout error");
        assert_eq!(solutions.len(), 1);
    }

    #[test]
    fn test_by_tag() {
        let mut db = ErrorDb::new();
        db.record_error_with_context(
            "Connection refused",
            "Start the server first",
            vec!["network".to_string(), "server".to_string()],
            Some("rust"),
            None,
        ).unwrap();

        assert_eq!(db.by_tag("network").len(), 1);
        assert_eq!(db.by_tag("database").len(), 0);
    }

    #[test]
    fn test_persistence() {
        let dir = std::env::temp_dir().join("kkr_errordb_test");
        let path = dir.join("errors.json");

        // Clean up from previous runs
        let _ = std::fs::remove_file(&path);

        {
            let mut db = ErrorDb::from_file(&path).unwrap();
            db.record_error("test error", "test fix").unwrap();
            db.record_error("test error", "test fix").unwrap();
        }

        // Load again — should have the record
        {
            let db = ErrorDb::from_file(&path).unwrap();
            assert_eq!(db.len(), 1);
            assert_eq!(db.all()[0].occurrences, 2);
        }

        // Clean up
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(dir);
    }
}
