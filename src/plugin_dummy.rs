//! The `dummy plugin` is an exemple of how to write a plugin.

use std::sync::Arc;

use crate::config_schema;
use crate::plugin::{PluginInfo, PluginInstance, PluginConfig, PluginArgument, PluginResult, PluginEnvironment};
use crate::reflect::ReflectStruct;
use crate::node::Node;
use crate::tree::{TreeNodeId, TreeNodeIdSchema};
use crate::value::Value;
use crate::tree::Tree;
use crate::error::{RustructError};

use serde::{Serialize, Deserialize};
use schemars::{JsonSchema};
use log::info;
use anyhow::Result;

use crate::plugin;

plugin!("dummy", "Test",  "A dummy module for testing purpose", Dummy, Arguments);

/// The dummy plugin
#[derive(Default)]
pub struct Dummy
{
    count : u32,
}


/// The argument struct that will be passed to the  run method of the plugin.
#[derive(Debug, Serialize, Deserialize,Default, JsonSchema)]
pub struct Arguments
{
    file_name : String,
    offset : u32,
    #[schemars(with = "TreeNodeIdSchema")]
    parent : Option<TreeNodeId>,
}

/// The results class that will be returned from the plugin.
#[derive(Debug, Serialize, Deserialize,Default)]
pub struct Results
{
    count : u32
}

#[derive(Debug)]
struct DummyStatic
{
  a : u8,
  b : u64,
  c : String,
}

impl DummyStatic 
{
  pub fn new(a : u8, b : u64, c : String) -> Self
  {
    DummyStatic{ a, b, c }
  }

  fn new_node(&self) -> Node
  {
    let node  = Node::new("DummyStatic");

    node.value().add_attributes(
        vec![("a", Value::from(self.a), None),
             ("b", Value::from(self.b), None),
             ("c", Value::from(self.c.clone()), None),]);
    node
  }
}

#[derive(Debug)]
struct DummyDynamic 
{
  a : u32,
  b : u64,
}

impl DummyDynamic 
{
  pub fn new() -> Self
  {
    DummyDynamic{a : 1, b : 2}
  }

  pub fn field_c(&self) -> u64
  {
    self.a as u64 + self.b
  }
}

impl ReflectStruct for DummyDynamic 
{
  fn name(&self) -> &'static str
  {
    "DummyDynamic"
  }

  fn infos(&self) -> Vec<(&'static str, Option<&'static str>) >
  {
    vec![("a", None), ("b", None), ("c", None)]
  }

  fn get_value(&self, name : &str) -> Option<Value>
  {
    match name
    {
      "a" => Some(Value::from(self.a)),
      "b" => Some(Value::from(self.b)),
      "c" => Some(Value::from(self.field_c())),
      _ => None,
    }
  }
}

pub struct DummyDynamicValue
{
}

impl DummyDynamicValue
{
  pub fn calc_with_value(&self, value : Value) -> Value
  {
    Value::from(value.to_string())
  }

  pub fn calc_void(&self) -> Value
  {
    Value::from("ABCDEFGH1234567890")
  }

  pub fn set_to_node(node : Node) -> Node 
  {
    let node = node;
    let handler = Self{};

    let func = Box::new(move || handler.calc_void());
    node.value().add_attribute("calc_void", Value::Func(Arc::new(func)), None);

    let handler =  Self{};
    let func_arg = Box::new(move |x| handler.calc_with_value(x));
    node.value().add_attribute("calc_with_value", Value::FuncArg(Arc::new(func_arg), Box::new(Value::from("ABCDEFGH1234567890"))), None);
    node
  }
}

impl Dummy
{
    fn create_nodes(&self, parent_id : TreeNodeId, tree : Tree) -> Result<()>
    {
      let dummy_node = Node::new("Dummy");
      dummy_node.value().add_attribute("offset", Value::U64(0x1000), None);
      let dummy_node_id = match tree.add_child(parent_id, dummy_node)
      {
        Ok(dummy_node_id) => dummy_node_id,
        //Err(_) => return Err(RustructError::Unknown("Node Dummy already exists, module is already launched.".to_string()).into())
        Err(err) => return Err(err) 
      };

      let dummy_static = DummyStatic::new(255, 0x1000, "dummy".to_string()).new_node();
      tree.add_child(dummy_node_id, dummy_static).unwrap(); 

      let dummy_dynamic = DummyDynamic::new(); 
      let dummy_dynamic_node = Node::new("DummyDynamic");
      dummy_dynamic_node.value().add_attribute("dummy_dynamic", Arc::new(dummy_dynamic), None);
      tree.add_child(dummy_node_id, dummy_dynamic_node).unwrap();

      let dummy_dynamic_value = DummyDynamicValue::set_to_node(Node::new("DummyDynamicValue"));
      tree.add_child(dummy_node_id, dummy_dynamic_value).unwrap();

      Ok(())
    }

    fn run(&mut self, argument : Arguments, env : PluginEnvironment) -> Result< Results>
    {
        info!("\tdummy run({:?})", argument);

        info!("\tdummy parser is running on file : {:?}", argument.file_name);
        self.count += 1;
        info!("\tdummy counter : {}", self.count);

        info!("\tdummy is creating node :");
        let parent = match argument.parent
        {
            Some(parent) => parent,
            None => return Err(RustructError::ArgumentNotFound("parent").into()),
        };
        self.create_nodes(parent, env.tree)?;
        info!("\tdummy finished");

        Ok(Results{count : self.count})
    }
}

