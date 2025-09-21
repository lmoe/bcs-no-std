#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use core::convert::TryFrom;
use serde::{ser, Serialize};

use crate::error::{Error, Result};

pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>>
where
    T: ?Sized + Serialize,
{
    let mut output = Vec::new();
    let serializer = Serializer::new(&mut output, crate::MAX_CONTAINER_DEPTH);
    value.serialize(serializer)?;
    Ok(output)
}

pub fn to_bytes_with_limit<T>(value: &T, limit: usize) -> Result<Vec<u8>>
where
    T: ?Sized + Serialize,
{
    if limit > crate::MAX_CONTAINER_DEPTH {
        return Err(Error::NotSupported("limit exceeds the max allowed depth"));
    }
    let mut output = Vec::new();
    let serializer = Serializer::new(&mut output, limit);
    value.serialize(serializer)?;
    Ok(output)
}

pub fn serialized_size<T>(value: &T) -> Result<usize>
where
    T: ?Sized + Serialize,
{
    let mut counter = SizeCounter(0);
    let serializer = Serializer::new(&mut counter, crate::MAX_CONTAINER_DEPTH);
    value.serialize(serializer)?;
    Ok(counter.0)
}

pub fn serialized_size_with_limit<T>(value: &T, limit: usize) -> Result<usize>
where
    T: ?Sized + Serialize,
{
    if limit > crate::MAX_CONTAINER_DEPTH {
        return Err(Error::NotSupported("limit exceeds the max allowed depth"));
    }
    let mut counter = SizeCounter(0);
    let serializer = Serializer::new(&mut counter, limit);
    value.serialize(serializer)?;
    Ok(counter.0)
}

// Simple write trait for no_std
trait BcsWrite {
    fn write_all(&mut self, buf: &[u8]) -> Result<()>;
}

impl BcsWrite for Vec<u8> {
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.extend_from_slice(buf);
        Ok(())
    }
}

struct SizeCounter(usize);

impl BcsWrite for SizeCounter {
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.0 = self.0.checked_add(buf.len()).ok_or(Error::BufferFull)?;
        Ok(())
    }
}

struct Serializer<'a, W: ?Sized> {
    output: &'a mut W,
    max_remaining_depth: usize,
}

impl<'a, W> Serializer<'a, W>
where
    W: ?Sized + BcsWrite,
{
    fn new(output: &'a mut W, max_remaining_depth: usize) -> Self {
        Self {
            output,
            max_remaining_depth,
        }
    }

    fn output_u32_as_uleb128(&mut self, mut value: u32) -> Result<()> {
        while value >= 0x80 {
            let byte = (value & 0x7f) as u8;
            self.output.write_all(&[byte | 0x80])?;
            value >>= 7;
        }
        self.output.write_all(&[value as u8])?;
        Ok(())
    }

    fn output_variant_index(&mut self, v: u32) -> Result<()> {
        self.output_u32_as_uleb128(v)
    }

    fn output_seq_len(&mut self, len: usize) -> Result<()> {
        if len > crate::MAX_SEQUENCE_LENGTH {
            return Err(Error::ExceededMaxLen(len));
        }
        self.output_u32_as_uleb128(len as u32)
    }

    fn enter_named_container(&mut self, name: &'static str) -> Result<()> {
        if self.max_remaining_depth == 0 {
            return Err(Error::ExceededContainerDepthLimit(name));
        }
        self.max_remaining_depth -= 1;
        Ok(())
    }
}

