#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT : & 'static str = "export namespace AddConference {\n                \n            export type Boolean = boolean;\n            export type Float = number;\n            export type Int = number;\n            export type ID = string;\n            export type String = string;\n\n            \n\n            \n\n            \n\n            \n        \n                export namespace AddConference {\n                    \n        \n\n        /** Object to represent a talk */\n        export interface Talks {\n            \n        \n            /**\n* The technical id\n            */\n        \n        id: ID\n    \n        }\n        \n                }\n                \n\n        /** Object to represent a conference */\n        export interface AddConference {\n            \n        \n            /**\n* The technical id\n            */\n        \n        id: ID\n    ,\n\n        \n            /**\n* Name of the conference\n            */\n        \n        name: String\n    ,\n\n        \n            /**\n* City where the conference is held\n            */\n        \n        city: Maybe<String>\n    ,\n\n        \n            /**\n* Talks on the conference agenda\n            */\n        \n        talks: Maybe<Array<AddConference.Talks>>\n    \n        }\n        \n\n            \n            export interface Variables {\n                name: String,\ncity: String\n            }\n            \n\n            export interface ResponseData {\n                \n        \n            /**\n* Add a new conference\n            */\n        \n        addConference: Maybe<AddConference>\n    \n            }\n        \n            }" ;
#[allow(clippy::all)]
pub struct AddConference;
#[allow(clippy::all)]
pub mod add_conference {
    #![allow(dead_code)]
    pub const OPERATION_NAME: &str = "AddConference";
    pub const QUERY : & str = "mutation AddConference($name: String!, $city: String!) {\r\n    addConference(conference: {\r\n        name: $name,\r\n        city: $city\r\n    }) {\r\n        id\r\n        name\r\n        city\r\n        talks {\r\n            id\r\n        }\r\n    }\r\n}" ;
    use serde::{Deserialize, Serialize};
    #[allow(dead_code)]
    type Boolean = bool;
    #[allow(dead_code)]
    type Float = f64;
    #[allow(dead_code)]
    type Int = i64;
    #[allow(dead_code)]
    type ID = String;
    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    #[doc = "Object to represent a talk"]
    pub struct AddConferenceAddConferenceTalks {
        #[doc = "The technical id"]
        pub id: ID
    }
    impl AddConferenceAddConferenceTalks {
        #[allow(unused_variables)]
        fn selection(variables: &Variables) -> Vec<::artemis::codegen::FieldSelector> {
            vec![::artemis::codegen::FieldSelector::Scalar(
                "id",
                String::new()
            )]
        }
    }
    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    #[doc = "Object to represent a conference"]
    pub struct AddConferenceAddConference {
        #[doc = "The technical id"]
        pub id: ID,
        #[doc = "Name of the conference"]
        pub name: String,
        #[doc = "City where the conference is held"]
        pub city: Option<String>,
        #[doc = "Talks on the conference agenda"]
        pub talks: Option<Vec<AddConferenceAddConferenceTalks>>
    }
    impl AddConferenceAddConference {
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
                    AddConferenceAddConferenceTalks::selection(variables)
                ),
            ]
        }
    }
    #[derive(Clone, Debug, PartialEq, Serialize)]
    #[cfg_attr(target_arch = "wasm32", derive(Deserialize))]
    pub struct Variables {
        pub name: String,
        pub city: String
    }
    impl Variables {}
    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    pub struct ResponseData {
        #[doc = "Add a new conference"]
        #[serde(rename = "addConference")]
        pub add_conference: Option<AddConferenceAddConference>
    }
    impl ::artemis::codegen::QueryInfo<Variables> for ResponseData {
        fn selection(variables: &Variables) -> Vec<::artemis::codegen::FieldSelector> {
            vec![::artemis::codegen::FieldSelector::Object(
                "addConference",
                format!("({{city:{:?},name:{:?}}})", variables.city, variables.name),
                "Conference",
                AddConferenceAddConference::selection(variables)
            )]
        }
    }
}
#[allow(clippy::all)]
impl ::artemis::GraphQLQuery for AddConference {
    type Variables = add_conference::Variables;
    type ResponseData = add_conference::ResponseData;
    fn build_query(
        variables: Self::Variables
    ) -> (
        ::artemis::QueryBody<Self::Variables>,
        ::artemis::exchange::OperationMeta
    ) {
        let meta = ::artemis::exchange::OperationMeta {
            query_key: 1806959457u32,
            operation_type: ::artemis::exchange::OperationType::Mutation,
            involved_types: vec!["Conference", "Talk"]
        };
        let body = ::artemis::QueryBody {
            variables,
            query: add_conference::QUERY,
            operation_name: add_conference::OPERATION_NAME
        };
        (body, meta)
    }
}
