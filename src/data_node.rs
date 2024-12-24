use lazy_static::lazy_static;

use crate::prelude::*;
use crate::transformations::{
    DataNodeFieldsTransform,
    DataNodeHashTransform,
    transform
};

enum DataNodeTransform {
    A(DataNodeFieldsTransform),
    B(DataNodeHashTransform),
}
pub type DataNodeFields = HashMap<String, String>;

pub struct DataNode {
    pub id: ID,
    pub context_id: ID,
    pub hash: Hash,
    pub lineage: Lineage,
    pub fields: DataNodeFields,
}

pub fn get_fields_transform() -> DataNodeFieldsTransform {
    unimplemented!()
    //FIELD_TRANSFORMATIONS.first().clone().unwrap()
}

pub fn get_hash_transform() -> DataNodeHashTransform {
    unimplemented!()
    //HASH_TRANSFORMATIONS.first().clone().unwrap()
}

pub fn apply_fields_transform(
    transformation: DataNodeTransform,
    fields: DataNodeFields
) -> DataNodeFields {
    transform(transformation, fields)
}

pub fn apply_hash_transform(
    transformation: DataNodeTransform,
    fields: DataNodeFields
) -> Hash {
    let hash_string: &str = transform(transformation, fields);
    Hash::from_str(hash_string)
}

impl DataNode {
    pub fn new(
        context_id: ID,
        fields: DataNodeFields
        description: String,
        parent_lineage: &Lineage,
    ) -> Self {
        let hash: Hash = apply_hash_transform(get_hash_transform(), fields.clone());


        // we'll need original set of fields if we want to determine which other data node is recursively related to the current one
        let fields: DataNodeFields = apply_fields_transform(get_fields_transform(), fields.clone());
        let lineage = parent_lineage.with_item(hash.clone());

        DataNode {
            id: ID::new(),
            hash,
            context_id,
            fields,
            lineage,
        }
    }

    pub fn get_hash(&self) -> Hash {
        self.hash.clone()
    }
}

//lazy_static! {
//    pub static ref HASH_TRANSFORMATIONS: Vec<DataNodeHashTransform> = vec![
//        XmlHashTransformation {
//            runtime: Runtime::AWK,
//            description: String::from("Hashing XML elements"),
//            regex: Regex::new(r#"
//            "#).unwrap(),
//            expression: String::from(r#"{ print $0 }"#),
//        }
//    ];
//}
//
//lazy_static! {
//    pub static ref FIELD_TRANSFORMATIONS: Vec<DataNodeFieldTransform> = vec![
//        XmlHashTransformation {
//            runtime: Runtime::AWK,
//            description: String::from("Hashing XML elements"),
//            regex: Regex::new(r#"
//            "#).unwrap(),
//            expression: String::from(r#"{ print $0 }"#),
//        }
//    ];
//}
