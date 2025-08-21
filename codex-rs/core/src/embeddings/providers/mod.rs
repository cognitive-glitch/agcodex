//! Embedding provider implementations

pub mod gemini;
pub mod openai;
pub mod voyage;

pub use gemini::GeminiProvider;
pub use openai::OpenAIProvider;
pub use voyage::VoyageProvider;
