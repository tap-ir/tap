//! Node is used as a tree item that let you access the static and dynamic attributes added by the plugins.
use std::fmt;
use std::borrow::Cow;

use crate::value::{Value};
use crate::attribute::{Attribute, Attributes};

use serde::ser::{Serialize, Serializer};

/// [Node] is used as a [tree](crate::tree::Tree) item. It's an abstraction layer above an Attribute.
pub struct Node
{
  attribute : Attribute,
}

impl Node 
{
  /// Return a [Node].
  pub fn new<S>(name : S) -> Self 
    where S: Into<Cow<'static, str>>
  {
    Node{ attribute : Attribute::new(name.into(), Value::Attributes(Attributes::new()), None) }
  }

  /// Return the underlying [attribute](Attribute).
  pub fn attribute(&self) -> &Attribute
  {
    &self.attribute
  }

  /// Return the [attribute](Attribute) value.
  pub fn value(&self) -> Attributes
  {
    self.attribute.value().as_attributes()
  }

  /// Return the [Node] name
  pub fn name(&self) -> String 
  {
    self.attribute.name().to_string()
  }
}

impl Serialize for Node 
{
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer,
  {
     //serialize name ?
     let attribute = self.attribute.value().as_attributes();
     attribute.serialize(serializer)
  }
}

impl fmt::Display for Node 
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result 
  {
    write!(f, "{}", self.attribute)
  }
}

impl fmt::Debug for Node 
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result 
  {
    write!(f, "{}", self.attribute)
  }
}

#[cfg(test)]
mod tests
{
    use std::sync::{Arc};

    use super::Node;
    use crate::value::{Value, ValueTypeId};
    use crate::reflect::ReflectStruct;

    #[test]
    fn create_node()
    {
      let node = Node::new("test");
      assert!(node.name() == "test");
    }

    #[test]
    fn create_node_with_static_attributes()
    {
      let node = Node::new("test");
      node.value().add_attribute("attribute", Value::U32(0x1000), Some("test attribute"));
      node.value().add_attributes(vec![("attribute2", Value::from(String::from("something")), Some("Intersting string")),
                               ("attribute3", Value::Seq(vec![Value::U32(0), Value::from(String::from("test"))]), None)]);
      assert!(node.value().count() == 3);
      let attributes = node.value();
      let attribute = attributes.get_attribute("attribute").unwrap();
      assert!(attribute.name() == "attribute");
      assert!(node.value().get_value("attribute").unwrap().as_u32() == 0x1000); 
      assert!(attribute.description() == Some("test attribute"));
      assert!(node.value().get_type_id("attribute").unwrap() as u32 == ValueTypeId::U32 as u32);
      assert!(node.value().get_value("attribute2").unwrap().as_string() == "something");
      let vec = node.value().get_value("attribute3").unwrap().as_vec();
      assert!(vec.len() == 2);
      assert!(vec[0].as_u32() == 0);
      assert!(vec[1].as_string() == "test");
    }

    #[test]
    fn create_node_with_dynamic_attributes()
    {
       #[derive(Debug)]
       struct Test
       {
         string1 : &'static str,
         string2 : &'static str,
       }

       impl Test
       {
         fn calc(&self) -> u32
         {
           self.string1.len() as u32 + self.string2.len() as u32
         }

         fn new_node(self, name : &'static str) -> Node
         {
            let node = Node::new(name);
            //node.value().add_struct(Arc::new(self));
            node.value().add_attribute("Test", Arc::new(self), None);
            node
         }
       }

       impl ReflectStruct for Test
       {
         fn name(&self) -> &'static str
         {
           "Test"
         }

         fn infos(&self) -> Vec<(&'static str, Option<&'static str>) >
         {
            vec![("string1", None), ("string2", None), ("calc", None)]
         }

         fn get_value(&self, name : &str) -> Option<Value>
         {
            match name
            {
                "string1" => Some(Value::from(self.string1)),
                "string2" => Some(Value::from(self.string2)),
                "calc" => Some(Value::U32(self.calc())),
                _ => None,
            }
         }
       }

       let test = Test{string1 : "first", string2 : "second"}.new_node("Test");
       assert!(test.value().get_value("Test").unwrap().as_reflect_struct().get_value("string1").unwrap().as_string() == "first");
       assert!(test.value().get_value("Test").unwrap().as_reflect_struct().get_value("string2").unwrap().as_string() == "second");
       assert!(test.value().get_value("Test").unwrap().as_reflect_struct().get_value("calc").unwrap().as_u32() == 11);
    }
} 
