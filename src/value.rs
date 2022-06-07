//! Value is a variant type container used to store different kind of data inside an `Attribute`.

use std::fmt;
use std::cmp::Ordering;
use std::sync::{Arc};
use std::collections::HashMap;

use crate::vfile::{VFileBuilder};
use crate::tree::{TreeNodeId, AttributePath};
use crate::attribute::Attributes;
use crate::reflect::ReflectStruct;

use serde::{Serialize, Deserialize};
use serde::ser::{Serializer};
use chrono::{DateTime, Utc};
use std::borrow::Cow;

type ValueFunc = Arc<Box<dyn Fn() -> Value + Sync + Send>>;
type ValueFuncArg = Arc<Box<dyn Fn(Value) -> Value + Sync + Send>>;

/**
 *  [Value] is a clonable and serializable variant kind use as value of [Attribute](crate::attribute::Attribute).
 */
#[derive(Deserialize,Serialize, Clone)]
#[serde(untagged)]
pub enum Value 
{
    #[serde(skip_deserializing)] 
    Attributes(Attributes),
    #[serde(skip_deserializing)]
    ReflectStruct(Arc<dyn ReflectStruct+ Sync + Send>),
    VFileBuilder(Arc< dyn VFileBuilder>),
    Bool(bool),

    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),

    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),

    F32(f32),
    F64(f64),
  
    USize(usize),

    Char(char),
    String(String),
    Str(Cow<'static, str>),

    Unit,
    Option(Option<Box<Value>>),
    Newtype(Box<Value>),
    Seq(Vec<Value>),
    Bytes(Vec<u8>),
    DateTime(DateTime<Utc>),

    Map(HashMap<String, Value>),
    #[serde(skip_deserializing, serialize_with="serialize_func")] 
    Func(ValueFunc),
    #[serde(skip_deserializing, serialize_with="serialize_value_func")] 
    FuncArg(ValueFuncArg, Box<Value>),

    NodeId(TreeNodeId),
    AttributePath(AttributePath),
    //Enum(ReflectEnum),//Enum(ReflectStruct)
    //None,
}

