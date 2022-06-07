//! [MappedVFileBuilder] is a file system developement helper, you can use it to create a generator of `Reader`.
//! You don't need to implement [Read] or [Seek] method but just to add different pointer (offset and size) to [chunk](FileRanges) of data from an existing `Reader` to the container.

use std::io::Read; 
use std::io::Seek;
use std::io::SeekFrom;
use std::io::{Error, ErrorKind};
use std::sync::{Arc};

use serde::{Serialize, Deserialize};
use serde::de::{Deserializer};
use serde::ser::{Serializer, SerializeMap};

use crate::error::{RustructError};
use crate::vfile::{VFile, VFileBuilder};

use anyhow::Result;
use intervaltree::IntervalTree;
use lru::LruCache;

/**
 *  [FileRanges] contain a [Vec](Vec)<([Range](std::ops::Range)<u64>, [FileOffset])>.
 *  Each [range](std::ops::Range) is slice a of data representating a new futur generated file
 *  the generated file will be composed of all those slice concatenated in the order where the range was pushed.
 *  Each corresponding [FileOffset] contain a Builder which is the parent file from which the data is read
 *  and an offset from where to read the data in that parent file.
 */
#[derive(Default, Debug)]
pub struct FileRanges
{
  pub ranges : Vec<(std::ops::Range<u64>, FileOffset)>,
  pub id : u32,
}

impl FileRanges
{
  pub fn new() -> Self
  {
    FileRanges{ranges : Vec::new(), id : 0}
  }

  //return error if mapping offset is > as file size, or mapping overlap ?
  /// Add a new [`offset_range`](std::ops::Range) corresponding to a new block of the futur file,
  /// and the offset `builder_offset` from where to read the data in the parent [VFileBuilder] `builder`.
  pub fn push(&mut self, offset_range : std::ops::Range<u64>, builder_offset : u64, builder : Arc<dyn VFileBuilder>)
  {
    let file_offset = FileOffset{ builder, offset : builder_offset, id : self.id }; 
    self.id += 1;
    self.ranges.push((offset_range, file_offset));
  }
}

/**
 * This is an implementation of the trait [VFileBuilder] that help to easily write filesystem plugin
 * by creating a file builder that accept a [FileRanges] that help building the different chunk of data of the generated file.
 */
pub struct MappedVFileBuilder
{
 mapper : Arc< Mapper > //Is it better to clone or too slow for a file with lot of chunk ?
}

impl MappedVFileBuilder
{
  /// Return a new [VFileBuilder] from a [range](FileRanges) which contain [Range](std::ops::Range) and [FileOffset] helping build new file.
  pub fn new(file_ranges : FileRanges) -> Self
  {
    MappedVFileBuilder{mapper : Arc::new(Mapper::new(file_ranges))}
  }
}

#[typetag::serde]
impl VFileBuilder for MappedVFileBuilder
{
  /// When open is called it create a [VFile] from a clone of the internal `mapper`.
  fn open(&self) -> Result<Box<dyn VFile>>
  {
    Ok(Box::new(MappedVFile::new(self.mapper.clone())))
  }

  /// Return the size of the mapped file.
  fn size(&self) -> u64
  {
    self.mapper.size()
  }
}

impl Serialize for MappedVFileBuilder
{
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> 
    where S: Serializer,
  {
     let mut map = serializer.serialize_map(Some(1))?;

     map.serialize_entry("size", &self.size())?;
     map.end()
  }
}

impl<'de> Deserialize<'de> for MappedVFileBuilder
{
  fn deserialize<D>(_deserializer: D) -> std::result::Result<MappedVFileBuilder, D::Error>
  where
    D: Deserializer<'de>,
  {
    Err(serde::de::Error::custom("MappedVFileBuilder::deserialize not implemented")) 
  }
}

/**
 *  This implement [VFile] trait for [MappedVFile].
 *  This structure is created by [MappedVFileBuilder::open].
 *  The goal is to ease filesystem developement as the [Read] and [Seek] trait and `tell` function is implemented transparently.
 */
struct MappedVFile
{
  pub mapper : Arc<Mapper>,
  pub size : u64,
  pub pos : u64,
  pub cache : LruCache<u32, Box<dyn VFile>>,
}

impl MappedVFile
{
  /// Return a new [MappedVFile] from a [Arc]<[Mapper]>.
  /// This is used by [MappedVFileBuilder].
  fn new(mapper : Arc<Mapper>) -> Self
  {
    let size = mapper.size();
    let cache = LruCache::new(10); //get mapper number of vfile ?
    MappedVFile{ mapper, size, pos : 0, cache  }
  }

  // Return the current position of the cursor in the file
  //fn tell(&self) -> u64
  //{
    //self.pos
  //}

