//! VFileBuilder is a trait that help build new 'Virtual File'.
//! It's implemented as a trait with an open function returning a VFile (Read+Seek) 
//! this can be added as Value to the tree and so permit to create a stacked VFS 
//! as node can read on a VFile and generate a new one

use std::io;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::fmt;

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};

/**
 *  A trait that generate [VFile] trait object. 
 */
#[typetag::serde(tag = "type")]
pub trait VFileBuilder : Sync + Send
{
  /// Create and a return a [VFile] trait object
  fn open(&self) -> Result<Box<dyn VFile>>;
  /// Return the size of the created [VFile]
  fn size(&self) -> u64;
}

impl std::fmt::Debug for dyn VFileBuilder
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result 
  {
     write!(f, "VFileBuilder")
  }
}

/*impl Serialize for dyn VFileBuilder + Sync + Send 
{
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> 
    where S: Serializer,
  {
     let mut map = serializer.serialize_map(Some(1))?;

     map.serialize_entry("size", &self.size())?;
     map.end()

     serialize the first 16 bytes of data 
     let mut buffer = [0; 16];
     let mut file = match val.open() //XXX return error
     {
       Ok(file) => file,
       Err(err) => return Err(serde::ser::Error::custom(err)),
     };
     let len = match file.read(&mut buffer)
     {
       Ok(file) => file,
       Err(_err) => 0, //return Err(serde::ser::Error::custom(err)), serialize empty buffer to avoid serialization error
     }; //XXX check read ret val
     serializer.serialize_bytes(&buffer[0..len])
  }
}
*/

/*impl<'de> Deserialize<'de> for dyn VFileBuilder + Sync + Send
{
  fn deserialize<D>(deserializer: D) -> std::result::Result<D::Error, D::Error>
  where
    D: Deserializer<'de>,
  {
    Err(serde::de::Error::custom("VFileBuilder::deserialize not implemented")) 
  }
}*/

/**
 *  A trait that implement [Read] + [Seek].
 */
pub trait VFile : Read + Seek + Sync + Send 
{
  fn tell(&mut self) -> io::Result<u64> 
  {
    self.seek(SeekFrom::Current(0))
  }
}

impl<T: Read + Seek + Sync + Send > VFile for T 
{
}

// This is some helper function 

/**
 *  Read an UTF-16 string from `file` of size `size` and return a [String] 
 *  `size` is the size in byte of the u16 string.
 **/
pub fn read_utf16_exact<T : VFile>(file : &mut T, size : usize) -> Result<String>
{
  let mut data = vec![0; size];
  file.read_exact(&mut data)?;

  let iter = (0..(size/2)).map(|i| u16::from_le_bytes([data[(2*i) as usize], data[(2*i+1) as usize]]));
  let iter = iter.take_while(|&byte| byte != 0x00);
  std::char::decode_utf16(iter).collect::<std::result::Result<String, _>>().map_err(|err| err.into())
}

/**
 *  Read a Pascal UTF-16 string from `file, first read the string size as a [u16] then read the data and convert it to String 
 *  `size` is the size in byte of the u16 string.
 */
pub fn read_sized_utf16<T: VFile>(file : &mut T) -> Result<String> //pascal_utf16 or tlv_utf16?
{
  let size = file.read_u16::<LittleEndian>()?; 
  read_utf16_exact(file, ((size *2) + 2 )as usize) //XXX read_utf16 should take an utf16 size (u8 size/2)
}

/**
 *  Read a consecutive list of UTF-16 String from a slice of `file` of size `size`.
 **/
pub fn read_utf16_list<T : VFile>(file : &mut T, size : usize) -> Result<Vec<String>>
{
  let mut data = vec![0; size];
  file.read_exact(&mut data)?; //limit size to avoid too large buffer
 
  let mut list : Vec<String> = Vec::new();
  let mut current_string : Vec<u16> = Vec::new();
  for i in 0..(size/2)
  {
    let b = u16::from_le_bytes([data[(2*i) as usize], data[(2*i+1) as usize]]); //call read each time?
    if b == 0x00 {
      let decoded =  std::char::decode_utf16(current_string).collect::<std::result::Result<String, _>>().unwrap(); //map_err(|err| err.into())?;
      list.push(decoded);
      current_string = Vec::new();
    }
    else {
     current_string.push(b);
    }
  }

  Ok(list)
}
