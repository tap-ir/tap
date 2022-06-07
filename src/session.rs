//! Session is the main component of this library.
//! it give you access to all the functionality of the library
//! (plugins, taskmanager, the attributes and data tree, ...). 

use std::sync::{Arc};

use crate::tree::{Tree};
use crate::plugins_db::PluginsDB;
use crate::task_scheduler::{TaskScheduler, TaskId};
use crate::plugin::{PluginArgument,PluginResult};
use crate::error::RustructError;

/**
 * Contain instances of structure needed by TAP.
 */
pub struct Session
{
  /// A [PluginsDB] instance
  pub plugins_db : PluginsDB,
  /// A [Tree] instance
  pub tree : Tree,
  /// A [TaskScheduler] instance
  pub task_scheduler : TaskScheduler,
}

impl Session
{
  /// Return a new [Session]
  pub fn new() -> Session
  {
    let tree = Tree::new();
    let task_scheduler = TaskScheduler::new(tree.clone());
    Session{ plugins_db : PluginsDB::new(), tree, task_scheduler }
  }

  /// Replace [tree](Tree) and [task_scheduler](TaskScheduler) by a new intance.
  pub fn clear(&mut self) 
  {
    self.tree = Tree::new();
    self.task_scheduler = TaskScheduler::new(self.tree.clone());
  }

  /// Create a [crate::plugin::PluginInstance] from `plugin_name` and `argument` add it to the scheduler and return it's task id.
  pub fn schedule(&self, plugin_name : &str, argument : PluginArgument, relaunch : bool) -> Result<TaskId, anyhow::Error>
  {
    let plugin = match self.plugins_db.find(plugin_name)
    {
      Some(plugin) => plugin,
      None => return Err(RustructError::PluginNotFound{ name : plugin_name.into()}.into()),
    };
    let plugin = plugin.instantiate();
        
    self.task_scheduler.schedule(plugin, argument, relaunch)
  }

  /// Create a [crate::plugin::PluginInstance], add it to an available worker, wait for it to be executed  and return the results.
  /// This function is blocking the [TaskScheduler], so must be avoided in multithreaded code.
  pub fn run(&self, plugin_name : &str, argument : PluginArgument, relaunch : bool) -> Result<PluginResult, Arc<anyhow::Error>>
  {
    let plugin = match self.plugins_db.find(plugin_name)
    {
      Some(plugin) => plugin,
      None => return Err(Arc::new(RustructError::PluginNotFound{ name : plugin_name.into()}.into())), 
    };
    let plugin = plugin.instantiate();

    self.task_scheduler.run(plugin, argument, relaunch)
  }
   
  /// Join on all scheduled task.
  /// This function is blocking the [TaskScheduler], so must be avoided in multithreaded code.
  pub fn join(&self) 
  {
    self.task_scheduler.join();
  }
}

impl Default for Session
{
  fn default() -> Self
  {
    Self::new()
  }
}

#[cfg(test)]
mod tests
{
  use super::Session;
  use crate::plugin_dummy;
  use crate::tree::AttributePath;

  use serde_json::json;

  #[test]
  fn schedule_dummy_plugin()
  {
    let mut session = Session::new();
    session.plugins_db.register(Box::new(plugin_dummy::Plugin::new()));

    let _dummy_config = session.plugins_db.config("dummy").unwrap();
    let dummy_arg = json!({"parent" : session.tree.root_id, "file_name" : "/home/user/test.txt", "offset" : 0});

    let id = session.schedule("dummy", dummy_arg.to_string(), false).unwrap();
    session.task_scheduler.join();
    session.task_scheduler.task(id).unwrap();
  }
 
  #[test]
  fn run_dummy()
  {
    let mut session = Session::new();
    session.plugins_db.register(Box::new(plugin_dummy::Plugin::new()));

    session.run("dummy", json!({"parent" : session.tree.root_id, "file_name" : "/home/user/test.txt", "offset" : 0}).to_string(), false).unwrap();
  }

  #[test] //XXX put this test in tree
  fn new_attribute_path()
  {
    let mut session = Session::new();
    session.plugins_db.register(Box::new(plugin_dummy::Plugin::new()));

    session.run("dummy", json!({"parent" : session.tree.root_id, "file_name" : "/home/user/test.txt", "offset" : 0}).to_string(), false).unwrap();

    //XXX put this test in tree
    let attribute_path = AttributePath::new(&session.tree, "/root/Dummy/DummyStatic:b").unwrap();
    assert!(attribute_path.get_node(&session.tree).unwrap().name() == "DummyStatic");
    assert!(attribute_path.get_value(&session.tree).unwrap().as_u64() == 0x1000);

    let dynamic_attribute_path = AttributePath::new(&session.tree, "/root/Dummy/DummyDynamicValue:calc_void").unwrap();
    assert!(dynamic_attribute_path.get_node(&session.tree).unwrap().name() == "DummyDynamicValue");
    assert!(dynamic_attribute_path.get_value(&session.tree).unwrap().to_string() == "ABCDEFGH1234567890");
  }
}
