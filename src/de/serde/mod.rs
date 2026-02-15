use serde::de::{
    self, Deserialize, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess,
    VariantAccess, Visitor,
};
use serde::Deserializer;

use std::iter::{Peekable, Take};
use std::num::{ParseFloatError, ParseIntError};
use std::ops::Range;
use std::str::FromStr;

use super::{KeyValue, LexerChildren};
use crate::error::DeserializerError;
use crate::serde::atom::AtomParser;

pub mod atom;

pub fn from_str<'a, T>(s: &'a str) -> Result<T, DeserializerError>
where
    T: Deserialize<'a>,
{
    let mut iter = s
        .lines()
        .filter(|x| !x.trim().is_empty())
        .filter(|x| !x.starts_with('#'));
    let start = s
        .lines()
        .take_while(|x| x.starts_with('#') || x.is_empty())
        .map(|x| x.len() + 1)
        .sum();
    let mut lex = Parser::new(iter);
    #[cfg(feature = "tracing")]
    tracing::info!("Setting start to: {}", start);
    lex.iter.byte_offset = start..start;
    T::deserialize(&mut lex)
}

struct Parser<'de, I: Iterator> {
    iter: LexerChildren<'de, Peekable<I>>,
    deser_any_col: bool,
}

impl<'de, I: Iterator<Item = &'de str>> Parser<'de, I> {
    fn new(iter: I) -> Self {
        let iter = LexerChildren::new(iter.peekable());
        Self {
            iter,
            deser_any_col: false,
        }
    }
    fn key_value(&mut self) -> Option<KeyValue<'de>> {
        self.iter.next().and_then(KeyValue::new)
    }
    fn value(&mut self) -> Result<(&'de str, Range<usize>), DeserializerError> {
        let kv = self
            .key_value()
            .ok_or(DeserializerError::ExpectedValueNode)?;
        let val = kv.path().next().unwrap_or(kv.value);
        // SAFETY: `val` comes from `kv.orig`
        let value_start =
            val.as_ptr() as usize - kv.orig.as_ptr() as usize + self.iter.byte_offset.start;
        let range = value_start..value_start + val.len();
        #[cfg(feature = "tracing")]
        tracing::trace!(
            full = kv.orig,
            range = tracing::field::debug(&range),
            slice = kv.orig.get(range.clone()),
            value = val
        );
        Ok((val, range))
    }
    fn atom(&mut self) -> Result<AtomParser<'de>, DeserializerError> {
        let (input, span) = self.value()?;
        Ok(AtomParser { input, span })
    }
}

impl<'de, I: Iterator<Item = &'de str>> Parser<'de, I> {
    fn peek_key_value(&mut self) -> Result<KeyValue<'de>, DeserializerError> {
        self.iter
            .peek()
            .and_then(KeyValue::new)
            .ok_or(DeserializerError::ExpectedKeyValuePair)
    }
}

impl<'a, 'de, I: 'de> Deserializer<'de> for &'a mut Parser<'de, I>
where
    I: Iterator<Item = &'de str>,
{
    type Error = DeserializerError;

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, visitor)))]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let kv = self.peek_key_value()?;
        let level = kv.path().count();
        if !self.deser_any_col && level > 0 {
            let ident = kv.path().next().unwrap();
            if ident.chars().all(|x| x.is_ascii_digit()) {
                self.deserialize_seq(visitor)
            } else {
                self.deserialize_map(visitor)
            }
        } else {
            self.atom()?.deserialize_any(visitor).map_err(Into::into)
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?.deserialize_bool(visitor).map_err(Into::into)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?.deserialize_i8(visitor).map_err(Into::into)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?.deserialize_i16(visitor).map_err(Into::into)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?.deserialize_i32(visitor).map_err(Into::into)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, visitor)))]
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?.deserialize_i64(visitor).map_err(Into::into)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?.deserialize_u8(visitor).map_err(Into::into)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?.deserialize_u16(visitor).map_err(Into::into)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?.deserialize_u32(visitor).map_err(Into::into)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?.deserialize_u64(visitor).map_err(Into::into)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?.deserialize_f32(visitor).map_err(Into::into)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?.deserialize_f64(visitor).map_err(Into::into)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?.deserialize_char(visitor).map_err(Into::into)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?.deserialize_str(visitor).map_err(Into::into)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?.deserialize_string(visitor).map_err(Into::into)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?.deserialize_bytes(visitor).map_err(Into::into)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?
            .deserialize_byte_buf(visitor)
            .map_err(Into::into)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, visitor)))]
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(SeqParser::new(self))
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?
            .deserialize_tuple(len, visitor)
            .map_err(Into::into)
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(self)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.atom()?
            .deserialize_identifier(visitor)
            .map_err(Into::into)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

