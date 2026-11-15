pub mod database;
pub mod db_offline_queries;
pub mod db_scanner_queries;
pub mod error;
pub mod events;
pub mod fast_library_scan;
pub mod init;
pub mod utils;

/**
 * Structs
 */
pub mod playlist;
pub mod track;

/**
 * Tests
 */
#[cfg(test)]
#[path = "./tests/database_tests.rs"]
mod database_tests;
