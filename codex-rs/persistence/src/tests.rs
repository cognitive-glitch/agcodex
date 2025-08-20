//! Integration tests for the persistence module

#[cfg(test)]
mod tests {
    use crate::compression::CompressionLevel;
    use crate::compression::Compressor;
    // use crate::error::Result; // unused
    use crate::types::SessionIndex;
    use crate::types::SessionMetadata;
    use chrono::Utc;
    // use tempfile::TempDir; // unused
    use uuid::Uuid;

    #[test]
    fn test_compression_works() {
        let compressor = Compressor::new(CompressionLevel::Balanced);
        let data = b"This is test data for AGCodex session persistence!";

        let compressed = compressor.compress(data).unwrap();
        assert!(compressed.len() < data.len());

        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_session_index_operations() {
        let mut index = SessionIndex::new();

        let id = Uuid::new_v4();
        let metadata = SessionMetadata {
            id,
            title: "Test Session".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_accessed: Utc::now(),
            message_count: 5,
            turn_count: 3,
            current_mode: crate::types::OperatingMode::Build,
            model: "gpt-5".to_string(),
            tags: vec!["test".to_string(), "demo".to_string()],
            is_favorite: true,
            file_size: 1024,
            compression_ratio: 0.7,
            format_version: crate::FORMAT_VERSION,
            checkpoints: Vec::new(),
        };

        // Test adding session
        index.add_session(metadata.clone());
        assert_eq!(index.sessions.len(), 1);
        assert!(index.favorite_sessions.contains(&id));
        assert_eq!(index.recent_sessions[0], id);

        // Test search
        let results = index.search("Test");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);

        // Test tag search
        let results = index.search("demo");
        assert_eq!(results.len(), 1);

        // Test removal
        let removed = index.remove_session(&id);
        assert!(removed.is_some());
        assert_eq!(index.sessions.len(), 0);
        assert!(!index.favorite_sessions.contains(&id));
    }

    #[test]
    fn test_compression_levels() {
        let data = vec![b'X'; 10000]; // Highly compressible

        let fast = Compressor::new(CompressionLevel::Fast);
        let balanced = Compressor::new(CompressionLevel::Balanced);
        let maximum = Compressor::new(CompressionLevel::Maximum);

        let fast_result = fast.compress(&data).unwrap();
        let balanced_result = balanced.compress(&data).unwrap();
        let maximum_result = maximum.compress(&data).unwrap();

        // Maximum should compress better than fast
        assert!(maximum_result.len() <= fast_result.len());

        // All should decompress correctly
        assert_eq!(fast.decompress(&fast_result).unwrap(), data);
        assert_eq!(balanced.decompress(&balanced_result).unwrap(), data);
        assert_eq!(maximum.decompress(&maximum_result).unwrap(), data);
    }

    #[test]
    fn test_session_metadata_serialization() {
        let metadata = SessionMetadata {
            id: Uuid::new_v4(),
            title: "Serialization Test".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_accessed: Utc::now(),
            message_count: 10,
            turn_count: 5,
            current_mode: crate::types::OperatingMode::Review,
            model: "o3".to_string(),
            tags: vec!["test".to_string()],
            is_favorite: false,
            file_size: 2048,
            compression_ratio: 0.8,
            format_version: crate::FORMAT_VERSION,
            checkpoints: Vec::new(),
        };

        // Test bincode serialization
        let bytes = bincode::serde::encode_to_vec(&metadata, bincode::config::standard()).unwrap();
        let (deserialized, _): (SessionMetadata, _) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap();

        assert_eq!(metadata.id, deserialized.id);
        assert_eq!(metadata.title, deserialized.title);
        assert_eq!(metadata.message_count, deserialized.message_count);
    }
}

// Add a dummy OperatingMode for testing since core won't compile
#[cfg(test)]
mod types {
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum OperatingMode {
        Plan,
        Build,
        Review,
    }
}
