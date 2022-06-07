//! [PluginsDB] is the database containing all the registred plugins 
//! it provides you with helper function to manipulate plugins. 

use crate::plugin::{PluginInfo, PluginInstance, PluginConfig};
use crate::error::RustructError;
use anyhow::Result;

#[derive(Default)]
pub struct PluginsDB
{
  plugins_info : Vec<Box<dyn PluginInfo + Sync + Send> >,
}

/// A database containing all the registred plugins
/// it provides you with helper function to manipulate plugins.
impl PluginsDB
{
  /// Return a new [PluginsDB].
  pub fn new() -> PluginsDB
  {
    Default::default()
  }

  /// Return the number of Plugins in the DB.
  pub fn len(&self) -> usize
  {
    self.plugins_info.len()
  }

  /// Return if DB is empty.
  pub fn is_empty(&self) -> bool
  {
    self.plugins_info.is_empty()
  }

  /// Return an iterator to the Plugins list.
  pub fn iter(&self) ->  std::slice::Iter< Box<dyn PluginInfo + Sync + Send> >
  {
    self.plugins_info.iter()
  }

  /// Return a Plugin that match `name`.
  #[allow(clippy::borrowed_box)]
  pub fn find(&self, name : &str) -> Option<&Box<dyn PluginInfo + Sync + Send> >
  {
    self.plugins_info.iter().find(|x| {
      x.name() == name
    })
  }

  /// Return the configuration that you should pass to a Plugin run method.
  pub fn config(&self, name : &str) -> Result<PluginConfig>
  {
    match self.plugins_info.iter().find(|x| {x.name() == name})
    {
      Some(plugin_info) => Ok(plugin_info.config()?),
      None =>  Err(RustructError::PluginNotFound{ name : name.to_string() }.into()),
    }
  }

  /// Instantiate a new Plugin. 
  pub fn instantiate(&self, name : &'static str) -> Option< Box< dyn PluginInstance+ Send + Sync> >
  {
    self.find(name).map(|plugin| plugin.instantiate())
  }

  /// Register a new Plugin.
  pub fn register(&mut self, plugin_info: Box< dyn PluginInfo + Sync + Send >) -> bool 
  {
    //try to find if a plugins with the same name is already registred 
    match self.find(plugin_info.name())
    { 
      Some(_) => false,
      None => { self.plugins_info.push(plugin_info); true }
    }
  }

  /// Unregister a Plugin.
  pub fn unregister(&mut self, name : &'static str) -> bool
  {
    match self.find(name)
    {
      Some(_) => { self.plugins_info.retain(|info| info.name() != name); true}
      None => false
    }
  }
}

#[cfg(test)]
mod tests 
{
    use super::PluginsDB;
    use crate::plugin::PluginEnvironment;
    use crate::plugin_dummy;
    use crate::tree::Tree;

    //test db len ?
    #[test]
    fn plugins_db_test_register()
    {
        let mut plugins_db = PluginsDB{ plugins_info : Vec::new() };
        assert!(plugins_db.register(Box::new(plugin_dummy::Plugin::new())));
    }

    #[test]
    fn plugins_db_test_register_twice()
    {
        let mut plugins_db = PluginsDB{ plugins_info : Vec::new() };

        assert!(plugins_db.register(Box::new(plugin_dummy::Plugin::new())));
        /*plugin already registred must return false */
        assert!(!plugins_db.register(Box::new(plugin_dummy::Plugin::new())));    
    }

    #[test]
    fn plugins_db_test_unregister()
    {
        let mut plugins_db = PluginsDB::new();

        plugins_db.register(Box::new(plugin_dummy::Plugin::new()));
        assert!(plugins_db.unregister("dummy"));
    }

    #[test]
    fn plugins_db_test_unregister_twice()
    {
        let mut plugins_db = PluginsDB::new();
       
        plugins_db.register(Box::new(plugin_dummy::Plugin::new()));
        assert!(plugins_db.unregister("dummy"));
        assert!(!plugins_db.unregister("dummy"));
    }

    #[test]
    fn plugins_db_iter()
    {
        let mut plugins_db = PluginsDB::new();
        let tree = Tree::new(); 

        plugins_db.register(Box::new(plugin_dummy::Plugin::new()));

        for plugin in plugins_db.iter()
        {
            let config = plugin.config().unwrap();
            let mut plugin = plugin.instantiate();
            let _res = plugin.run(config, PluginEnvironment::new(tree.clone(), None));
        }
    }

    #[test]
    fn plugins_find()
    {
        let mut plugins_db = PluginsDB::new();
       
        plugins_db.register(Box::new(plugin_dummy::Plugin::new()));
        assert!(plugins_db.find("dummy").is_some())
    }

    #[test]
    fn plugins_db_instantiate()
    {
        let mut plugins_db = PluginsDB::new();
       
        plugins_db.register(Box::new(plugin_dummy::Plugin::new()));
        assert!(plugins_db.instantiate("dummy").is_some())
    }

    #[test]
    fn plugins_db_test_instance_name_equality()
    {
        let mut plugins_db = PluginsDB::new();
       
        plugins_db.register(Box::new(plugin_dummy::Plugin::new()));
        for plugin_info in plugins_db.iter()
        {
            let instance = plugin_info.instantiate();
            assert_eq!(plugin_info.name(), instance.name())
        }
    }
}
