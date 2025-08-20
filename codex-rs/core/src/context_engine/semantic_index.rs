//! Semantic index scaffolding for the Context Engine.
//! Provides placeholder types and an in-memory stub.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u64);

#[derive(Debug, Clone, Default)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    pub location: Option<(String, u32, u32)>, // file, line, column
}

#[derive(Debug, Default)]
pub struct SemanticIndex {
    files: HashMap<FileId, String>,
    symbols: HashMap<FileId, Vec<SymbolInfo>>,
}

impl SemanticIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(&mut self, id: FileId, path: String) {
        self.files.insert(id, path);
    }

    pub fn add_symbol(&mut self, file: FileId, sym: SymbolInfo) {
        self.symbols.entry(file).or_default().push(sym);
    }

    pub fn file_path(&self, id: FileId) -> Option<&str> {
        self.files.get(&id).map(|s| s.as_str())
    }

    pub fn symbols_in_file(&self, id: FileId) -> &[SymbolInfo] {
        self.symbols.get(&id).map(Vec::as_slice).unwrap_or(&[])
    }
}
