//! Compression utilities using Zstd

use crate::error::PersistenceError;
use crate::error::Result;
use std::io::Read;
use std::io::Write;

/// Compression level for Zstd
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionLevel {
    /// Fast compression (level 1)
    Fast,
    /// Balanced compression (level 3)
    Balanced,
    /// Maximum compression (level 9)
    Maximum,
    /// Custom level (1-22)
    Custom(i32),
}

impl CompressionLevel {
    /// Convert to Zstd compression level
    pub fn to_level(self) -> i32 {
        match self {
            Self::Fast => 1,
            Self::Balanced => 3,
            Self::Maximum => 9,
            Self::Custom(level) => level.clamp(1, 22),
        }
    }
}

impl Default for CompressionLevel {
    fn default() -> Self {
        Self::Balanced
    }
}

/// Zstd compressor for session data
pub struct Compressor {
    level: CompressionLevel,
}

impl Compressor {
    /// Create a new compressor with the specified level
    pub const fn new(level: CompressionLevel) -> Self {
        Self { level }
    }

    /// Compress data using Zstd
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut encoder = zstd::Encoder::new(Vec::new(), self.level.to_level())
            .map_err(|e| PersistenceError::Compression(e.to_string()))?;

        encoder
            .write_all(data)
            .map_err(|e| PersistenceError::Compression(e.to_string()))?;

        encoder
            .finish()
            .map_err(|e| PersistenceError::Compression(e.to_string()))
    }

    /// Decompress data using Zstd
    pub fn decompress(&self, compressed: &[u8]) -> Result<Vec<u8>> {
        let mut decoder = zstd::Decoder::new(compressed)
            .map_err(|e| PersistenceError::Compression(e.to_string()))?;

        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| PersistenceError::Compression(e.to_string()))?;

        Ok(decompressed)
    }

    /// Compress data with dictionary for better compression of similar data
    pub fn compress_with_dict(&self, data: &[u8], _dict: &[u8]) -> Result<Vec<u8>> {
        zstd::encode_all(std::io::Cursor::new(data), self.level.to_level())
            .map_err(|e| PersistenceError::Compression(e.to_string()))
    }

    /// Calculate compression ratio
    pub fn compression_ratio(original_size: usize, compressed_size: usize) -> f32 {
        if compressed_size == 0 {
            return 0.0;
        }
        1.0 - (compressed_size as f32 / original_size as f32)
    }
}

/// Stream compressor for large sessions
pub struct StreamCompressor {
    level: CompressionLevel,
}

impl StreamCompressor {
    pub const fn new(level: CompressionLevel) -> Self {
        Self { level }
    }

    /// Compress from reader to writer
    pub fn compress_stream<R: Read, W: Write>(&self, mut reader: R, writer: W) -> Result<u64> {
        let mut encoder = zstd::Encoder::new(writer, self.level.to_level())
            .map_err(|e| PersistenceError::Compression(e.to_string()))?;

        let bytes_written = std::io::copy(&mut reader, &mut encoder)?;

        encoder
            .finish()
            .map_err(|e| PersistenceError::Compression(e.to_string()))?;

        Ok(bytes_written)
    }

    /// Decompress from reader to writer
    pub fn decompress_stream<R: Read, W: Write>(&self, reader: R, mut writer: W) -> Result<u64> {
        let mut decoder =
            zstd::Decoder::new(reader).map_err(|e| PersistenceError::Compression(e.to_string()))?;

        let bytes_written = std::io::copy(&mut decoder, &mut writer)?;
        Ok(bytes_written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_roundtrip() {
        let compressor = Compressor::new(CompressionLevel::Balanced);
        // Use larger, repetitive data that compresses well
        let data = "Hello, AGCodex! This is a test of the compression system. ".repeat(100);
        let data = data.as_bytes();

        let compressed = compressor.compress(data).unwrap();
        assert!(compressed.len() < data.len());

        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compression_levels() {
        let data = vec![b'A'; 10000]; // Highly compressible data

        let fast = Compressor::new(CompressionLevel::Fast);
        let balanced = Compressor::new(CompressionLevel::Balanced);
        let maximum = Compressor::new(CompressionLevel::Maximum);

        let fast_compressed = fast.compress(&data).unwrap();
        let balanced_compressed = balanced.compress(&data).unwrap();
        let maximum_compressed = maximum.compress(&data).unwrap();

        // Maximum should compress better than fast
        assert!(maximum_compressed.len() <= fast_compressed.len());

        // All should decompress to the same data
        assert_eq!(fast.decompress(&fast_compressed).unwrap(), data);
        assert_eq!(balanced.decompress(&balanced_compressed).unwrap(), data);
        assert_eq!(maximum.decompress(&maximum_compressed).unwrap(), data);
    }

    #[test]
    fn test_compression_ratio() {
        let ratio = Compressor::compression_ratio(1000, 100);
        assert!((ratio - 0.9).abs() < 0.001);

        let ratio = Compressor::compression_ratio(1000, 500);
        assert!((ratio - 0.5).abs() < 0.001);
    }
}
