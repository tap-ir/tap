//! The `dummy singleton plugin` is an exemple of how to write a singleton/static plugin.
//! This plugin instantiate method will always return the same object.

use crate::config_schema;
use crate::plugin::{PluginInfo, PluginInstance, PluginConfig, PluginArgument, PluginResult, PluginEnvironment};

use anyhow::Result;
use owned_singleton::Singleton;
use serde::{Serialize, Deserialize};
use schemars::{JsonSchema};
use log::info;

#[Singleton(Send,Sync)]
static mut OwnedDummySingleton : DummySingleton = DummySingleton{ count : 0  };

#[derive(Default)]
pub struct DummySingletonInfo
{
}

impl DummySingletonInfo
{
    pub fn new() -> DummySingletonInfo
    {
        DummySingletonInfo{}
    }
}

impl PluginInfo for DummySingletonInfo
{
    fn name(&self) -> &'static str
    {
        "dummy_singleton"
    }

    fn category(&self) -> &'static str
    {
        "Test"
    }

    fn help(&self) -> &'static str
    {
        "A singleton dummy module for testing purpose"
    }

    fn config(&self) -> Result<PluginConfig>
    {
        let schema = config_schema!(Arguments);
        Ok(serde_json::to_string(&schema)?)
    }

    fn instantiate(&self) -> Box<dyn PluginInstance + Send + Sync>
    {
        unsafe 
        {
          Box::new(OwnedDummySingleton::new())
        }
    }
}

#[derive(Default)]
pub struct DummySingleton
{
    count : u32,
}

impl PluginInstance for OwnedDummySingleton
{
    fn name(&self) -> &'static str
    {
        "dummy_singleton"
    }

    fn run(&mut self, arg_str : PluginArgument, env : PluginEnvironment) -> Result< PluginResult >
    {
        let arg = serde_json::from_str(&arg_str)?;
        let result = self.run(arg, env)?;
        Ok(serde_json::to_string(&result)?)
    }
}

#[derive(Debug, Serialize, Deserialize,Default, JsonSchema)]
pub struct Arguments
{
    file_name : String,
    offset : u32,
}

#[derive(Debug, Serialize,Deserialize,Default)]
pub struct Results
{
    count : u32
}

impl OwnedDummySingleton
{
    fn run(&mut self, argument : Arguments, _env : PluginEnvironment) -> Result< Results>
    {
        info!("\tdummy_singleton run({:?})", argument);

        info!("\tdummy_singleton parser is running on file : {:?}", argument.file_name);
        self.count += 1;
        info!("\tdummy_singleton counter : {}", self.count);
        info!("\tdummy_singleton finished");

        Ok(Results{count : self.count})
    }
}


#[cfg(test)]
mod tests
{
    use serde_json::Value;
    use serde_json::json;
    use crate::plugin::{PluginInfo, PluginEnvironment};
    use crate::plugin_dummy_singleton::DummySingletonInfo;
    use crate::tree::Tree;

    #[test]
    fn dummy_plugin_singleton_test_instances()
    {
       let tree = Tree::new();
       let dummy_singleton_info = DummySingletonInfo::new();
       let mut dummy_singleton = dummy_singleton_info.instantiate();
       //let args = dummy_singleton_info.config().unwrap();

       let args = json!({"file_name" : "test", "offset" : 0}).to_string();
       match dummy_singleton.run(args.to_string(), PluginEnvironment::new(tree.clone(), None))
       {
         Ok(res) => {
                      let res : Value = serde_json::from_str(&res).unwrap();
                      match res["count"].as_u64().unwrap()
                      {
                       1 => assert!(true),
                        _ => assert!(false),
                      }
                   }
         Err(_err) => assert!(false),
       }

       match dummy_singleton.run(args.to_string(), PluginEnvironment::new(tree.clone(), None))
       {
         Ok(res) => {
                      let res : Value = serde_json::from_str(&res).unwrap();
                      match res["count"].as_u64().unwrap()
                      {
                       2 => assert!(true),
                        _ => assert!(false),
                      }
                   }
         Err(_err) => assert!(false),
       }

       let mut dummy_singleton_new = dummy_singleton_info.instantiate();
       match dummy_singleton_new.run(args.to_string(), PluginEnvironment::new(tree, None))
       {
         Ok(res) => {
                      let res : Value = serde_json::from_str(&res).unwrap();
                      match res["count"].as_u64().unwrap()
                      {
                        3 => assert!(true),
                        _ => assert!(false),
                      }
                   }
         Err(err) => { eprintln!("{}", err); assert!(false) },
       }
    }
}
