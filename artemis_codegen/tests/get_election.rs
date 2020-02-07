pub struct GetElection;
pub mod get_election {
    #![allow(dead_code)]
    pub const OPERATION_NAME: &'static str = "GetElection";
    pub const QUERY : & 'static str = "query GetElection($id: Uuid!) {\r\n    election(id: $id) {\r\n        id\r\n        name\r\n        description\r\n        choices\r\n        __typename\r\n    }\r\n}" ;
    use serde::{Deserialize, Serialize};
    #[allow(dead_code)]
    type Boolean = bool;
    #[allow(dead_code)]
    type Float = f64;
    #[allow(dead_code)]
    type Int = i64;
    #[allow(dead_code)]
    type ID = String;
    #[doc = "Uuid"]
    type Uuid = super::Uuid;
    #[derive(Deserialize)]
    #[doc = "An election"]
    pub struct GetElectionElection {
        #[doc = "The id of the election (not user facing)"]
        pub id: Uuid,
        #[doc = "The name of the election"]
        pub name: String,
        #[doc = "The description of the election"]
        pub description: String,
        #[doc = "The available choices to vote for"]
        pub choices: Vec<String>,
        #[serde(rename = "__typename")]
        pub typename: Option<String>
    }
    #[derive(Serialize)]
    pub struct Variables {
        pub id: Uuid
    }
    impl Variables {}
    #[derive(Deserialize)]
    pub struct ResponseData {
        #[doc = "Fetch an election by id"]
        pub election: Option<GetElectionElection>
    }
}
impl artemis::GraphQLQuery for GetElection {
    type Variables = get_election::Variables;
    type ResponseData = get_election::ResponseData;
    fn build_query(variables: Self::Variables) -> ::artemis::QueryBody<Self::Variables> {
        artemis::QueryBody {
            variables,
            query: get_election::QUERY,
            operation_name: get_election::OPERATION_NAME
        }
    }
}
