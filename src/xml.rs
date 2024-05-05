use serde::{Serialize, Deserialize, Serializer, Deserializer};
use xmltree::{Element, XMLNode};

#[derive(Clone, Debug)]
pub struct Xml {
    element: Option<Element>,
    text: Option<String>,
}

impl Serialize for Xml {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(element) = self.element {
            serializer.serialize_str(&element_to_string(&element))
        } else if let Some(text) = self.text {
            serializer.serialize_str(&text)
        } else {
            serializer.serialize_str("")
        }
    }
}

impl Xml {
    pub fn parse(cursor: std::io::Cursor) -> Xml {
        let element = Element::parse(&mut reader).expect("Could not parse XML");

        Xml {
            element: Some(element),
            text: None,
        }
    }
}

fn element_to_string(element: &Element) -> String {
    let mut opening_tag = format!("<{}", element.name);

    for (attr_key, attr_value) in element.attributes.iter() {
        opening_tag.push_str(&format!(" {}=\"{}\"", attr_key, attr_value));
    }

    opening_tag.push('>');

    let closing_tag = format!("</{}>", element.name);

    format!("{}{}", opening_tag, closing_tag)
}

