use artemis::{exchange::OperationMeta, GraphQLQuery, QueryBody};

pub mod todos_query {
    #![allow(dead_code)]
    pub const OPERATION_NAME: &str = "AddConference";
    pub const QUERY: &'static str = r#"
    todos {
      id
      text
      complete
      due
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
    pub struct Todo {
        pub id: String,
        pub text: String,
        pub complete: bool,
        pub due: Date
    }

    impl Todo {
        #[allow(unused_variables)]
        fn selection(variables: &Variables) -> Vec<FieldSelector> {
            vec![
                FieldSelector::Scalar("id", String::new()),
                FieldSelector::Scalar("text", String::new()),
                FieldSelector::Scalar("complete", String::new()),
                FieldSelector::Scalar("due", String::new()),
            ]
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct ResponseData {
        pub todos: Vec<Todo>
    }

    impl QueryInfo<Variables> for ResponseData {
        fn selection(variables: &Variables) -> Vec<FieldSelector> {
            vec![FieldSelector::Object(
                "todos",
                String::new(),
                "Todo",
                Todo::selection(variables)
            )]
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct Variables;
}

pub struct TodosQuery;

impl GraphQLQuery for TodosQuery {
    type Variables = todos_query::Variables;
    type ResponseData = todos_query::ResponseData;

    fn build_query(_variables: Self::Variables) -> (QueryBody<Self::Variables>, OperationMeta) {
        unimplemented!()
    }
}