fn serialize_func<S>(func : &ValueFunc, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
{
   func().serialize(serializer)
}

fn serialize_value_func<S>(func : &ValueFuncArg, arg : &Value, serializer : S) -> Result<S::Ok, S::Error>
  where 
    S: Serializer,
{
   func(Value::Newtype(Box::new(arg.clone()))).serialize(serializer)
}


impl std::cmp::PartialEq for Value
{
  fn eq(&self, other : &Self) -> bool
  {
     self == other 
  }
}

impl std::cmp::PartialOrd for Value
{
  fn partial_cmp(&self, other : &Self) -> Option<Ordering>
  {
     if self == other
     {
       return Some(Ordering::Equal)
     }

     if self > other
     {
      return Some(Ordering::Greater)
     }

     if self < other
     {
       return Some(Ordering::Less)
     }

     None
  }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[repr(u8)]
pub enum ValueTypeId
{
    Attributes = 0,
    ReflectStruct,
    VFileBuilder,
    Bool,
    U8,
    U16,
    U32, 
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    USize,
    Char, 
    String,
    Str,
    Unit,
    Option,
    Newtype,
    Seq, 
    Bytes,
    DateTime,
    Map, 
    Func, 
    FuncArg, 
    NodeId,
    AttributePath,
    //None,
}

impl Value
{
  #[inline]
  pub fn type_id(&self) -> ValueTypeId
  {
    match self
    {
      Value::Attributes(_) => ValueTypeId::Attributes,
      Value::ReflectStruct(_) => ValueTypeId::ReflectStruct,
      Value::VFileBuilder(_) => ValueTypeId::VFileBuilder,
      Value::Bool(_) => ValueTypeId::Bool,
      Value::U8(_) => ValueTypeId::U8,
      Value::U16(_) => ValueTypeId::U16,
      Value::U32(_) => ValueTypeId::U32, 
      Value::U64(_) => ValueTypeId::U64,
      Value::I8(_) => ValueTypeId::I8,
      Value::I16(_) => ValueTypeId::I16,
      Value::I32(_) => ValueTypeId::I32,
      Value::I64(_) => ValueTypeId::I64,
      Value::F32(_) => ValueTypeId::F32,
      Value::F64(_) => ValueTypeId::F64,
      Value::USize(_) => ValueTypeId::USize,
      Value::Char(_) => ValueTypeId::Char, 
      Value::String(_) => ValueTypeId::String, 
      Value::Str(_) => ValueTypeId::Str, 
      Value::Unit => ValueTypeId::Unit,
      Value::Option(_) => ValueTypeId::Option,
      Value::Newtype(_) => ValueTypeId::Newtype,
      Value::Seq(_) => ValueTypeId::Seq, 
      Value::Bytes(_) => ValueTypeId::Bytes,
      Value::DateTime(_) => ValueTypeId::DateTime,
      Value::Map(_) => ValueTypeId::Map, 
      Value::Func(_) => ValueTypeId::Func, 
      Value::FuncArg(_, _) => ValueTypeId::FuncArg, 
      Value::NodeId(_) => ValueTypeId::NodeId,
      Value::AttributePath(_) => ValueTypeId::AttributePath,
      //Value::None => ValueTypeId::None,
    }
  }
}

macro_rules! from_primitive 
{
  ( $it:expr, $t:ty ) => 
  {
    impl From<$t> for Value 
    {
      #[inline]
      fn from(input: $t) -> Self 
      {
        $it(input)
      }
    }
  };
}

macro_rules! as_primitive 
{
  ( $it:expr, $t:ty ) => 
  {
     impl Value 
     {
       paste::item !
       {
         #[inline]
         pub fn [<as_ $t>](&self) -> $t
         {
           match self
           {
             $it(val) => *val,
             _ => panic!("Can't convert to {}", stringify!($t)), 
           }
         }
       }
     }
  };
}

macro_rules! try_as_primitive
{
  ( $it:expr, $t:ty ) => 
  {
     impl Value 
     {
       paste::item !
       {
         #[inline]
         pub fn [<try_as_ $t>](&self) -> Option<$t>
         {
           match self
           {
             $it(val) => Some(*val),
             _ => None,
           }
         }
       }
     }
  };
}

macro_rules! as_from_primitive
{
  ( $it:expr, $t:ty ) => 
  {
    as_primitive!($it, $t);
    try_as_primitive!($it, $t);
    from_primitive!($it, $t);
  };
}

/*from_primitive!(Value::None, None);*/
as_from_primitive!(Value::Bool, bool);
as_from_primitive!(Value::U8, u8);
as_from_primitive!(Value::U16, u16);
as_from_primitive!(Value::U32, u32);
as_from_primitive!(Value::U64, u64);
as_from_primitive!(Value::I8, i8);
as_from_primitive!(Value::I16, i16);
as_from_primitive!(Value::I32, i32);
as_from_primitive!(Value::I64, i64);
as_from_primitive!(Value::F32, f32);
as_from_primitive!(Value::F64, f64);
as_from_primitive!(Value::USize, usize);
as_from_primitive!(Value::Char, char);


//unit
//from_primitive!(Value::Str, &'static str); //replaced by Cow for deserialization
from_primitive!(Value::Str, Cow<'static, str>);
from_primitive!(Value::String, String);
from_primitive!(Value::Newtype, Box<Value>);
//from_primitive!(Value::Seq, Vec<Value>); //replaced by From<Vec<T>>
//from_primitive!(Value::Bytes, Vec<u8>); //replaced by From Vec<T> 
from_primitive!(Value::DateTime, DateTime<Utc>);

from_primitive!(Value::Map, HashMap<String, Value>); //use map Value,Value and use generic like Seq
from_primitive!(Value::VFileBuilder, Arc<dyn VFileBuilder>);

from_primitive!(Value::Func, Arc<Box<dyn Fn() -> Value + Sync + Send>>);

from_primitive!(Value::NodeId, TreeNodeId);
from_primitive!(Value::AttributePath, AttributePath);
from_primitive!(Value::Attributes, Attributes);
from_primitive!(Value::ReflectStruct, Arc<dyn ReflectStruct + Sync + Send>);
//from_primitive!(Value::Option, Option<Box<Value>>);
//from_primitive!(Value::Option, Option<Value>);

impl From<Option<Box<Value>>> for Value 
{
  #[inline]
  fn from(input: Option<Box<Value>>) -> Self 
  {
     Value::Option(input) 
  }
}

/*impl From<Option<Box<String>>> for Value 
{
  #[inline]
  fn from(input: Option<Box<String>>) -> Self 
  {
     Value::Option(Some(Box::new(Value::from(input)))) 
  }
}
*/
/*impl<T> From<Option<Box<T>>> for Value
{
  #[inline]
  fn from(input : Option<Box<T>>) -> Value
  {
    Value::Option(Some(Box::new(Value::String("a".into()))))
  }
}*/


impl<T> From<Arc<T>> for Value
  where T : ReflectStruct + Sync + Send + 'static 
{
  #[inline]
  fn from(input : Arc<T>) -> Value
    where T : ReflectStruct + Sync + Send + 'static 
  {
     Value::ReflectStruct(input) 
  }
}

impl<T> From<Vec<T>> for Value
  where Value: From<T>, T : Clone
{
  #[inline]
  fn from(input : Vec<T>) -> Self
  {
    Value::Seq(input.iter().map(|value| Value::from(value.clone())).collect())
  }
}

impl From<(ValueFuncArg, Box<Value>)> for Value 
{
  #[inline]
  fn from(input: (ValueFuncArg, Box<Value>)) -> Self 
  {
      Value::FuncArg(input.0, input.1)
  }
}

impl From<&'static str> for Value
{
  #[inline]
  fn from(input : &'static str) -> Self
  {
    Value::Str(Cow::Borrowed(input))
  }
}

impl Value
{
  #[inline]
  pub fn as_string(&self) -> String
  {
    match self
    {
      Value::String(val) => val.to_string(),
      Value::Str(val) => (*val).to_string(),
      _ => panic!("Can't convert value to String"),
    }
  }

  #[inline]
  pub fn try_as_string(&self) -> Option<String>
  {
    match self
    {
      Value::String(val) => Some(val.to_string()),
      Value::Str(val) => Some((*val).to_string()),
      _ => None, //conversion Result<>
    }
  }

  #[inline]
  pub fn as_vec(&self) -> Vec<Value>
  {
    match self 
    {
      Value::Seq(val) => val.clone(),
      _ => panic!("Can't convert value to Vec"), 
    }
  }

  #[inline]
  pub fn try_as_vec(&self) -> Option<Vec<Value>>
  {
    match self 
    {
      Value::Seq(val) => Some(val.clone()),//to_vec ?
      _ => None, 
    }
  }

  #[inline]
  pub fn as_attributes(&self) -> Attributes
  {
    match self
    {
      Value::Attributes(val) => val.clone(),
      _ => panic!("Can't convert value to Attributes"),
    }
  }

  #[inline]
  pub fn try_as_attributes(&self) -> Option<Attributes>
  {
    match self
    {
      Value::Attributes(val) => Some(val.clone()),
      _ => None,
    }
  }

  #[inline]
  pub fn as_reflect_struct(&self) -> Arc<dyn ReflectStruct> 
  {
    match self
    {
      Value::ReflectStruct(val) => val.clone(),
      _ => panic!("Can't convert value to Attributes"),
    }
  }

  #[inline]
  pub fn try_as_reflect_struct(&self) -> Option<Arc<dyn ReflectStruct>>
  {
    match self
    {
      Value::ReflectStruct(val) => Some(val.clone()),
      _ => None,
    }
  }

  #[inline]
  pub fn as_vfile_builder(&self) -> Arc<dyn VFileBuilder>
  {
    match self
    {
      Value::VFileBuilder(val) => val.clone(),
      _ => panic!("Can't convert value to VFileBuilder"),
    }
  }

  #[inline]
  pub fn try_as_vfile_builder(&self) -> Option<Arc<dyn VFileBuilder>>
  {
    match self
    {
      Value::VFileBuilder(val) => Some(val.clone()),
      _ => None,
    }
  }

  #[inline]
  pub fn as_date_time(&self) -> DateTime<Utc> //ret as ref ? 
  {
    match self
    {
      Value::DateTime(val) => *val,
      _ => panic!("Can't convert value to VFileBuilder"),
    }
  }

  #[inline]
  pub fn try_as_date_time(&self) -> Option<DateTime<Utc>> //ret as ref ?
  {
    match self
    {
      Value::DateTime(val) => Some(*val),
      _ => None,
    }
  }
}


impl std::string::ToString for Value
{
  #[inline]
  fn to_string(&self) -> String
  {
    match self
    {
         //Value::None => String::from("None"),
         Value::Bool(val) => val.to_string(),

         Value::U8(val) => val.to_string(),
         Value::U16(val) => val.to_string(),
         Value::U32(val) => val.to_string(),
         Value::U64(val) => val.to_string(),

         Value::I8(val) => val.to_string(),
         Value::I16(val) => val.to_string(),
         Value::I32(val) => val.to_string(),
         Value::I64(val) => val.to_string(),

         Value::F32(val) => val.to_string(), 
         Value::F64(val) => val.to_string(), 

         Value::USize(val) => val.to_string(), 
 
         Value::Char(val) => val.to_string(), 
         Value::String(val) => val.to_string(),
         Value::Str(val) => (*val).to_string(),
          
         Value::Unit => String::from("()"),
         Value::Newtype(val) => val.to_string(),

         Value::Func(func) => func().to_string(),
         Value::FuncArg(func, arg) => func(Value::Newtype(arg.clone())).to_string(),//"Fn(".to_owned() + &arg.to_string() + ")",
         
         Value::Option(val) => format!("{:?}", val),
         Value::Seq(val) => format!("{:?}", val),
         Value::Bytes(val) => format!("{:?}", val),
         Value::DateTime(val) => format!("{:?}", val),
         Value::VFileBuilder(val) => format!("{:?}", val.size()), 
         //{
            //let mut file = val.open().unwrap(); //XXX return error
            //let mut buffer = [0; 16];
            //let _r = file.read(&mut buffer).unwrap();//XXX we can't use read_exact a buffer can be < 16
            //buffer
         //}),
         Value::NodeId(val) => format!("{:?}", val),
         Value::AttributePath(val) => format!("{:?}", val),
         Value::Map(val) => format!("{:?}", val),
         Value::Attributes(val) => format!("{:?}", val ),
         Value::ReflectStruct(val) => format!("{:?}", val ),
    }
  }
}

impl fmt::Debug for Value 
{
   #[inline]
   fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result 
   {
      match self {
         //Value::None => write!(f, "None"),
         Value::Bool(val) => write!(f, "{}", val),

         Value::U8(val) => write!(f, "{}", val),
         Value::U16(val) => write!(f, "{}", val),
         Value::U32(val) => write!(f, "{}", val),
         Value::U64(val) => write!(f, "{}", val),

         Value::I8(val) => write!(f, "{}", val),
         Value::I16(val) => write!(f, "{}", val),
         Value::I32(val) => write!(f, "{}", val),
         Value::I64(val) => write!(f, "{}", val),

         Value::F32(val) => write!(f, "{}", val),
         Value::F64(val) => write!(f, "{}", val),

         Value::USize(val) => write!(f, "{}", val),

         Value::Char(val) => write!(f, "'{}'", val),
         Value::String(val) => write!(f, "\"{}\"", val),
         Value::Str(val) => write!(f, "\"{}\"", val),
         
         Value::Unit => write!(f, "()"),
         Value::Option(val) => write!(f, "{:?}", val),
         Value::Newtype(val) => write!(f, "{:?}", val),
         Value::Seq(val) => write!(f, "{:?}", val),
         Value::Map(val) => write!(f, "{:?}", val),
         Value::Bytes(val) => write!(f, "{:?}", val),
         Value::DateTime(val) => write!(f, "{:?}", val),

         Value::Func(func) => write!(f, "{:?}", func()),
         Value::FuncArg(func, arg) => write!(f, "{:?}", func(Value::Newtype(arg.clone()))),
         Value::VFileBuilder(val) => write!(f, "{:?}", 
         { 
           let mut file = match val.open()
           {
             Ok(file) => file,
             Err(_err) => return write!(f, ""),//XXX ret some error ?
           };
           let mut buffer = [0; 16];
           let _r = match file.read(&mut buffer)
           {
             Ok(buff) => buff,
             Err(_err) => return write!(f, ""),//XXX ret some error ?
           };
           buffer
         }),
         Value::NodeId(val) => write!(f, "{:?}", val),
         Value::AttributePath(val) => write!(f, "{:?}", val),
         Value::Attributes(val) => write!(f, "{:?}", val),
         Value::ReflectStruct(val) => write!(f, "{:?}", val),
      }
   }
}

/*impl Serialize for Value
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self
        {
           Value::None => serializer.serialize_none(),
           Value::Bool(val) => serializer.serialize_bool(*val),

           Value::U8(val) => serializer.serialize_u8(*val),
           Value::U16(val) => serializer.serialize_u16(*val),
           Value::U32(val) => serializer.serialize_u32(*val),
           Value::U64(val) => serializer.serialize_u64(*val),

           Value::I8(val) => serializer.serialize_i8(*val),
           Value::I16(val) => serializer.serialize_i16(*val),
           Value::I32(val) => serializer.serialize_i32(*val),
           Value::I64(val) => serializer.serialize_i64(*val),

           Value::F32(val) => serializer.serialize_f32(*val), 
           Value::F64(val) => serializer.serialize_f64(*val), 

           Value::USize(val) => serializer.serialize_u64(*val as u64), //as u64 ?
 
           Value::Char(val) => serializer.serialize_char(*val), 
           Value::String(val) => serializer.serialize_str(&val),
           Value::Str(val) => serializer.serialize_str(&val),
          
           Value::Unit => serializer.serialize_unit(),

           Value::Newtype(val) => val.serialize(serializer),
           Value::Func(func) => func().serialize(serializer),

           Value::FuncArg(func, arg) => func(Value::Newtype(arg.clone())).serialize(serializer),
       
           Value::Option(val) => val.serialize(serializer),
           Value::Seq(val) => val.serialize(serializer),
           Value::Map(val) => val.serialize(serializer),
           //{
              //let mut map = serializer.serialize_map(Some(val.len()))?;
              //for (k, v) in val
              //{
                //map.serialize_entry(k, v)?;
              //}
              //map.end()
           //},
           Value::Bytes(val) => val.serialize(serializer),
           Value::VFileBuilder(val) => val.serialize(serializer), 
           Value::DateTime(val) => val.serialize(serializer),
           Value::NodeId(val) => val.serialize(serializer),
           Value::AttributePath(val) => val.serialize(serializer),
           Value::Attributes(val) => val.serialize(serializer),
        }
    }
}*/
