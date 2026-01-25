//! MCP (Model Context Protocol) Server Implementation for Serena
//!
//! This module provides a pure Rust MCP server that exposes Serena's
//! semantic code analysis tools to AI agents like Claude.

pub mod error;
pub mod handler;
pub mod server;
pub mod tools;

pub use error::{SerenaError, SerenaResult};
pub use handler::SerenaServerHandler;
pub use server::{server_details, start_server};
