//! Migration utilities for handling format changes

use crate::AGCX_MAGIC;
use crate::FORMAT_VERSION;
use crate::error::PersistenceError;
use crate::error::Result;
use std::fs::File;
use std::fs::{self};
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use tracing::info;
use tracing::warn;

/// Migration manager for handling format upgrades
pub struct MigrationManager {
    base_path: PathBuf,
}

impl MigrationManager {
    /// Create a new migration manager
    pub const fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Check if migration is needed
    pub fn check_migration_needed(&self) -> Result<Option<MigrationPlan>> {
        // Check for old Codex format
        let old_agcodex_path = self
            .base_path
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join(".codex"));

        if let Some(ref path) = old_agcodex_path
            && path.exists()
        {
            info!("Found old Codex data at {:?}", path);
            return Ok(Some(MigrationPlan::FromCodex {
                source_path: path.clone(),
            }));
        }

        // Check for version mismatch in existing AGCodex data
        if let Ok(entries) = fs::read_dir(&self.base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) == Some("agcx")
                    && let Ok(version) = self.read_file_version(&path)
                    && version != FORMAT_VERSION
                {
                    return Ok(Some(MigrationPlan::VersionUpgrade {
                        from_version: version,
                        to_version: FORMAT_VERSION,
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Perform migration if needed
    pub async fn migrate(&self, plan: MigrationPlan) -> Result<MigrationReport> {
        match plan {
            MigrationPlan::FromCodex { source_path } => self.migrate_from_codex(&source_path).await,
            MigrationPlan::VersionUpgrade {
                from_version,
                to_version,
            } => self.migrate_version(from_version, to_version).await,
        }
    }

    /// Migrate from old Codex format to AGCodex
    async fn migrate_from_codex(&self, source_path: &Path) -> Result<MigrationReport> {
        info!("Starting migration from Codex to AGCodex");

        let mut report = MigrationReport::new();

        // Create backup directory
        let backup_path = self.base_path.join("migration_backup");
        fs::create_dir_all(&backup_path)?;

        // TODO: Implement actual Codex format reading and conversion
        // For now, we'll create a placeholder implementation

        // Look for conversation files in old format
        let conversations_path = source_path.join("conversations");
        if conversations_path.exists() {
            let entries = fs::read_dir(&conversations_path)?;

            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    match self.convert_agcodex_session(&path).await {
                        Ok(session_id) => {
                            report.sessions_migrated += 1;
                            info!("Migrated session: {}", session_id);
                        }
                        Err(e) => {
                            report.sessions_failed += 1;
                            report
                                .errors
                                .push(format!("Failed to migrate {:?}: {}", path, e));
                            warn!("Failed to migrate session from {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        report.success = report.sessions_failed == 0;
        Ok(report)
    }

    /// Convert a single Codex session to AGCodex format
    async fn convert_agcodex_session(&self, _path: &Path) -> Result<uuid::Uuid> {
        // TODO: Implement actual conversion logic
        // This would involve:
        // 1. Reading the old format
        // 2. Converting to new types
        // 3. Saving in AGCodex format

        // Placeholder: return a new UUID
        Ok(uuid::Uuid::new_v4())
    }

    /// Migrate between AGCodex format versions
    async fn migrate_version(&self, from_version: u16, to_version: u16) -> Result<MigrationReport> {
        info!("Migrating from version {} to {}", from_version, to_version);

        let mut report = MigrationReport::new();

        // Create backup
        let backup_path = self.base_path.join(format!("backup_v{}", from_version));
        fs::create_dir_all(&backup_path)?;

        // List all session files
        let entries = fs::read_dir(&self.base_path)?;

        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().and_then(|ext| ext.to_str()) == Some("agcx") {
                // Backup the file
                let file_name = path.file_name().unwrap();
                let backup_file = backup_path.join(file_name);
                fs::copy(&path, &backup_file)?;

                // Perform version-specific migration
                match self.migrate_file_version(&path, from_version, to_version) {
                    Ok(_) => {
                        report.sessions_migrated += 1;
                        info!("Migrated {:?}", path);
                    }
                    Err(e) => {
                        report.sessions_failed += 1;
                        report
                            .errors
                            .push(format!("Failed to migrate {:?}: {}", path, e));
                        warn!("Failed to migrate {:?}: {}", path, e);
                    }
                }
            }
        }

        report.success = report.sessions_failed == 0;
        Ok(report)
    }

    /// Migrate a single file between versions
    fn migrate_file_version(
        &self,
        _path: &Path,
        from_version: u16,
        _to_version: u16,
    ) -> Result<()> {
        // Version-specific migration logic would go here
        match from_version {
            0 => {
                // Migration from version 0 to current
                // This would involve reading the old format and converting
                warn!("Migration from version 0 not yet implemented");
                Ok(())
            }
            _ => {
                // Unknown version
                Err(PersistenceError::MigrationRequired(
                    from_version,
                    FORMAT_VERSION,
                ))
            }
        }
    }

    /// Read the version from a session file
    fn read_file_version(&self, path: &Path) -> Result<u16> {
        let mut file = File::open(path)?;
        let mut buffer = [0u8; 6];
        file.read_exact(&mut buffer)?;

        // Check magic bytes
        if &buffer[0..4] != AGCX_MAGIC {
            return Err(PersistenceError::InvalidMagic);
        }

        // Read version
        Ok(u16::from_le_bytes([buffer[4], buffer[5]]))
    }

    /// Create a backup of all current data
    pub fn create_backup(&self, name: &str) -> Result<PathBuf> {
        let backup_path = self.base_path.join(format!("backups/{}", name));
        fs::create_dir_all(&backup_path)?;

        // Copy all session files
        let entries = fs::read_dir(&self.base_path)?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name().unwrap();
                let backup_file = backup_path.join(file_name);
                fs::copy(&path, &backup_file)?;
            }
        }

        info!("Created backup at {:?}", backup_path);
        Ok(backup_path)
    }

    /// Restore from a backup
    pub fn restore_backup(&self, backup_path: &Path) -> Result<()> {
        if !backup_path.exists() {
            return Err(PersistenceError::PathNotFound(
                backup_path.to_string_lossy().to_string(),
            ));
        }

        // Clear current data
        let entries = fs::read_dir(&self.base_path)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("agcx") {
                fs::remove_file(&path)?;
            }
        }

        // Copy backup files
        let backup_entries = fs::read_dir(backup_path)?;
        for entry in backup_entries.flatten() {
            let source = entry.path();
            if source.is_file() {
                let file_name = source.file_name().unwrap();
                let dest = self.base_path.join(file_name);
                fs::copy(&source, &dest)?;
            }
        }

        info!("Restored from backup at {:?}", backup_path);
        Ok(())
    }
}

/// Migration plan describing what needs to be done
#[derive(Debug, Clone)]
pub enum MigrationPlan {
    /// Migrate from old Codex format
    FromCodex { source_path: PathBuf },
    /// Upgrade between AGCodex versions
    VersionUpgrade { from_version: u16, to_version: u16 },
}

/// Report of migration results
#[derive(Debug, Clone, Default)]
pub struct MigrationReport {
    pub success: bool,
    pub sessions_migrated: usize,
    pub sessions_failed: usize,
    pub errors: Vec<String>,
    pub backup_path: Option<PathBuf>,
}

impl MigrationReport {
    fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_check_migration_needed() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MigrationManager::new(temp_dir.path().to_path_buf());

        // Should return None when no migration needed
        let result = manager.check_migration_needed().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_backup_and_restore() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();
        let manager = MigrationManager::new(base_path.clone());

        // Create some test files
        let test_file = base_path.join("test.agcx");
        use std::io::Write;
        File::create(&test_file)
            .unwrap()
            .write_all(b"test data")
            .unwrap();

        // Create backup
        let backup_path = manager.create_backup("test_backup").unwrap();
        assert!(backup_path.exists());

        // Delete original file
        fs::remove_file(&test_file).unwrap();
        assert!(!test_file.exists());

        // Restore from backup
        manager.restore_backup(&backup_path).unwrap();
        assert!(test_file.exists());

        // Verify content
        let mut content = String::new();
        File::open(&test_file)
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "test data");
    }
}