impl<'a, W> ser::Serializer for Serializer<'a, W>
where
    W: ?Sized + BcsWrite,
{
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = MapSerializer<'a, W>;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn collect_str<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + core::fmt::Display,
    {
        use alloc::string::ToString;
        self.serialize_str(&value.to_string())
    }
    
    fn serialize_bool(self, v: bool) -> Result<()> {
        self.serialize_u8(v as u8)
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_u8(v as u8)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_u16(v as u16)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_u32(v as u32)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_i128(self, v: i128) -> Result<()> {
        self.serialize_u128(v as u128)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.output.write_all(&[v])
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.output.write_all(&v.to_le_bytes())
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.output.write_all(&v.to_le_bytes())
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.output.write_all(&v.to_le_bytes())
    }

    fn serialize_u128(self, v: u128) -> Result<()> {
        self.output.write_all(&v.to_le_bytes())
    }

    fn serialize_f32(self, _v: f32) -> Result<()> {
        Err(Error::NotSupported("serialize_f32"))
    }

    fn serialize_f64(self, _v: f64) -> Result<()> {
        Err(Error::NotSupported("serialize_f64"))
    }

    fn serialize_char(self, _v: char) -> Result<()> {
        Err(Error::NotSupported("serialize_char"))
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.serialize_bytes(v.as_bytes())
    }

    fn serialize_bytes(mut self, v: &[u8]) -> Result<()> {
        self.output_seq_len(v.len())?;
        self.output.write_all(v)
    }

    fn serialize_none(self) -> Result<()> {
        self.serialize_u8(0)
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.output.write_all(&[1])?;
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_struct(mut self, name: &'static str) -> Result<()> {
        self.enter_named_container(name)?;
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        mut self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        self.enter_named_container(name)?;
        self.output_variant_index(variant_index)
    }

    fn serialize_newtype_struct<T>(mut self, name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.enter_named_container(name)?;
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        mut self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.enter_named_container(name)?;
        self.output_variant_index(variant_index)?;
        value.serialize(self)
    }

    fn serialize_seq(mut self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        if let Some(len) = len {
            self.output_seq_len(len)?;
            Ok(self)
        } else {
            Err(Error::MissingLen)
        }
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(self)
    }

    fn serialize_tuple_struct(
        mut self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.enter_named_container(name)?;
        Ok(self)
    }

    fn serialize_tuple_variant(
        mut self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.enter_named_container(name)?;
        self.output_variant_index(variant_index)?;
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(MapSerializer::new(self))
    }

    fn serialize_struct(
        mut self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        self.enter_named_container(name)?;
        Ok(self)
    }

    fn serialize_struct_variant(
        mut self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.enter_named_container(name)?;
        self.output_variant_index(variant_index)?;
        Ok(self)
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

impl<'a, W> ser::SerializeSeq for Serializer<'a, W>
where
    W: ?Sized + BcsWrite,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(Serializer::new(self.output, self.max_remaining_depth))
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W> ser::SerializeTuple for Serializer<'a, W>
where
    W: ?Sized + BcsWrite,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(Serializer::new(self.output, self.max_remaining_depth))
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W> ser::SerializeTupleStruct for Serializer<'a, W>
where
    W: ?Sized + BcsWrite,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(Serializer::new(self.output, self.max_remaining_depth))
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W> ser::SerializeTupleVariant for Serializer<'a, W>
where
    W: ?Sized + BcsWrite,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(Serializer::new(self.output, self.max_remaining_depth))
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

struct MapSerializer<'a, W: ?Sized> {
    serializer: Serializer<'a, W>,
    entries: Vec<(Vec<u8>, Vec<u8>)>,
    next_key: Option<Vec<u8>>,
}

impl<'a, W: ?Sized> MapSerializer<'a, W> {
    fn new(serializer: Serializer<'a, W>) -> Self {
        MapSerializer {
            serializer,
            entries: Vec::new(),
            next_key: None,
        }
    }
}

impl<'a, W> ser::SerializeMap for MapSerializer<'a, W>
where
    W: ?Sized + BcsWrite,
{
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.next_key.is_some() {
            return Err(Error::ExpectedMapValue);
        }

        let mut output = Vec::new();
        key.serialize(Serializer::new(&mut output, self.serializer.max_remaining_depth))?;
        self.next_key = Some(output);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        match self.next_key.take() {
            Some(key) => {
                let mut output = Vec::new();
                value.serialize(Serializer::new(&mut output, self.serializer.max_remaining_depth))?;
                self.entries.push((key, output));
                Ok(())
            }
            None => Err(Error::ExpectedMapKey),
        }
    }

    fn end(mut self) -> Result<()> {
        if self.next_key.is_some() {
            return Err(Error::ExpectedMapValue);
        }

        // Sort entries for canonical encoding
        self.entries.sort_by(|e1, e2| e1.0.cmp(&e2.0));

        // Manual duplicate removal since we want to avoid depending on additional traits
        let mut write_idx = 0;
        for read_idx in 1..self.entries.len() {
            if self.entries[write_idx].0 != self.entries[read_idx].0 {
                write_idx += 1;
                if write_idx != read_idx {
                    self.entries.swap(write_idx, read_idx);
                }
            }
        }
        if !self.entries.is_empty() {
            self.entries.truncate(write_idx + 1);
        }

        let len = self.entries.len();
        self.serializer.output_seq_len(len)?;

        for (key, value) in &self.entries {
            self.serializer.output.write_all(key)?;
            self.serializer.output.write_all(value)?;
        }

        Ok(())
    }
}

impl<'a, W> ser::SerializeStruct for Serializer<'a, W>
where
    W: ?Sized + BcsWrite,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(Serializer::new(self.output, self.max_remaining_depth))
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W> ser::SerializeStructVariant for Serializer<'a, W>
where
    W: ?Sized + BcsWrite,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(Serializer::new(self.output, self.max_remaining_depth))
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}