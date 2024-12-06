use lazy_static::lazy_static;

pub struct PageType {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub json_schema: String,
}

/// Content aggregator
/// Article
/// Discussion
/// Video
/// Social Media feed
/// Job listing

lazy_static! {
    pub static ref PAGE_TYPES: Vec<PageType> = vec![
        PageType {
            id: 1,
            name: String::from("content_aggregator"),
            description: String::from("Links to related content"),
            json_schema: String::from(r#"
            {
                "$schema": "http://json-schema.org/draft-07/schema#",
                "title": "Page",
                "type": "object",
                "properties": {
                    "submissions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": { "type": "string" },
                                "title": { "type": "string" },
                                "url": { "type": "string" }
                            },
                            "required": ["id", "label"]
                        }
                    }
                },
                "required": ["submissions"]
            }
            "#),
        },
        PageType {
            id: 2,
            name: String::from("article"),
            description: String::from("Links to related content"),
            json_schema: String::from(r#"
            {
                "$schema": "http://json-schema.org/draft-07/schema#",
                "title": "Page",
                "type": "object",
                "properties": {
                    "submissions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": { "type": "string" },
                                "title": { "type": "string" },
                                "url": { "type": "string" }
                            },
                            "required": ["id", "label"]
                        }
                    }
                },
                "required": ["submissions"]
            }
            "#),
        },
        PageType {
            id: 3,
            name: String::from("discussion"),
            description: String::from("Links to related content"),
            json_schema: String::from(r#"
            {
                "$schema": "http://json-schema.org/draft-07/schema#",
                "title": "Page",
                "type": "object",
                "properties": {
                    "submissions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": { "type": "string" },
                                "title": { "type": "string" },
                                "url": { "type": "string" }
                            },
                            "required": ["id", "label"]
                        }
                    }
                },
                "required": ["submissions"]
            }
            "#),
        },
        PageType {
            id: 4,
            name: String::from("video"),
            description: String::from("Links to related content"),
            json_schema: String::from(r#"
            {
                "$schema": "http://json-schema.org/draft-07/schema#",
                "title": "Page",
                "type": "object",
                "properties": {
                    "submissions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": { "type": "string" },
                                "title": { "type": "string" },
                                "url": { "type": "string" }
                            },
                            "required": ["id", "label"]
                        }
                    }
                },
                "required": ["submissions"]
            }
            "#),
        },
        PageType {
            id: 5,
            name: String::from("social_media_feed"),
            description: String::from("Links to related content"),
            json_schema: String::from(r#"
            {
                "$schema": "http://json-schema.org/draft-07/schema#",
                "title": "Page",
                "type": "object",
                "properties": {
                    "submissions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": { "type": "string" },
                                "title": { "type": "string" },
                                "url": { "type": "string" }
                            },
                            "required": ["id", "label"]
                        }
                    }
                },
                "required": ["submissions"]
            }
            "#),
        },
        PageType {
            id: 6,
            name: String::from("job_listing"),
            description: String::from("Links to related content"),
            json_schema: String::from(r#"
            {
                "$schema": "http://json-schema.org/draft-07/schema#",
                "title": "Page",
                "type": "object",
                "properties": {
                    "submissions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": { "type": "string" },
                                "title": { "type": "string" },
                                "url": { "type": "string" }
                            },
                            "required": ["id", "label"]
                        }
                    }
                },
                "required": ["submissions"]
            }
            "#),
        },
    ];
}
