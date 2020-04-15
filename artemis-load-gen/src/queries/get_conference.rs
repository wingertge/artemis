#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT : & 'static str = "export namespace GetConference {\n                \n            export type Boolean = boolean;\n            export type Float = number;\n            export type Int = number;\n            export type ID = string;\n            export type String = string;\n\n            \n\n            \n\n            \n\n            \n        \n                export namespace Conference {\n                    \n        \n                export namespace Talks {\n                    \n        \n\n        /** Object to represent a Person */\n        export interface Speakers {\n            \n        \n            /**\n* Fullname of the person\n            */\n        \n        name: String\n    \n        }\n        \n                }\n                \n\n        /** Object to represent a talk */\n        export interface Talks {\n            \n        \n            /**\n* The technical id\n            */\n        \n        id: ID\n    ,\n\n        \n            /**\n* Title of the talk\n            */\n        \n        title: String\n    ,\n\n        \n            /**\n* Speakers of the talk\n            */\n        \n        speakers: Maybe<Array<Talks.Speakers>>\n    \n        }\n        \n                }\n                \n\n        /** Object to represent a conference */\n        export interface Conference {\n            \n        \n            /**\n* The technical id\n            */\n        \n        id: ID\n    ,\n\n        \n            /**\n* Name of the conference\n            */\n        \n        name: String\n    ,\n\n        \n            /**\n* City where the conference is held\n            */\n        \n        city: Maybe<String>\n    ,\n\n        \n            /**\n* Talks on the conference agenda\n            */\n        \n        talks: Maybe<Array<Conference.Talks>>\n    \n        }\n        \n\n            \n            export interface Variables {\n                id: Long\n            }\n            \n\n            export interface ResponseData {\n                \n        \n            /**\n* Find a conference based on the name\n            */\n        \n        conference: Maybe<Conference>\n    \n            }\n        \n            }" ;
#[allow(clippy::all)]
pub struct GetConference;
#[allow(clippy::all)]
pub mod get_conference {
    #![allow(dead_code)]
    pub const OPERATION_NAME: &str = "GetConference";
    pub const QUERY : & str = "query GetConference($id: Long!) {\r\n    conference(id: $id) {\r\n        id\r\n        name\r\n        city\r\n        talks {\r\n            id\r\n            title\r\n            speakers {\r\n                name\r\n            }\r\n        }\r\n    }\r\n}" ;
    use serde::{Deserialize, Serialize};
    #[allow(dead_code)]
    type Boolean = bool;
    #[allow(dead_code)]
    type Float = f64;
    #[allow(dead_code)]
    type Int = i64;
    #[allow(dead_code)]
    type ID = String;
    #[doc = "Long type"]
    type Long = crate::Long;
    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    #[doc = "Object to represent a Person"]
    pub struct GetConferenceConferenceTalksSpeakers {
        #[doc = "Fullname of the person"]
        pub name: String
    }
    impl GetConferenceConferenceTalksSpeakers {
        #[allow(unused_variables)]
        fn selection(variables: &Variables) -> Vec<::artemis::codegen::FieldSelector> {
            vec![::artemis::codegen::FieldSelector::Scalar(
                "name",
                String::new()
            )]
        }
    }
    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    #[doc = "Object to represent a talk"]
    pub struct GetConferenceConferenceTalks {
        #[doc = "The technical id"]
        pub id: ID,
        #[doc = "Title of the talk"]
        pub title: String,
        #[doc = "Speakers of the talk"]
        pub speakers: Option<Vec<GetConferenceConferenceTalksSpeakers>>
    }
    impl GetConferenceConferenceTalks {
        #[allow(unused_variables)]
        fn selection(variables: &Variables) -> Vec<::artemis::codegen::FieldSelector> {
            vec![
                ::artemis::codegen::FieldSelector::Scalar("id", String::new()),
                ::artemis::codegen::FieldSelector::Scalar("title", String::new()),
                ::artemis::codegen::FieldSelector::Object(
                    "speakers",
                    String::new(),
                    "Person",
                    GetConferenceConferenceTalksSpeakers::selection(variables)
                ),
            ]
        }
    }
    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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
    impl GetConferenceConference {
        #[allow(unused_variables)]
        fn selection(variables: &Variables) -> Vec<::artemis::codegen::FieldSelector> {
            vec![
                ::artemis::codegen::FieldSelector::Scalar("id", String::new()),
                ::artemis::codegen::FieldSelector::Scalar("name", String::new()),
                ::artemis::codegen::FieldSelector::Scalar("city", String::new()),
                ::artemis::codegen::FieldSelector::Object(
                    "talks",
                    String::new(),
                    "Talk",
                    GetConferenceConferenceTalks::selection(variables)
                ),
            ]
        }
    }
    #[derive(Clone, Debug, PartialEq, Serialize)]
    #[cfg_attr(target_arch = "wasm32", derive(Deserialize))]
    pub struct Variables {
        pub id: Long
    }
    impl Variables {}
    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    pub struct ResponseData {
        #[doc = "Find a conference based on the name"]
        pub conference: Option<GetConferenceConference>
    }
    impl ::artemis::codegen::QueryInfo<Variables> for ResponseData {
        fn selection(variables: &Variables) -> Vec<::artemis::codegen::FieldSelector> {
            vec![::artemis::codegen::FieldSelector::Object(
                "conference",
                format!("(id:{:?})", variables.id),
                "Conference",
                GetConferenceConference::selection(variables)
            )]
        }
    }
}
#[allow(clippy::all)]
impl ::artemis::GraphQLQuery for GetConference {
    type Variables = get_conference::Variables;
    type ResponseData = get_conference::ResponseData;
    fn build_query(
        variables: Self::Variables
    ) -> (
        ::artemis::QueryBody<Self::Variables>,
        ::artemis::exchange::OperationMeta
    ) {
        let meta = ::artemis::exchange::OperationMeta {
            query_key: 1354603040u32,
            operation_type: ::artemis::exchange::OperationType::Query,
            involved_types: vec!["Person", "Talk", "Conference"]
        };
        let body = ::artemis::QueryBody {
            variables,
            query: get_conference::QUERY,
            operation_name: get_conference::OPERATION_NAME
        };
        (body, meta)
    }
}