#[cfg(test)]
mod tests
{
    use crate::plugin::{PluginInfo, PluginEnvironment};
    use crate::plugin_dummy::Plugin;
    use crate::tree::Tree;
    
    use serde_json::Value;
    use serde_json::json;

    #[test]
    fn dummy_plugin_test_run()
    {
      let tree = Tree::new();
      let dummy_info = Plugin::new();
      let mut dummy = dummy_info.instantiate();

      let args = json!({"parent" :  tree.root_id, "file_name" : "/home/user/test.txt", "offset" : 0}).to_string();

      
      match dummy.run(args.to_string(), PluginEnvironment::new(tree, None))
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
    }

    #[test]
    fn dummy_plugin_arg_json_value()
    {
      let tree = Tree::new();
      let dummy_info = Plugin::new();
      let config = dummy_info.config().unwrap();
      let mut dummy = dummy_info.instantiate();

      let mut args : Value = serde_json::from_str(&config).unwrap();
      args["file_name"] = Value::String("/home/user/file".to_string());
      args["parent"] = json!(tree.root_id);
      args["offset"] = Value::Number(serde_json::Number::from(0));

      match dummy.run(args.to_string(), PluginEnvironment::new(tree, None))
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
    }

    //we forbid launchign instances on the same mount point, as node with same name will be created
    //if we want to test multiple instance we must create nodes on multiple mount point/parent
    #[test]
    fn dummy_plugin_test_instances()
    {
       let tree = Tree::new();
       let dummy_info = Plugin::new();
       let mut dummy = dummy_info.instantiate();

       let args = json!({"parent" :  tree.root_id, "file_name" : "/home/user/test.txt", "offset" : 0}).to_string();
       match dummy.run(args.to_string(), PluginEnvironment::new(tree.clone(), None))
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

       match dummy.run(args.to_string(), PluginEnvironment::new(tree.clone(), None))
       {
         Ok(res) => {
                      let res : Value = serde_json::from_str(&res).unwrap();
                      match res["count"].as_u64().unwrap()
                      {
                       2 => assert!(true),
                        _ => assert!(false),
                      }
                   }
         Err(_err) => assert!(true), //return error becasuse we use same mount point, and node with same name will be created at same mountpoint returning error
       }

       let mut dummy_new = dummy_info.instantiate();
       let args = json!({"parent" :  tree.root_id, "file_name" : "/home/user/test.txt", "offset" : 0}).to_string();

       match dummy_new.run(args.to_string(), PluginEnvironment::new(tree, None))
       {
         Ok(res) => {
                      let res : Value = serde_json::from_str(&res).unwrap();
                      match res["count"].as_u64().unwrap()
                      {
                        1 => assert!(true),
                        _ => assert!(false),
                      }
                   }
         Err(_err) => assert!(true),
       }
    }

    #[test]
    fn dummy_plugin_test_tree_value()
    {
      let tree = Tree::new();
      let dummy_info = Plugin::new();
      
      let mut dummy = dummy_info.instantiate();
      let args = json!({"parent" :  tree.root_id, "file_name" : "/home/user/test.txt", "offset" : 0}).to_string();

      dummy.run(args.to_string(), PluginEnvironment::new(tree.clone(), None)).unwrap();
     
      let dummy_node = tree.get_node("/root/Dummy").unwrap();
      assert!(dummy_node.value().get_value("offset").unwrap().as_u64() == 0x1000);

      let dummy_static_node = tree.get_node("/root/Dummy/DummyStatic").unwrap();
      let dummy_static_node_attributes = dummy_static_node.value();
      assert!(dummy_static_node_attributes.get_value("a").unwrap().as_u8() == 255);
      assert!(dummy_static_node_attributes.get_value("b").unwrap().as_u64() == 0x1000);
      assert!(dummy_static_node_attributes.get_value("c").unwrap().as_string() == "dummy");

      let dummy_dynamic_node = tree.get_node("/root/Dummy/DummyDynamic").unwrap();
      let dummy_dynamic_node_attributes = dummy_dynamic_node.value();
      assert!(dummy_dynamic_node_attributes.get_value("dummy_dynamic").unwrap().as_reflect_struct().get_value("a").unwrap().as_u32() == 1);
      assert!(dummy_dynamic_node_attributes.get_value("dummy_dynamic").unwrap().as_reflect_struct().get_value("b").unwrap().as_u64() == 2);
      assert!(dummy_dynamic_node_attributes.get_value("dummy_dynamic").unwrap().as_reflect_struct().get_value("c").unwrap().as_u64() == 3);

      let dummy_dynamic_value_node = tree.get_node("/root/Dummy/DummyDynamicValue").unwrap();
      let dummy_dynamic_value_node_attributes = dummy_dynamic_value_node.value();

      assert!(dummy_dynamic_value_node_attributes.get_value("calc_void").unwrap().to_string() == "ABCDEFGH1234567890");
      assert!(dummy_dynamic_value_node_attributes.get_value("calc_with_value").unwrap().to_string() == "ABCDEFGH1234567890");
    }
}
