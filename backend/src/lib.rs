// --- File: backend/src/lib.rs ---

// This file acts as the entry point for the `backend` library.
// By declaring `web_server` as a public module here, we make its
// contents available to other crates, like our integration test.
pub mod auth;
pub mod config;
pub mod error;
pub mod web_server;