  /// Fill the buff with most data available, get from the provided offset in the virtually mapped file.
  fn fill(&mut self, buf : &mut [u8]) -> Result<u64>
  {
    let mut readed = 0;

    let to_read : u64 = match self.size - self.pos <  buf.len() as u64
    {
      true => self.size - self.pos,
      false => buf.len() as u64,
    };

    while readed < to_read && (readed as u64) < self.size
    {
      let elements: Vec<_> = self.mapper.tree.query_point(self.pos).collect();

      match elements.len()
      {
        len if len == 0 => return Ok(readed as u64),//must check if we're at end of a file ex: we read a block of 512 by default but the file size is only 20 so we must return 20 not error, 
        //XXX ret error  if we didn't find the elem XXX?
        len if len > 1 => return Err(RustructError::Unknown("Chunk overlap".into()).into()),
        _ => {
            let element = elements[0];
            //shift = current_offset in virtual file  - start of the currently found chunk
            //this give us the number of byte that we must skip inside this chunk
            let shift = self.pos - element.range.start;

            //we check if the builder returned by query point is opened and in cache
            let file = match self.cache.get_mut(&element.value.id)
            {
               Some(vfile) => vfile, 
               None =>
               {
                 let file = element.value.builder.open()?;
                 self.cache.put(element.value.id, file);
                 self.cache.get_mut(&element.value.id).unwrap() 
               },
            };

            //we seek to the offset that correspond inside the builder and we add the shift to go to the right position relatively to the start 
            let seeked = file.seek(SeekFrom::Start(element.value.offset + shift))?; //avoid seeking each time ? //check seek == end ! 
            if seeked !=  element.value.offset + shift
            {
              return Ok(readed as u64) //ok or error ?
            }

            //we calculate how many byte we have to read 
            //left = total byte to read - readed that's equal to the size we still need to read
            let left : u64 = to_read  - readed as u64;
            //if there is enough byte to read in this chunk we read of left
            //else we must read as much as we can until this range is finish
            //so at the next iteration the next builder will be opened and we will fill the buff from this one
            let size_to_read : u64 = if left > (element.range.end - self.pos)
            {
                element.range.end - self.pos
            }
            else 
            {
               left 
            };
            let n = file.read(&mut buf[readed as usize ..readed as usize + size_to_read as usize])?;
            if n == 0
            {
             return Ok(readed as u64)
            }
            
            readed += n as u64;
            self.pos += n as u64; //add n or size -...
        }
      }
    }
    Ok(readed as u64) 
  }
}

impl Read for MappedVFile
{
  /// [Read] implem of [MappedVFile].
  fn read(&mut self, buf : &mut [u8]) -> std::io::Result<usize>
  {
    match self.fill(buf)
    {
      Ok(n) => Ok(n as usize), //n != buff.len ...
      Err(err) => Err(Error::new(ErrorKind::Other, err)),
    }
  }
}

impl Seek for MappedVFile
{
  /// [Seek] implem of [MappedVFile].
  fn seek(&mut self, pos : SeekFrom) -> std::io::Result<u64>
  {
    let pos : u64 = match pos 
    {
      SeekFrom::Start(pos) => pos,
      SeekFrom::End(pos) => 
      { 
        if self.size as i64 + pos < 0 
          { return Err(Error::new(ErrorKind::Other, "MappedVFile::Seek : Can't seek past end of file")) };
        (self.size as i64 + pos) as u64 
      },
      SeekFrom::Current(pos) => (pos + self.pos as i64) as u64,
    };

    if pos <= self.size
    {
      self.pos = pos;
      return Ok(self.pos);
    }
  
    Err(Error::new(ErrorKind::Other, format!("MappedVFile::Seek : Can't seek to {} past end of file of size {}", pos, self.size)))
  }
}

/**
 * [FileOffset] contain a [`builder`](VFileBuilder), the `offset` from where we start reading the data of the builder, and a unique `id`.
 */
#[derive(Debug)]
pub struct FileOffset
{ 
  /// [Builder](VFileBuilder) from which data will be read.
  pub builder : Arc<dyn VFileBuilder>, 
  /// Offset of the data in the [VFileBuilder] `builder`.
  pub offset  : u64,
  /// Unique id for each file [FileOffset] in a [FileRanges], used by the cache to identify cached FileOffset.
  pub id : u32, 
}

/**
 *  [Mapper] contain an [interval tree](IntervalTree) containing the different [FileOffset] and the final `size` of the mapped file.
 *  it's an helper used internally by [MappedVFile]. Mapper can be used to get data from the different chunk composing the [MappedVFile] easily.
 */
struct Mapper
{
  tree : IntervalTree<u64, FileOffset>,
  size : u64,
}

impl Mapper
{
  /// Create a new [Mapper] from the [FileRanges] and [FileOffset] of the original file.
  /// It calculate the futur mapped file size from the different info passed.
  /// This struct is shared by the different instance of [VFile] created by the [VFileBuilder].
  fn new(file_ranges : FileRanges) -> Self //can raise error if validate is not ok 
  {
    let mut size : u64 = 0;

    for file_range in file_ranges.ranges.iter()
    {
      size += file_range.0.end - file_range.0.start;
    }
    Mapper{tree : file_ranges.ranges.into_iter().collect(), size}
  }

  /// Return the size of the mapped data.
  fn size(&self) -> u64
  {
    self.size
  }
}