impl<'de, I: Iterator<Item = &'de str> + 'de> MapAccess<'de> for Parser<'de, I> {
    type Error = DeserializerError;

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, seed)))]
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        #[cfg(feature = "tracing")]
        tracing::debug!(
            prev = self.iter.cache,
            cur = self.iter.lines.peek(),
            prefix = self.iter.prefix
        );
        if self.iter.is_finished() {
            #[cfg(feature = "tracing")]
            tracing::trace!("------------- done ----------");
            return Ok(None);
        }
        // dbg!(self.lines.peek());
        self.deser_any_col = true;
        let val = seed.deserialize(&mut *self).map(Some);
        self.deser_any_col = false;
        self.iter.increment_prefix_level();
        val
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, seed)))]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let val = seed.deserialize(&mut *self);
        self.iter.decrement_prefix_level();
        self.iter.next();
        val
    }
}

const SEQ_ENDER: &'static [&'static str] = &["length", "num"];

#[cfg(feature = "alloc")]
extern crate alloc;

pub struct SeqParser<'a, 'de, I: Iterator<Item = &'de str>> {
    /// Original parser
    de: &'a mut Parser<'de, I>,
    #[cfg(feature = "alloc")]
    /// The cached lines read after figuring out the length of seq.
    read_lines: alloc::vec::Vec<&'de str>,
    #[cfg(feature = "alloc")]
    /// The cached lines read after figuring out the length of seq.
    read_lines_iter: Option<Parser<'de, alloc::vec::IntoIter<&'de str>>>,
    #[cfg(feature = "alloc")]
    /// The seen indices so far.
    read_indices: alloc::collections::BTreeSet<i64>,
    /// The read length if found
    read_length: Option<i64>,
    // The next sequence index expected to be read
    index: i64,
}

impl<'a, 'de, I: Iterator<Item = &'de str>> SeqParser<'a, 'de, I> {
    fn new(de: &'a mut Parser<'de, I>) -> Self {
        Self {
            de,
            index: 0,
            #[cfg(feature = "alloc")]
            read_lines: Vec::new(),
            #[cfg(feature = "alloc")]
            read_lines_iter: None,
            #[cfg(feature = "alloc")]
            read_indices: alloc::collections::BTreeSet::new(),
            read_length: None,
        }
    }
}

impl<'a, 'de, I: Iterator<Item = &'de str> + 'de> SeqAccess<'de> for SeqParser<'a, 'de, I> {
    type Error = DeserializerError;

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, seed)))]
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        #[cfg(not(feature = "alloc"))]
        {
            if self.de.iter.is_finished() {
                return Ok(None);
            }

            let (ident, span) = self.de.value()?;
            if ident.chars().all(|x| x.is_ascii_digit()) {
                self.de.iter.increment_prefix_level();
                let val = seed.deserialize(&mut *self.de);
                self.de.iter.decrement_prefix_level();
                self.de.iter.next();
                Some(val).transpose()
            } else if SEQ_ENDER.iter().any(|x| ident.eq_ignore_ascii_case(x)) {
                // length always comes last due to lexicographic ordering
                // 0-9 < a-z
                Ok(None)
            } else {
                Err(DeserializerError::ExpectedSequenece { unexpected: span })
            }
        }
        #[cfg(feature = "alloc")]
        {
            if self.de.iter.is_finished() && self.read_lines.is_empty() {
                return Ok(None);
            }
            // HACK: This code sucks

            #[cfg(feature = "tracing")]
            tracing::debug!(
                index = self.index,
                length = self.read_length,
                values = tracing::field::debug(&self.read_indices)
            );

            while !self.de.iter.is_finished() {
                if let Some(str) = self.de.iter.peek() {
                    self.read_lines.push(str);
                }
                let (ident, span) = self.de.value()?;
                #[cfg(feature = "tracing")]
                tracing::debug!(
                    ident = ident,
                    span = tracing::field::debug(&span),
                    "Initial length read."
                );

                if ident.chars().all(|x| x.is_ascii_digit()) {
                    let ident = ident.parse::<i64>().unwrap();
                    self.read_indices.insert(ident);
                } else if SEQ_ENDER.iter().any(|x| ident.eq_ignore_ascii_case(x)) {
                    self.de.iter.increment_prefix_level();
                    let marker = std::marker::PhantomData::<i64>;
                    self.read_length = marker.deserialize(&mut *self.de).ok();
                    self.de.iter.decrement_prefix_level();
                    break;
                } else {
                    tracing::error!(ident=ident, "Got something unexpected.");
                    return Err(DeserializerError::ExpectedSequenece { unexpected: span });
                }
            }

            while !self.read_indices.contains(&self.index)
                && self
                    .read_indices
                    .iter()
                    .find(|i| i > &&self.index)
                    .is_some()
            {
                self.index += 1;
            }

            // TODO: get rid of this clone
            let mut lookup = Parser::new(self.read_lines.clone().into_iter());
            if !self.read_indices.contains(&self.index)
                || self
                    .read_length
                    .map(|x| self.index >= x)
                    .unwrap_or_default()
            {
                self.index = 0;
                loop {
                    if lookup.iter.is_finished() {
                        break;
                    }
                    let (ident, span) = lookup.value()?;

                    #[cfg(feature = "tracing")]
                    tracing::debug!(ident= ident,  "Final read.");

                    if ident.chars().all(|x| x.is_ascii_digit()) {
                        continue;
                    } else if SEQ_ENDER.iter().any(|x| ident.eq_ignore_ascii_case(x)) {
                        break;
                    } else {
                        // HACK: get the actual span from the outer parser
                        return Err(DeserializerError::ExpectedSequenece { unexpected: span });
                    }
                }
                Ok(None)
            } else {
                let mut value = None;
                while value.is_none() {
                    if lookup.iter.is_finished() {
                        break;
                    }
                    let (ident, _span) = lookup.value()?;
                    #[cfg(feature = "tracing")]
                    tracing::debug!(ident= ident,  "Reading value.");

                    if ident.chars().all(|x| x.is_ascii_digit()) {
                        let ident = ident.parse::<i64>().unwrap();
                        if ident == self.index {
                            lookup.iter.increment_prefix_level();
                            value = Some(seed.deserialize(&mut lookup));
                            lookup.iter.decrement_prefix_level();
                            break;
                        }
                    } else {
                        break;
                    }
                }

                self.index += 1;
                value.transpose()
            }
        }
    }
}

