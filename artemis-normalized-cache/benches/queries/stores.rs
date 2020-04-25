use artemis::{exchange::OperationMeta, GraphQLQuery, QueryBody};

pub mod stores {
    #![allow(dead_code)]
    pub const OPERATION_NAME: &str = "Stores";
    pub const QUERY: &'static str = r#"
  query {
    stores {
      id
      country
      started
      name
    }
  }"#;
    use artemis::codegen::{FieldSelector, QueryInfo};
    use serde::{Deserialize, Serialize};

    #[allow(dead_code)]
    type Boolean = bool;
    #[allow(dead_code)]
    type Float = f64;
    #[allow(dead_code)]
    type Int = i64;
    #[allow(dead_code)]
    type ID = String;
    #[allow(dead_code)]
    type Date = String;

    #[derive(Clone, Serialize, Deserialize)]
    pub struct Store {
        pub id: String,
        pub name: String,
        pub started: String,
        pub country: String
    }

    impl Store {
        #[allow(unused_variables)]
        fn selection(variables: &Variables) -> Vec<FieldSelector> {
            vec![
                FieldSelector::Scalar("id", String::new()),
                FieldSelector::Scalar("name", String::new()),
                FieldSelector::Scalar("started", String::new()),
                FieldSelector::Scalar("country", String::new()),
            ]
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct ResponseData {
        pub stores: Vec<Store>
    }

    impl QueryInfo<Variables> for ResponseData {
        fn selection(variables: &Variables) -> Vec<FieldSelector> {
            vec![FieldSelector::Object(
                "stores",
                String::new(),
                "Store",
                Store::selection(variables)
            )]
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct Variables;
}

pub struct Stores;

impl GraphQLQuery for Stores {
    type Variables = stores::Variables;
    type ResponseData = stores::ResponseData;

    fn build_query(_variables: Self::Variables) -> (QueryBody<Self::Variables>, OperationMeta) {
        unimplemented!()
    }
}
