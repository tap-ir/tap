//! The main error enum used in TAP. 
//! It can handle different type of error.

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum RustructError
{
  #[error("Plugin {name} not found")]
  PluginNotFound { name : String, },

  #[error("Same plugin with same argument already runned")]
  PluginAlreadyRunned,

  #[error("Plugin {0} error {1}")]
  PluginError(&'static str, &'static str),

  #[error("Task {0} not finished yet")]
  TaskNotFinished(u32),

  #[error("Task {0} not found")] 
  TaskNotFound(u32),

  #[error("Result for task {0} not found")]
  ResultNotFound(u32),

  #[error("Argument {0} not found")]
  ArgumentNotFound(&'static str),

  #[error("Value {0} not found")]
  ValueNotFound(&'static str),

  #[error("Value Type mismatch")]
  ValueTypeMismatch, 

  #[error("Path {path} not found")]
  VFileBuilderPathNotFound{ path : &'static str, },

  #[error("Error opening file {0}")]
  OpenFile(String),

  #[error("Error {0}")]
  Unknown(String),
}