impl<'a, 'de, I: Iterator<Item = &'de str> + 'de> EnumAccess<'de> for &'a mut Parser<'de, I> {
    type Error = DeserializerError;

    type Variant = Self;

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, seed)))]
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        self.deser_any_col = true;
        let val = seed.deserialize(&mut *self);
        self.deser_any_col = false;
        val.map(|x| (x, self))
    }
}

impl<'a, 'de, I: Iterator<Item = &'de str> + 'de> VariantAccess<'de> for &'a mut Parser<'de, I> {
    type Error = DeserializerError;

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, seed)))]
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.iter.increment_prefix_level();
        let val = seed.deserialize(&mut *self);
        self.iter.decrement_prefix_level();
        val
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, visitor)))]
    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.iter.increment_prefix_level();
        let val = self.deserialize_tuple(len, visitor);
        self.iter.decrement_prefix_level();
        val
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, visitor)))]
    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.iter.increment_prefix_level();
        let val = self.deserialize_map(visitor);
        self.iter.decrement_prefix_level();
        val
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_derive::Deserialize;
    use test_log::test;

    use super::*;

    #[test]
    fn read_map() {
        let input = "foo = 1
bar = 2
baz = 3
quux = 4
";
        let mut expected = HashMap::new();
        expected.insert("foo", 1);
        expected.insert("bar", 2);
        expected.insert("baz", 3);
        expected.insert("quux", 4);
        assert_eq!(from_str(input), Ok(expected));
    }
    #[test]
    fn read_nested_map() {
        let input = "foo.bar = 1
foo.baz = 2
bar.baz = 3
bar.quux = 4";
        let mut expected = HashMap::new();

        let mut foo = HashMap::new();
        foo.insert("bar", 1);
        foo.insert("baz", 2);

        let mut bar = HashMap::new();
        bar.insert("baz", 3);
        bar.insert("quux", 4);

        expected.insert("foo", foo);
        expected.insert("bar", bar);
        assert_eq!(from_str(input), Ok(expected));
    }

    #[test]
    fn read_struct() {
        #[derive(Debug, PartialEq, Deserialize)]
        struct Test {
            foo: u32,
            bar: f32,
            baz: bool,
            inner: Inner,
            #[serde(flatten)]
            custom: HashMap<String, String>,
        }
        #[derive(Debug, PartialEq, Deserialize)]
        struct Inner {
            name: String,
            id: u32,
        }
        let input = "foo=32
bar=1.234
baz=true
inner.name=John Smith
inner.id=69
extra=stuff";
        let data: Test = from_str(input).unwrap();
        let mut custom = HashMap::new();
        custom.insert("extra".to_string(), "stuff".to_string());
        let expected = Test {
            foo: 32,
            bar: 1.234,
            baz: true,
            inner: Inner {
                name: "John Smith".to_string(),
                id: 69,
            },
            custom,
        };
        assert_eq!(data, expected);
    }

    #[test]
    fn read_seq() {
        let input = "0=0
1=1
10=10
11=11
2=2
3=3
4=4
5=5
6=6
7=7
8=8
9=9
length=12";
        let data: Vec<i64> = from_str(input).unwrap();
        assert_eq!(data, (0..12).collect::<Vec<_>>());
    }

    #[test]
    fn read_bad_seq() {
        let input = "0=0
1=1
10=10
11=11
2=2
3=3
4=4
5=5
6=6
7=7
8=8
9=9
length=10";
        let data: Vec<i64> = from_str(input).unwrap();
        assert_eq!(data, (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn read_unsized_seq() {
        let input = "0=0
1=1
10=10
11=11
2=2
3=3
4=4
5=5
6=6
7=7
8=8
9=9";
        let data: Vec<i64> = from_str(input).unwrap();
        assert_eq!(data, (0..12).collect::<Vec<_>>());
    }

    #[test]
    fn read_nested_struct() {
        let input = "
view_point.aspect=1.77778
view_point.fov.type=1
view_point.fov.value=0.93616
view_point.fov_is_horizontal=1
rot.x.type=0
rot.y.type=0
rot.z.type=0
scale.x.type=1
scale.x.value=1
scale.y.type=1
scale.y.value=1
scale.z.type=1
scale.z.value=1
trans.x.type=0
trans.y.type=0
trans.z.type=0
";
        #[derive(Debug, PartialEq, Deserialize)]
        struct Camera {
            #[serde(flatten)]
            transform: ModelTransform,
            view_point: ViewPoint,
        }
        #[derive(Debug, PartialEq, Deserialize)]
        struct ModelTransform {
            trans: Vec3<KeySet>,
            scale: Vec3<KeySet>,
            rot: Vec3<KeySet>,
        }
        #[derive(Debug, PartialEq, Deserialize)]
        struct Vec3<T> {
            x: T,
            y: T,
            z: T,
        }
        #[derive(Debug, PartialEq, Deserialize)]
        struct ViewPoint {
            aspect: f32,
            #[serde(flatten)]
            fov: FieldOfView,
        }
        #[derive(Debug, PartialEq, Deserialize)]
        struct FieldOfView {
            #[serde(rename = "fov_is_horizontal")]
            horizontal: u8,
            #[serde(rename = "fov")]
            value: KeySet,
        }
        #[derive(Debug, PartialEq, Deserialize)]
        struct KeySet {
            #[serde(rename = "type")]
            ty: u8,
            #[serde(default)]
            value: f64,
        }

        let val: Camera = from_str(input).unwrap();
        #[cfg(feature = "tracing")]
        tracing::debug!(value = tracing::field::debug(&val));
        let expected = Camera {
            transform: ModelTransform {
                trans: Vec3 {
                    x: KeySet { ty: 0, value: 0.0 },
                    y: KeySet { ty: 0, value: 0.0 },
                    z: KeySet { ty: 0, value: 0.0 },
                },
                scale: Vec3 {
                    x: KeySet { ty: 1, value: 1.0 },
                    y: KeySet { ty: 1, value: 1.0 },
                    z: KeySet { ty: 1, value: 1.0 },
                },
                rot: Vec3 {
                    x: KeySet { ty: 0, value: 0.0 },
                    y: KeySet { ty: 0, value: 0.0 },
                    z: KeySet { ty: 0, value: 0.0 },
                },
            },
            view_point: ViewPoint {
                aspect: 1.77778,
                fov: FieldOfView {
                    horizontal: 1,
                    value: KeySet {
                        ty: 1,
                        value: 0.93616,
                    },
                },
            },
        };
        assert_eq!(val, expected);
    }

    #[test]
    fn read_enum() {
        #[derive(Debug, PartialEq, Deserialize)]
        enum Bar {
            None,
            Foo(u32),
            Bar(u32, f32),
            Baz(String),
            Quux { foo: u32, bar: f32 },
            Foobar(Foobar),
        }
        #[derive(Debug, PartialEq, Deserialize)]
        struct Foobar {
            foo: u32,
            bar: f32,
        }
        let input = "
0=None
1.Foo=123
2.Bar=(123, 3.1415)
3.Baz=Hello World!
4.Quux.foo=123
4.Quux.bar=3.1415
5.Foobar.foo=123
5.Foobar.bar=3.1415
length=6
";
        let data: Vec<Bar> = from_str(input).unwrap();
        let expected = vec![
            Bar::None,
            Bar::Foo(123),
            Bar::Bar(123, 3.1415),
            Bar::Baz("Hello World!".to_string()),
            Bar::Quux {
                foo: 123,
                bar: 3.1415,
            },
            Bar::Foobar(Foobar {
                foo: 123,
                bar: 3.1415,
            }),
        ];
        assert_eq!(data, expected);
    }
}
