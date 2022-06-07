//! # TAP 
//!
//! `TAP` is a library that let you easily represent, transform and analyze data coming from different kind of binary parser.

pub mod session;
pub mod node;
pub mod tree;
pub mod event;
pub mod value;
pub mod attribute;
pub mod reflect;
pub mod plugins_db;
pub mod task_scheduler; 
pub mod vfile;
pub mod mappedvfile;
pub mod zerovfile;
pub mod memoryvfile;
pub mod error;
pub mod plugin;
pub mod plugin_dummy;
pub mod plugin_dummy_singleton;
pub mod datetime;
