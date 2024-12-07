use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InterfaceType {
    pub id: String,
    pub name: String,
    pub description: String,
    pub has_recursive: bool,
    pub json_schema: Option<String>,
}

// Content aggregator
// Article
// Discussion
// Video
// Social Media feed
// Job listing

lazy_static! {
    pub static ref INTERFACE_TYPES: Vec<InterfaceType> = vec![
        InterfaceType {
            id: String::from("7bba3bdf-3343-4f71-a0c9-c24a076dc7e8"),
            name: String::from("content_aggregator"),
            description: String::from("Content aggregators are web platforms that curate and compile information from various sources, presenting it in a single, convenient location for users to easily access and explore. These platforms do not typically produce original content themselves but instead collate articles, news stories, blog posts, and other digital media from across the internet. Examples of popular content aggregators include Reddit, where users submit and vote on links, creating dynamic discussions and community-driven content relevance; Hacker News, which features a constantly updated mix of significant tech and startup industry news curated by user submissions; and Google Search Results, which aggregate webpages, images, videos, and other types of content based on user queries, offering a broad spectrum of the most relevant and authoritative sources available online. Content aggregators serve as valuable tools for staying informed by allowing users to discover content aligned with their interests, preferences, or professional needs efficiently."),
            has_recursive: false,
            json_schema: Some(String::from(r#"
            {
               "$schema": "http://json-schema.org/draft-07/schema#",
               "title": "Content Aggregator",
               "type": "object",
               "properties": {
                 "entries": {
                   "type": "array",
                   "description": "A list of content entries aggregated by the application.",
                   "items": {
                     "type": "object",
                     "properties": {
                       "title": {
                         "type": "string",
                         "description": "The main title of each entry, typically displayed prominently."
                       },
                       "url": {
                         "type": "string",
                         "description": "The URL directing to the original content."
                       },
                       "score": {
                         "type": "string",
                         "description": "The popularity score of the entry, reflecting its user engagement."
                       },
                       "submitted": {
                         "type": "string",
                         "description": "The timestamp indicating when the entry was submitted."
                       }
                     },
                     "required": ["title", "url", "submitted"]
                   }
                 }
               },
               "required": ["entries"]
             }
            "#)),
        },
        InterfaceType {
            id: String::from("990fdb11-7b94-40b7-b982-d657b2290327"),
            name: String::from("article"),
            description: String::from("Articles are structured pieces of writing that aim to convey information, analysis, or opinions on a wide array of topics. They are designed to inform, educate, or entertain audiences across different platforms, including digital and print media. Articles vary in style, length, and complexity, and can take the form of informative essays, opinion pieces, analytical reports, or narrative stories. They often include headlines to capture attention, introductions to set the context, and body paragraphs to elaborate on the main ideas or arguments. Articles serve diverse purposes, from providing in-depth explorations of scientific discoveries to offering commentary on cultural trends, thereby playing a crucial role in disseminating knowledge and stimulating thought across varied audiences."),
            has_recursive: false,
            json_schema: Some(String::from(r#"
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
            "#)),
        },
        InterfaceType {
            id: String::from("5cbf66af-ddf2-4c6d-ab61-cd8cbd65d38d"),
            name: String::from("discussion"),
            description: String::from("Discussion pages are interactive platforms designed to facilitate conversation and exchange of ideas among users. These pages allow participants to post comments, ask questions, and share opinions on specific topics or threads, fostering community engagement and dialogue. Typically organized in a chronological or threaded format, discussion pages enable users to reply directly to specific comments, creating a structured conversation that can branch into various sub-discussions. Examples include comment sections on Reddit and Hacker News, where users engage in discussions related to user-submitted content, as well as communication tools like Slack and Microsoft Teams, which allow real-time conversations among teams or groups in a professional setting. These pages are instrumental in building communities, sharing knowledge, and collaborating by providing a space for diverse voices to be heard and ideas to be exchanged."),
            has_recursive: true,
            json_schema: Some(String::from(r#"
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
            "#)),
        },
        InterfaceType {
            id: String::from("7b102fa6-e79d-42b1-956a-5d2d67371258"),
            name: String::from("video"),
            description: String::from("Video pages are digital platforms that host and display video content, allowing users to view, interact with, and share multimedia presentations. These pages typically feature an embedded video player that provides controls for playing, pausing, and navigating through the video content. Video pages often include additional information such as titles, descriptions, and timestamps to give viewers a clear understanding of the video's content and context. Interactive elements like comments sections, like/dislike buttons, and sharing options encourage user engagement, enabling viewers to participate in discussions or share their thoughts and feedback. Platforms like YouTube serve as prominent examples of video pages, offering a vast array of content ranging from educational tutorials and entertainment clips to personal vlogs and live streams. These pages are essential for creators and audiences alike, providing a dynamic medium for storytelling, informationdissemination, and community building."),
            has_recursive: false,
            json_schema: Some(String::from(r#"
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
            "#)),
        },
        InterfaceType {
            id: String::from("7b8b6222-ed3c-4aa3-9478-53392190b7a3"),
            name: String::from("social_media_feed"),
            description: String::from("Social media feeds are dynamic, continuously updated streams of content that allow users to view and engage with posts, updates, and interactions from their network of friends, followers, or subscribed accounts. These feeds aggregate various content types, such as text updates, images, videos, links, and advertisements, presenting them in a chronological or algorithmically prioritized order. Platforms like Facebook and Twitter exemplify social media feeds by offering a personalized experience where users can interact through likes, comments, shares, and retweets, facilitating connections and conversations across diverse communities. Social media feeds serve as real-time windows into the activities and thoughts of one's social circle, as well as global events and trending topics, making them essential tools for staying informed, entertained, and connected with the world."),
            has_recursive: false,
            json_schema: Some(String::from(r#"
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
            "#)),
        },
        InterfaceType {
            id: String::from("fb9477d3-ef77-49d9-81f4-6b20042667c8"),
            name: String::from("job_listing"),
            description: String::from("Job listing pages are specialized web pages that aggregate and display employment opportunities from various industries, providing a centralized platform for job seekers to explore potential career paths. These pages typically feature a searchable database of job openings, allowing users to filter listings by criteria such as location, industry, job title, and experience level. Each job listing generally includes crucial details like the position's title, description, required qualifications, company name, and application instructions, giving candidates a clear understanding of the opportunity and how to apply.Job listing pages serve as valuable resources for both employers looking to fill vacancies and job seekers aiming to find suitable employment, streamlining the job search process byconnecting candidates with prospective employers efficiently."),
            has_recursive: false,
            json_schema: Some(String::from(r#"
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
            "#)),
        },
    ];
}
