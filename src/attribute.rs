//! [Attributes] are the base element stored in the [Tree](crate::tree::Tree) by the Plugins .
//! Each [Attribute] contain a `name`, a [Value] and a `description`, 
//! and can be generated statically or dynamically. 

use std::fmt;
use std::borrow::Cow;
use std::sync::{Arc, RwLock, RwLockReadGuard};

use crate::value::{Value, ValueTypeId};

use serde::{Serialize, Deserialize};
use serde::ser::{Serializer, SerializeMap};

/**
 * An Attribute contain a `name`, a `value` and a `description`.
 */
#[derive(Clone, Serialize, Deserialize)]
pub struct Attribute
{
  name : Cow<'static, str>,
  value : Value,
  #[serde(skip)] //We don't serialize the description by default
  description : Option<Cow<'static, str>>,
}

impl Attribute
{
  /// Create an [Attribute]from it's `name`, `value` and `description`.
  pub fn new<S>(name : S, value : Value, description : Option<S>) -> Self
    where S: Into<Cow<'static, str>>
  {
    match description
    {
      Some(description) => Attribute{name : name.into(), value, description : Some(description.into()) },
      None => Attribute{name : name.into(), value, description : None },
    }
  }

  /// Return the `name` of this [attribute](Attribute).
  pub fn name(&self) -> &str
  {
    &self.name
  }

  /// Return the `value` of this [attribute](Attribute).
  pub fn value(&self) -> &Value 
  {
    &self.value
  }

  /// Return the `value` [ValueTypeId] of this [attribute](Attribute).
  pub fn type_id(&self) -> ValueTypeId
  {
    self.value.type_id()
  }

  /// Return the `description` of this [attribute](Attribute).
  pub fn description(&self) -> Option<&str>
  {
    match &self.description
    {
       Some(description) => Some(description),
       None => None,
    }
  }
}

impl fmt::Display for Attribute
{
   fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result 
   {
      write!(f, "\"{}\" : {:?}", self.name(), self.value())
   }
}


/**
 * [Attributes] is a container for [Attribute].
 */
#[derive(Default, Clone)]
pub struct Attributes
{
  attributes : Arc<RwLock<Vec<Attribute>>>,
}

impl Attributes
{
  /// Return a new [Attributes].
  pub fn new() -> Self
  {
    Attributes{ attributes : Arc::new(RwLock::new(Vec::new())) }
  }

  /// Return the `name` of all the attribute contained in this [attributes](Attributes).
  pub fn names(&self) -> Vec<String>
  {
    self.attributes.read().unwrap().iter().map(|x| x.name().into()).collect()
  }

  /// Add a new [attribute](Attribute) by passing it's `name`, `value` and `description`.
  pub fn add_attribute<S, V : Into<Value>>(&mut self, name : S, value : V, descr : Option<S>)
    where S: Into<Cow<'static, str>>
  {
    self.attributes.write().unwrap().push(Attribute::new(name, value.into(), descr))
  }
 
  /// Remove an [attribute](Attribute) by `name`.
  pub fn remove_attribute(&mut self, name : &str) -> bool
  {
    let mut attributes = self.attributes.write().unwrap();
    if let Some(index) = attributes.iter().position(|attribute| attribute.name == name)
    {
      attributes.swap_remove(index);
      return true
    }
    false
  }

  /*pub fn replace_attribute<S, V : Into<Value>>(&mut self, name : S, value : V, descr : Option<S>)
    where S: Into<Cow<'static, str>>
  {
    self.remove_attribute(&name.into());
    self.add_attribute(name, value, descr);
  }*/

  /// Add [attributes](Attribute) by passing a Vector of tuple containing the `name`, `value` and `description` of the [attribute](Attribute).
  pub fn add_attributes<S>(&mut self, attr: Vec<(S, Value, Option<S>) >)
    where S: Into<Cow<'static, str>>
  {
    let mut attributes = self.attributes.write().unwrap();
    for (name, value, descr) in attr
    {
      attributes.push(Attribute::new(name, value, descr));
    }
  }

  /// Return the number of [attribute](Attribute) contained in this [attributes](Attributes).
  pub fn count(&self) -> usize
  {
    self.attributes.read().unwrap().len()
  }

