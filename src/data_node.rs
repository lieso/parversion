use crate::hash::{Hash};
use crate::id::{ID};
use crate::transformations::{
    DataNodeFieldsTransform,
    DataNodeHashTransform,
    transform
};

pub type DataNodeFields = HashSet<String>;

pub struct DataNode {
    id: ID,
    context_id: ID,
    hash: Hash,
    fields: DataNodeFields,
}

pub fn get_fields_transform() -> DataNodeFieldsTransform {
    FIELD_TRANSFORMATIONS.first().clone().unwrap()
}

pub fn apply_fields_transform(
    transformation: DataNodeFieldTransform,
    fields: DataNodeFields
) -> DataNodeFields {
    let serialized = serde_json::to_string(fields).expect("Could not serialize fields");
    transform(transformation, serialized)
}

pub fn get_hash_transform() -> DataNodeHashTransform {
    HASH_TRANSFORMATIONS.first().clone().unwrap()
}

pub fn apply_hash_transform(
    transformation: DataNodeHashTransform,
    fields: DataNodeFields
) -> Hash {

}

impl DataNode {
    pub fn new(
        context_id: ID,
        fields: DataNodeFields
        description: String,
    ) -> Self {
        let transform = get_hash_transform();
        let hash: Hash = apply_hash_transform(transform, fields.clone());

        let transform = get_fields_transform();
        let fields: DataNodeFields = apply_fields_transform(transform, fields.clone());

        DataNode {
            id: ID::new(),
            hash,
            description,
            context_id,
            fields,
        }
    }

    pub fn get_hash(&self) -> Hash {
        self.hash.clone()
    }
}

lazy_static! {
    pub static ref HASH_TRANSFORMATIONS: Vec<DataNodeHashTransform> = vec![
        XmlHashTransformation {
            runtime: Runtime::AWK,
            description: String::from("Hashing XML elements"),
            regex: Regex::new(r#"
            "#).unwrap(),
            expression: String::from(r#"{ print $0 }"#),
        }
    ];
}
lazy_static! {
    pub static ref FIELD_TRANSFORMATIONS: Vec<DataNodeFieldTransform> = vec![
        XmlHashTransformation {
            runtime: Runtime::AWK,
            description: String::from("Hashing XML elements"),
            regex: Regex::new(r#"
            "#).unwrap(),
            expression: String::from(r#"{ print $0 }"#),
        }
    ];
}
