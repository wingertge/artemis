pub struct AddConference;
pub mod add_conference {
    #![allow(dead_code)]
    pub const OPERATION_NAME: &'static str = "AddConference";
    pub const QUERY : & 'static str = "mutation AddConference($name: String!, $city: String!) {\r\n    addConference(conference: {\r\n        name: $name,\r\n        city: $city\r\n    }) {\r\n        id\r\n        name\r\n        city\r\n        talks {\r\n            id\r\n        }\r\n    }\r\n}" ;
    pub const KEY: u32 = 1806959457u32;
    pub const OPERATION_TYPE: ::artemis::OperationType = ::artemis::OperationType::Mutation;
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
    impl ::artemis::QueryInfo for AddConferenceAddConferenceTalks {
        fn typename(&self) -> &'static str {
            "Talk"
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
    impl ::artemis::QueryInfo for AddConferenceAddConference {
        fn typename(&self) -> &'static str {
            "Conference"
        }
    }
    #[derive(Clone, Debug, PartialEq, Serialize)]
    pub struct Variables {
        pub name: String,
        pub city: String
    }
    impl Variables {}
    impl ::artemis::QueryVariables for Variables {}
    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    pub struct ResponseData {
        #[doc = "Add a new conference"]
        #[serde(rename = "addConference")]
        pub add_conference: Option<AddConferenceAddConference>
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
            key: 1806959457u32,
            operation_type: ::artemis::OperationType::Mutation,
            involved_types: vec!["Talk", "Conference"]
        };
        let body = ::artemis::QueryBody {
            variables,
            query: add_conference::QUERY,
            operation_name: add_conference::OPERATION_NAME
        };
        (body, meta)
    }
}
