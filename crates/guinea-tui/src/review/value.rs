//! Ordered generic Serde value for read-only data review. Retains non-finite
//! floats and complex YAML mapping keys without depending on libyaml.
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{MapAccess, SeqAccess, Visitor},
    ser::{SerializeMap, SerializeSeq},
};

pub(super) enum Value {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    Text(String),
    Sequence(Vec<Value>),
    Mapping(Vec<(Value, Value)>),
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(v) => serializer.serialize_bool(*v),
            Self::I64(v) => serializer.serialize_i64(*v),
            Self::U64(v) => serializer.serialize_u64(*v),
            Self::F64(v) => serializer.serialize_f64(*v),
            Self::Text(v) => serializer.serialize_str(v),
            Self::Sequence(values) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    seq.serialize_element(value)?;
                }
                seq.end()
            }
            Self::Mapping(values) => {
                let mut map = serializer.serialize_map(Some(values.len()))?;
                for (key, value) in values {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor;
        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a data value")
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }
            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Value, E> {
                Ok(Value::Bool(value))
            }
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Value, E> {
                Ok(Value::I64(value))
            }
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Value, E> {
                Ok(Value::U64(value))
            }
            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Value, E> {
                Ok(Value::F64(value))
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Value, E> {
                Ok(Value::Text(value.to_owned()))
            }
            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Value, E> {
                Ok(Value::Text(value))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element()? {
                    values.push(value);
                }
                Ok(Value::Sequence(values))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
                let mut values = Vec::new();
                while let Some(pair) = map.next_entry()? {
                    values.push(pair);
                }
                Ok(Value::Mapping(values))
            }
        }
        deserializer.deserialize_any(ValueVisitor)
    }
}