  /// Return an [attribute](Attribute) `value`.
  pub fn get_value(&self, name : &str) -> Option<Value>
  {
    self.attributes.read().unwrap().iter().find(|x| {x.name() == name}).map(|attribute| attribute.value().clone())
  }

  /// Return an [attribute](Attribute).
  pub fn get_attribute(&self, name : &str) -> Option<Attribute>
  {
    self.attributes.read().unwrap().iter().find(|x| {x.name() == name}).cloned()
  }

  /// Return an [attribute](Attribute) [value](Value) [type_id](ValueTypeId).
  pub fn get_type_id(&self, name : &str) -> Option<ValueTypeId>
  {
    self.attributes.read().unwrap().iter().find(|x| {x.name() == name}).map(|attribute| attribute.value().type_id())
  }

  /*/// Return true if an attribute with this name exists in the container
  //handle "." attribute for attribute container inside attribute ? 
  pub fn has_attribute(&self, name : &str) -> bool
  {
    //iter manually rather than using copy ?
     self.attributes().iter().any(|attr| {attr.name() == name})
  }*/


  /// Return an iterator to the contained [Attributes](Attribute).
  pub fn attributes(&self) -> LockedAttributes<'_>
  {
    LockedAttributes{items :self.attributes.read().unwrap() }
  }
}

pub struct LockedAttributes<'a>
{
   items :  RwLockReadGuard<'a, std::vec::Vec<Attribute>>
}

impl<'a> LockedAttributes<'a> 
{
    pub fn iter(&self) -> impl Iterator<Item = &Attribute> 
    {
        self.items.iter()
    }
}

impl Serialize for Attributes 
{
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
      where S: Serializer,
  {
     let attributes = self.attributes.read().unwrap();
     let count = attributes.len();   

     let mut map = serializer.serialize_map(Some(count))?;

     for attribute in attributes.iter()
     {
        map.serialize_entry(&attribute.name(), &attribute.value())?;
     }
     
     map.end()
  }
}

impl fmt::Debug for Attributes 
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result 
  {
    let attributes = self.attributes.read().unwrap();
    write!(f, "{{").unwrap();
    for attribute in attributes.iter()
    {
      write!(f, "{}, ", attribute).unwrap();
    }
    write!(f, "}}").unwrap();
    Ok(())
  }
}

impl std::cmp::PartialEq for Attributes
{
  fn eq(&self, other: &Self) -> bool
  {
    if self.count() != other.count()
    {
      return false;
    }

    for attribute in self.attributes.read().unwrap().iter()
    {
      match other.get_value(attribute.name())
      {
        Some(other_value) =>  if *attribute.value() != other_value { return false; },
        None => return false,
      };
    }
    true
  }
}

#[cfg(test)]
mod tests
{
    use super::{Attribute, Attributes};
    use crate::value::{Value, ValueTypeId};

    #[test]
    fn create_attribute()
    {
      let attribute = Attribute::new("attribute", Value::U32(0x1000), Some("test attribute"));
      assert!(attribute.name() == "attribute");
      assert!(attribute.value().as_u32() == 0x1000); 
      assert!(attribute.description() == Some("test attribute"));
      assert!(attribute.type_id() as u32 == ValueTypeId::U32 as u32);
      assert!(format!("{}", attribute) == "\"attribute\" : 4096");
    }

    #[test]
    fn create_attributes()
    {
      let mut attributes = Attributes::new();
      attributes.add_attribute("attribute", Value::U32(0x1000), Some("test attribute"));
      attributes.add_attributes(vec![("attribute2", Value::String(String::from("something")), Some("Intersting string")),
                          ("attribute3", Value::Seq(vec![Value::U32(0), Value::from(String::from("test"))]), None)]);
      assert!(attributes.count() == 3);
      let attribute = attributes.get_attribute("attribute").unwrap();
      assert!(attribute.name() == "attribute");
      assert!(attribute.value().as_u32() == 0x1000); 
      assert!(attribute.description() == Some("test attribute"));
      assert!(attribute.type_id() as u32 == ValueTypeId::U32 as u32);
      assert!(format!("{}", attribute) == "\"attribute\" : 4096");
      assert!(attributes.get_value("attribute2").unwrap().as_string() == "something");
      let vec = attributes.get_value("attribute3").unwrap().as_vec();
      assert!(vec.len() == 2);
      assert!(vec[0].as_u32() == 0);
      assert!(vec[1].as_string() == "test");
    }
}
