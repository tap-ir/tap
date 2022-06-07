//! A [VFileBuilder] that cache in memory the content of an other [VFileBuilder].

use std::io::{Read, Seek, SeekFrom}; 
use std::io::{Error, ErrorKind};
use std::sync::Arc;

use crate::vfile::{VFile, VFileBuilder};

use serde::{Serialize, Deserialize};
use serde::de::{Deserializer};
use serde::ser::{Serializer, SerializeMap};

/**
 * Implement a [VFileBuilder] that cache in memory the content of an other [VFileBuilder].
 */
pub struct MemoryVFileBuilder
{
  buffer : Arc<Vec<u8>>,
}

impl MemoryVFileBuilder
{
  /// `builder` will be used to generate a `VFile` read it's content end cache it in internal `buffer`.
  /// The whole file will be read and cached in ram, so the passed [VFileBuilder] generated file must fit in memory.
  pub fn new(builder : Arc<dyn VFileBuilder>) -> anyhow::Result<Arc<MemoryVFileBuilder>>
  {
    let mut file = builder.open()?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    Ok(Arc::new(MemoryVFileBuilder{ buffer : Arc::new(buffer) }))
  }
}

#[typetag::serde]
impl VFileBuilder for MemoryVFileBuilder
{
  fn open(&self) -> anyhow::Result<Box<dyn VFile>>
  {
    Ok(Box::new(MemoryVFile::new(self.buffer.clone())))
  }

  fn size(&self) -> u64
  { 
    self.buffer.as_ref().len() as u64
  }
}

impl Serialize for MemoryVFileBuilder 
{
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> 
    where S: Serializer,
  {
     let mut map = serializer.serialize_map(Some(1))?;

     map.serialize_entry("size", &self.size())?;
     map.end()
  }
}

impl<'de> Deserialize<'de> for MemoryVFileBuilder 
{
  fn deserialize<D>(_deserializer: D) -> std::result::Result<MemoryVFileBuilder, D::Error>
  where
    D: Deserializer<'de>,
  {
    Err(serde::de::Error::custom("MemoryVFileBuilder::deserialize not implemented")) 
  }
}

/**
 * [MemoryVFile] implement [VFile] [Read] + [Seek] trait for a [Vec]<[u8]>.
 */
pub struct MemoryVFile
{
  buffer : Arc<Vec<u8>>,
  pos : u64,
}

impl MemoryVFile
{
  pub fn new(buffer : Arc<Vec<u8>>) -> MemoryVFile
  {
    MemoryVFile{buffer, pos : 0 }
  }

  pub fn remaining_slice(&self) -> &[u8] 
  {
    let len = self.pos.min(self.buffer.as_ref().len() as u64);
    &self.buffer.as_ref()[(len as usize)..]
  }
}


impl Read for MemoryVFile
{
  fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> 
  {
    let n = Read::read(&mut self.remaining_slice(), buf)?;
    self.pos += n as u64;
    Ok(n)
  }
}

impl Seek for MemoryVFile
{
  fn seek(&mut self, style: SeekFrom) -> std::io::Result<u64> 
  {
    let (base_pos, offset) = match style 
    {
      SeekFrom::Start(n) => 
      {
        self.pos = n;
        return Ok(n);
      }
      SeekFrom::End(n) => (self.buffer.as_ref().len() as u64, n),
      SeekFrom::Current(n) => (self.pos, n),
    };

    let new_pos = if offset >= 0 
    {
      base_pos.checked_add(offset as u64)
    } 
    else 
    {
      base_pos.checked_sub((offset.wrapping_neg()) as u64)
    };

    match new_pos 
    {
      Some(n) => 
      {
        self.pos = n;
        Ok(self.pos)
      }
      None => Err(Error::new(ErrorKind::Other, "MemoryVFileBuilder: invalid seek to a negative or overflowing position")),
    }
  }
}
