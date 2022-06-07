//! A reflection trait for Rust struct, that permit to access struct member as [Attribute].
//! [ReflectStruct] can be used with tap_derive macro to automatically generate [Attribute] from Struct.

use std::fmt::Debug;
use crate::value::Value;
use crate::attribute::Attribute;
use serde::{Serialize};
use serde::ser::{Serializer, SerializeStruct};

/** 
 *  [ReflectStruct] is a trait used to wrapper a struct and give dynamic reflection information and access to the value of their a members. 
 **/
pub trait ReflectStruct : Sync + Send + Debug
{
  /// Return the name of the [ReflectStruct].
  fn name(&self) -> &'static str;//We should add a TypeId describing the structure type
  
  /// Return a tuple containing the name and description of each field of the [ReflectStruct].
  fn infos(&self) -> Vec<(&'static str, Option<&'static str>) >;

  /// Return field `name` [Value].
  fn get_value(&self, name : &str) -> Option<Value>;

  /// Return name of all the member field of the struct.
  fn names(&self) -> Vec<&'static str> 
  {
    self.infos().iter().map(|x| x.0).collect()
  }

  /// Return description of all the member field of the struct.
  fn descriptions(&self) -> Vec<Option<&'static str>>
  {
    self.infos().iter().map(|x| x.1).collect()
  }

  /// Return a Vector of Attribute containing a tuple name, value, description of the all the field of the struct.
  /// If [ReflectStruct] bind function, the function will be called to get the resulting Value.
  fn attributes(&self) -> Vec<Attribute>
  {
    let mut attributes = Vec::new();
   
    for info in self.infos()
    {
      if let Some(value) = self.get_value(info.0)
      {
         attributes.push(Attribute::new(info.0, value, info.1));
      }
    }
    attributes
  }

  /// Return the number of field in the Struct.
  fn count(&self) -> usize
  {
    self.infos().len()
  }
} 

impl Serialize for dyn ReflectStruct + Sync + Send
{
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
      where S: Serializer,
  {
      let mut state = serializer.serialize_struct(self.name(), self.count())?;

      for info in self.infos()
      {
        if let Some(value) = self.get_value(info.0)
        {
          state.serialize_field(info.0, &value)?;
        }
      }
      state.end()
  }
}
