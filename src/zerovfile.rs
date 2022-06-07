use std::io::Read; 
use std::io::Seek;
use std::io::SeekFrom;
use std::io::{Error, ErrorKind};

use crate::vfile::{VFile, VFileBuilder};

use anyhow::Result;
use serde::{Serialize, Deserialize};

/**
 * VFileBuilder implementation for ZeroVFile.
 * A VFile with an infinize size that return data set to 0 can be used in a MappedVFile to simulate sparse zone.
 */
#[derive(Debug,Serialize,Deserialize)]
pub struct ZeroVFileBuilder
{
}

#[typetag::serde]
impl VFileBuilder for ZeroVFileBuilder
{
  fn open(&self) -> Result<Box<dyn VFile>>
  {
    Ok(Box::new(ZeroVFile{ pos : 0}))
  }

  fn size(&self) -> u64
  {
    //we're infinite ...
    u64::MAX
  }
}

/**
 * A VFile with an infinize size that return data set to 0 
 * can be used in a MappedVFile to simulate sparse zone.
 */
struct ZeroVFile
{
  pub pos : u64
}

impl Read for ZeroVFile
{
  fn read(&mut self, buf : &mut [u8]) -> std::io::Result<usize>
  {
    //we can zero buf, but generally buffer are already zeroed
    Ok(buf.len())
  }
}

impl Seek for ZeroVFile
{
  fn seek(&mut self, pos : SeekFrom) -> std::io::Result<u64>
  {
    let pos : u64 = match pos 
    {
      SeekFrom::Start(pos) => pos,
      SeekFrom::End(_pos) =>  return Err(Error::new(ErrorKind::Other, "MappedVFile::Seek : Can't seek past end of file")),
      SeekFrom::Current(pos) => (pos + self.pos as i64) as u64,
    };
    self.pos = pos;
    Ok(self.pos)
  }
}
