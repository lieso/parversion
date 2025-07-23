use uuid::Uuid;
use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer, Visitor, Error as SerdeError};
use std::fmt;

#[derive(Clone, Debug, Hash)]
pub struct ID {
    value: String
}

impl ID {
    pub fn new() -> Self {
        ID {
            value: Uuid::new_v4().to_string()
        }
    }
    
    pub fn from_str(value: &str) -> Self {
        ID {
            value: value.to_string()
        }
    }

    #[allow(dead_code)]
    pub fn to_string(&self) -> String {
        self.value.clone()
    }
}

impl PartialEq for ID {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for ID {}

impl<'de> Deserialize<'de> for ID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IDVisitor;

        impl<'de> Visitor<'de> for IDVisitor {
            type Value = ID;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing an ID")
            }

            fn visit_str<E>(self, v: &str) -> Result<ID, E>
            where
                E: SerdeError,
            {
                Ok(ID::from_str(v))
            }
        }

        deserializer.deserialize_str(IDVisitor)
    }
}

impl Serialize for ID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.value)
    }
}
