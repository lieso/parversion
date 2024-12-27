lazy_static! {
use crate::provider::Provider;
    pub static ref XML_TRANSFORMATIONS: Vec<Transformation> = vec![
        NativeXMLElementTransformation {
            description: "XML element transformation during document preprocessing. Blacklisted elements and attributes are eliminated, unseen by the LLM, reducing token count for inference.",
            transform: |element_name, attributes| {
                if UNSEEN_BLACKLISTED_ELEMENTS::contains(element_name) {
                    return None;
                }

                
            },
        },
    ];
}
