use std::{fmt, marker::PhantomData};

use itertools::unfold;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use tokio::sync::oneshot;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PubSubResult {
    Syncing(bool),
}

/// Either value or array of values.
///
/// A value must serialize into a string.
#[derive(Default, Debug, PartialEq, Clone)]
pub struct ValueOrArray<T>(pub Vec<T>);

impl<T> From<T> for ValueOrArray<T> {
    fn from(value: T) -> Self {
        Self(vec![value])
    }
}

impl<T: Serialize> Serialize for ValueOrArray<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0.len() {
            0 => serializer.serialize_none(),
            1 => Serialize::serialize(&self.0[0], serializer),
            _ => Serialize::serialize(&self.0, serializer),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for ValueOrArray<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> de::Visitor<'de> for Visitor<T> {
            type Value = ValueOrArray<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("string value or sequence of values")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                use serde::de::IntoDeserializer;

                Deserialize::deserialize(value.into_deserializer())
                    .map(|value| ValueOrArray(vec![value]))
            }

            fn visit_seq<S>(self, visitor: S) -> Result<Self::Value, S::Error>
            where
                S: de::SeqAccess<'de>,
            {
                unfold(visitor, |vis| vis.next_element().transpose())
                    .collect::<Result<_, _>>()
                    .map(ValueOrArray)
            }
        }

        deserializer.deserialize_any(Visitor(PhantomData))
    }
}

#[derive(Debug)]
pub struct Completable<T> {
    pub command: T,
    pub completion_sender: oneshot::Sender<()>,
}
