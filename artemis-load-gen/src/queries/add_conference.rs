pub struct AddConference;
pub mod add_conference {
    #![allow(dead_code)]
    pub const OPERATION_NAME: &'static str = "AddConference";
    pub const QUERY : & 'static str = "mutation AddConference($name: String!, $city: String!) {\r\n    addConference(conference: {\r\n        name: $name,\r\n        city: $city\r\n    }) {\r\n        id\r\n        name\r\n        city\r\n        talks {\r\n            id\r\n        }\r\n    }\r\n}" ;
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
    #[doc = "Object to represent a talk"]
    pub struct AddConferenceAddConferenceTalks {
        #[doc = "The technical id"]
        pub id: ID
    }
    impl ::artemis::QueryInfo<Variables> for AddConferenceAddConferenceTalks {
        fn typename(&self) -> &'static str {
            "Talk"
        }
        #[allow(unused_variables)]
        fn selection(variables: &Variables) -> Vec<::artemis::FieldSelector> {
            vec![::artemis::FieldSelector::Scalar(
                String::from("id"),
                String::new()
            )]
        }
    }
    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
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
    impl ::artemis::QueryInfo<Variables> for AddConferenceAddConference {
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
                    AddConferenceAddConferenceTalks::selection(variables)
                ),
            ]
        }
    }
    #[derive(Clone, Debug, PartialEq, Serialize)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
    pub struct Variables {
        pub name: String,
        pub city: String
    }
    impl Variables {}
    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
    pub struct ResponseData {
        #[doc = "Add a new conference"]
        #[serde(rename = "addConference")]
        pub add_conference: Option<AddConferenceAddConference>
    }
    impl ::artemis::QueryInfo<Variables> for ResponseData {
        fn typename(&self) -> &'static str {
            "Mutation"
        }
        fn selection(variables: &Variables) -> Vec<::artemis::FieldSelector> {
            vec![::artemis::FieldSelector::Object(
                String::from("addConference"),
                String::from(format!(
                    "({{city:{:?},name:{:?}}})",
                    &variables.city, &variables.name
                )),
                true,
                AddConferenceAddConference::selection(variables)
            )]
        }
    }
}
impl ::artemis::GraphQLQuery for AddConference {
    type Variables = add_conference::Variables;
    type ResponseData = add_conference::ResponseData;
    fn build_query(
        variables: Self::Variables
    ) -> (
        ::artemis::QueryBody<Self::Variables>,
        ::artemis::OperationMeta
    ) {
        let meta = ::artemis::OperationMeta {
            key: ::artemis::progressive_hash(6162058983754695521u64, &variables),
            operation_type: ::artemis::OperationType::Mutation,
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
