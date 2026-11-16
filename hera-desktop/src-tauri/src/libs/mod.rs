pub mod database;
pub mod db_agent_queries;
pub mod db_offline_queries;
pub mod db_scanner_queries;
pub mod db_scanner_row;
pub mod error;
pub mod events;
pub mod fast_library_scan;
pub mod init;
pub mod omkf_sync;
pub mod tray;
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

#[cfg(test)]
#[path = "./tests/zero_io_and_discovery_tests.rs"]
mod zero_io_and_discovery_tests;
