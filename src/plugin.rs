//! This module contain the different trait that Plugin must implement.

use crate::tree::Tree;
use crate::task_scheduler::TaskState;
use crossbeam::crossbeam_channel::{Sender};

/// JSON String containing [Plugin](PluginInfo) configuration
pub type PluginConfig = String;
/// JSON String containing [PluginInstance] argument
pub type PluginArgument = String;
/// JSON String containg [PluginInstance] result
pub type PluginResult = String;

/**
 * Contain structure needed by Plugin to interact with the core 
 */
pub struct PluginEnvironment
{
  pub tree: Tree,
  pub channel : Option<Sender<TaskState>>,   
}

impl PluginEnvironment
{
  pub fn new(tree : Tree, channel : Option<Sender<TaskState>>) -> Self
  {
    PluginEnvironment{ tree, channel }
  }
}

/**
 * This trait must be implemented by all Plugin.
 * The [PluginInfo] trait give differents informations about a Plugin and permit to create a new instance of a Plugin via the instantiate method.
 */
pub trait PluginInfo
{
  /// Return the `name` of the Plugin
  fn name(&self) -> &'static str;
  /// Return a `category` for the Plugin 
  fn category(&self) -> &'static str;
  /// Create and return a new instance of the Plugin
  fn instantiate(&self) -> Box<dyn PluginInstance + Send + Sync>;
  /// Return a `description` of what the plugin do
  fn help(&self) -> &'static str;
  ///Return a JSON [String] with structure taken as argument
  fn config(&self) -> anyhow::Result<PluginConfig>; 
}

/** 
 * This trait must be implemented by all Plugin.
 * The run function will be called from a [TaskScheduler](crate::task_scheduler::TaskScheduler) [Worker](crate::task_scheduler::Worker) with [`argument`](PluginArgument) and [`env`](PluginEnvironment), when a Plugin is executed.
 */
pub trait PluginInstance
{
  /// Return the name of the plugin.
  fn name(&self) -> &'static str;
  /// Run the plugin and pass it JSON `argument` [String].
  /// Return the result as a JSON `String` or an Error.
  fn run(&mut self, argument : PluginArgument, env : PluginEnvironment) -> anyhow::Result<PluginResult>;
}

#[macro_export]
macro_rules! config_schema
{
    ( $type:ty ) => 
    {
      schemars::gen::SchemaSettings::default().with(|s| {
            s.option_nullable = true;
            s.option_add_null_type = false;
      }).into_generator().into_root_schema_for::<$type>()
    }
}

/// Macro to help creation of plugin. 
#[macro_export]
macro_rules! plugin 
{
    ( $name:expr, $category:expr, $help:expr, $plugin_type:ty , $plugin_argument:ty) => 
    {
        #[derive(Default)]
        pub struct Plugin
        {
        }

        impl Plugin
        {
          pub fn new() -> Plugin
          {
             Plugin{}
          }
        }

        impl PluginInfo for Plugin
        {
            fn name(&self) -> &'static str
            {
              $name 
            }

            fn category(&self) -> &'static str
            {
              $category
            }

            fn instantiate(&self) -> Box<dyn PluginInstance + Send + Sync>
            {
              let plugin : $plugin_type = Default::default();
              Box::new(plugin)
            }

            fn help(&self) -> &'static str
            {
              $help 
            }

            fn config(&self) -> anyhow::Result<PluginConfig>
            {
                let schema = config_schema!($plugin_argument);
                Ok(serde_json::to_string(&schema)?)
            }
        }

        impl PluginInstance for $plugin_type
        {
            fn name(&self) -> &'static str
            {
              $name 
            }

            fn run(&mut self, arg_str : PluginArgument, env : PluginEnvironment) -> anyhow::Result< PluginResult >
            {
                 let arg = serde_json::from_str(&arg_str)?;
                 let result = self.run(arg, env)?;
                 Ok(serde_json::to_string(&result)?)
            }
        }
    }    
}
