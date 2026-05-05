//! Knowledge module
//!
//! KnowledgeStore (Layer 1 CRUD), KnowledgeAccessResolver, KnowledgeAccessProtocol

pub mod access_protocol;
pub mod access_resolver;
pub mod reveal;
pub mod store;

pub use access_protocol::KnowledgeAccessProtocol;
pub use access_resolver::KnowledgeAccessResolver;
pub use reveal::KnowledgeRevealHandler;
pub use store::KnowledgeStore;
