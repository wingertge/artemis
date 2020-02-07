pub struct GetConference;
pub mod get_conference {
    #![allow(dead_code)]
    pub const OPERATION_NAME: &'static str = "GetConference";
    pub const QUERY : & 'static str = "query GetConference {\r\n    conference(id: \"1\") {\r\n        id\r\n        name\r\n        city\r\n        talks {\r\n            id\r\n            title\r\n            speakers {\r\n                name\r\n            }\r\n        }\r\n    }\r\n}" ;
    use serde::{Deserialize, Serialize};
    #[allow(dead_code)]
    type Boolean = bool;
    #[allow(dead_code)]
    type Float = f64;
    #[allow(dead_code)]
    type Int = i64;
    #[allow(dead_code)]
    type ID = String;
    #[derive(Deserialize)]
    #[doc = "Object to represent a Person"]
    pub struct GetConferenceConferenceTalksSpeakers {
        #[doc = "Fullname of the person"]
        pub name: String
    }
    #[derive(Deserialize)]
    #[doc = "Object to represent a talk"]
    pub struct GetConferenceConferenceTalks {
        #[doc = "The technical id"]
        pub id: ID,
        #[doc = "Title of the talk"]
        pub title: String,
        #[doc = "Speakers of the talk"]
        pub speakers: Option<Vec<GetConferenceConferenceTalksSpeakers>>
    }
    #[derive(Deserialize)]
    #[doc = "Object to represent a conference"]
    pub struct GetConferenceConference {
        #[doc = "The technical id"]
        pub id: ID,
        #[doc = "Name of the conference"]
        pub name: String,
        #[doc = "City where the conference is held"]
        pub city: Option<String>,
        #[doc = "Talks on the conference agenda"]
        pub talks: Option<Vec<GetConferenceConferenceTalks>>
    }
    #[derive(Serialize)]
    pub struct Variables;
    #[derive(Deserialize)]
    pub struct ResponseData {
        #[doc = "Find a conference based on the name"]
        pub conference: Option<GetConferenceConference>
    }
}
impl artemis::GraphQLQuery for GetConference {
    type Variables = get_conference::Variables;
    type ResponseData = get_conference::ResponseData;
    fn build_query(variables: Self::Variables) -> ::artemis::QueryBody<Self::Variables> {
        artemis::QueryBody {
            variables,
            query: get_conference::QUERY,
            operation_name: get_conference::OPERATION_NAME
        }
    }
}
