use artemis::{exchange::OperationMeta, GraphQLQuery, QueryBody};

pub mod writers {
    #![allow(dead_code)]
    pub const OPERATION_NAME: &str = "Writers";
    pub const QUERY: &'static str = r#"
  query {
    writers {
      id
      name
      amountOfBooks
      interests
      recognised
      number
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
    pub struct Writer {
        pub id: String,
        pub name: String,
        #[serde(rename = "amountOfBooks")]
        pub amount_of_books: Int,
        pub interests: String,
        pub recognised: bool,
        pub number: Int
    }

    impl Writer {
        #[allow(unused_variables)]
        fn selection(variables: &Variables) -> Vec<FieldSelector> {
            vec![
                FieldSelector::Scalar("id", String::new()),
                FieldSelector::Scalar("name", String::new()),
                FieldSelector::Scalar("amountOfBooks", String::new()),
                FieldSelector::Scalar("interests", String::new()),
                FieldSelector::Scalar("recognised", String::new()),
                FieldSelector::Scalar("number", String::new()),
            ]
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct ResponseData {
        pub writers: Vec<Writer>
    }

    impl QueryInfo<Variables> for ResponseData {
        fn selection(variables: &Variables) -> Vec<FieldSelector> {
            vec![FieldSelector::Object(
                "writers",
                String::new(),
                "Writer",
                Writer::selection(variables)
            )]
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct Variables;
}

pub struct Writers;

impl GraphQLQuery for Writers {
    type Variables = writers::Variables;
    type ResponseData = writers::ResponseData;

    fn build_query(_variables: Self::Variables) -> (QueryBody<Self::Variables>, OperationMeta) {
        unimplemented!()
    }
}
