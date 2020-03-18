pub struct GetConference;
pub mod get_conference {
    #![allow(dead_code)]
    pub const OPERATION_NAME: &'static str = "GetConference";
    pub const QUERY : & 'static str = "query GetConference($id: Long!) {\r\n    conference(id: $id) {\r\n        id\r\n        name\r\n        city\r\n        talks {\r\n            id\r\n            title\r\n            speakers {\r\n                name\r\n            }\r\n        }\r\n    }\r\n}" ;
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
    #[doc = "Object to represent a Person"]
    pub struct GetConferenceConferenceTalksSpeakers {
        #[doc = "Fullname of the person"]
        pub name: String
    }
    impl ::artemis::QueryInfo<Variables> for GetConferenceConferenceTalksSpeakers {
        fn typename(&self) -> &'static str {
            "Person"
        }
        #[allow(unused_variables)]
        fn selection(variables: &Variables) -> Vec<::artemis::FieldSelector> {
            vec![::artemis::FieldSelector::Scalar(
                String::from("name"),
                String::new()
            )]
        }
    }
    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
    #[doc = "Object to represent a talk"]
    pub struct GetConferenceConferenceTalks {
        #[doc = "The technical id"]
        pub id: ID,
        #[doc = "Title of the talk"]
        pub title: String,
        #[doc = "Speakers of the talk"]
        pub speakers: Option<Vec<GetConferenceConferenceTalksSpeakers>>
    }
    impl ::artemis::QueryInfo<Variables> for GetConferenceConferenceTalks {
        fn typename(&self) -> &'static str {
            "Talk"
        }
        #[allow(unused_variables)]
        fn selection(variables: &Variables) -> Vec<::artemis::FieldSelector> {
            vec![
                ::artemis::FieldSelector::Scalar(String::from("id"), String::new()),
                ::artemis::FieldSelector::Scalar(String::from("title"), String::new()),
                ::artemis::FieldSelector::Object(
                    String::from("speakers"),
                    String::new(),
                    false,
                    GetConferenceConferenceTalksSpeakers::selection(variables)
                ),
            ]
        }
    }
    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
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
    impl ::artemis::QueryInfo<Variables> for GetConferenceConference {
        fn typename(&self) -> &'static str {
            "Conference"
        }
        #[allow(unused_variables)]
        fn selection(variables: &Variables) -> Vec<::artemis::FieldSelector> {
            vec![
                ::artemis::FieldSelector::Scalar(String::from("id"), String::new()),
                ::artemis::FieldSelector::Scalar(String::from("name"), String::new()),
                ::artemis::FieldSelector::Scalar(String::from("city"), String::new()),
                ::artemis::FieldSelector::Object(
                    String::from("talks"),
                    String::new(),
                    false,
                    GetConferenceConferenceTalks::selection(variables)
                ),
            ]
        }
    }
    #[derive(Clone, Debug, PartialEq, Serialize)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
    pub struct Variables {
        pub id: Long
    }
    impl Variables {}
    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
    pub struct ResponseData {
        #[doc = "Find a conference based on the name"]
        pub conference: Option<GetConferenceConference>
    }
    impl ::artemis::QueryInfo<Variables> for ResponseData {
        fn typename(&self) -> &'static str {
            "Query"
        }
        fn selection(variables: &Variables) -> Vec<::artemis::FieldSelector> {
            vec![::artemis::FieldSelector::Object(
                String::from("conference"),
                String::from(format!("(id:{:?})", &variables.id)),
                true,
                GetConferenceConference::selection(variables)
            )]
        }
    }
}
impl ::artemis::GraphQLQuery for GetConference {
    type Variables = get_conference::Variables;
    type ResponseData = get_conference::ResponseData;
    fn build_query(
        variables: Self::Variables
    ) -> (
        ::artemis::QueryBody<Self::Variables>,
        ::artemis::OperationMeta
    ) {
        let meta = ::artemis::OperationMeta {
            query_key: ::artemis::progressive_hash(8181565099941403168u64, &variables),
            operation_type: ::artemis::OperationType::Query,
            involved_types: vec!["Talk", "Conference", "Person"]
        };
        let body = ::artemis::QueryBody {
            variables,
            query: get_conference::QUERY,
            operation_name: get_conference::OPERATION_NAME
        };
        (body, meta)
    }
}
