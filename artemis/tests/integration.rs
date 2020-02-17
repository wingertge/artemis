use artemis::{Client, ClientBuilder, Exchange, ResultSource};

const URL: &str = "http://localhost:8080/graphql";

fn build_client() -> Client<impl Exchange> {
    let builder = ClientBuilder::new(URL).with_default_exchanges();

    builder.build()
}

#[tokio::test]
async fn test_query() {
    let client = build_client();

    use artemis_test::get_conference::{get_conference::*, GetConference};
    let variables = Variables {
        id: "1".to_string()
    };
    let result = client.query(GetConference, variables).await;

    assert!(result.is_ok(), "Query returned an error");

    let response = result.unwrap();
    assert!(response.errors.is_none(), "Query returned errors");
    assert!(response.data.is_some(), "Query didn't return any data");
    let data = response.data.unwrap().conference;
    assert!(data.is_some(), "Conference was not set");
    let conf = data.unwrap();
    assert_eq!(conf.id, "1", "Returned the wrong conference");
    assert_eq!(
        conf.name, "Nextbuild 2018",
        "Returned the wrong conference name"
    );
    assert!(conf.city.is_some(), "Missing city from conference");
    assert_eq!(conf.city.unwrap(), "Eindhoven", "Returned wrong city");
    assert!(conf.talks.is_some(), "Missing talks from conference");
    let mut talks = conf.talks.unwrap();
    assert_eq!(talks.len(), 1, "Length of talks isn't 1");
    let talk = talks.pop().unwrap();
    assert_eq!(talk.id, "22", "Returned wrong talk ID");
    assert_eq!(
        talk.title, "Software Architecture for Developers",
        "Returned wrong talk title"
    );
    assert!(talk.speakers.is_some(), "Speakers missing from talk");
    let mut speakers = talk.speakers.unwrap();
    assert_eq!(speakers.len(), 1, "Speakers list isn't of length 1");
    let speaker = speakers.pop().unwrap();
    assert_eq!(speaker.name, "Simon", "Returned wrong speaker name");
}

#[tokio::test]
async fn test_cache() {
    let client = build_client();

    use artemis_test::get_conference::{get_conference::*, GetConference};
    let variables = Variables {
        id: "1".to_string()
    };

    // NOT CACHED
    let result = client.query(GetConference, variables.clone()).await;

    assert!(result.is_ok(), "Query returned an error");

    let response = result.unwrap();
    assert!(response.errors.is_none(), "Query returned errors");
    assert!(response.data.is_some(), "Query didn't return any data");
    let data = response.data.unwrap().conference;
    assert!(data.is_some(), "Conference was not set");
    let conf = data.unwrap();
    assert_eq!(conf.id, "1", "Returned the wrong conference");
    assert_eq!(
        conf.name, "Nextbuild 2018",
        "Returned the wrong conference name"
    );
    assert_eq!(
        response.debug_info.unwrap().source,
        ResultSource::Network,
        "Response didn't come from the server"
    );

    let result = client.query(GetConference, variables.clone()).await;

    assert!(result.is_ok(), "Query returned an error");

    // CACHED
    let response = result.unwrap();
    assert!(response.errors.is_none(), "Query returned errors");
    assert!(response.data.is_some(), "Query didn't return any data");
    let data = response.data.unwrap().conference;
    assert!(data.is_some(), "Conference was not set");
    let conf = data.unwrap();
    assert_eq!(conf.id, "1", "Returned the wrong conference");
    assert_eq!(
        conf.name, "Nextbuild 2018",
        "Returned the wrong conference name"
    );
    assert_eq!(
        response.debug_info.unwrap().source,
        ResultSource::Cache,
        "Response didn't come from the cache"
    );
}

#[tokio::test]
async fn test_cache_invalidation() {
    let client = build_client();

    use artemis_test::get_conference::{get_conference::*, GetConference};
    let variables = Variables {
        id: "1".to_string()
    };

    // NOT CACHED
    let result = client.query(GetConference, variables.clone()).await;

    assert!(result.is_ok(), "Query returned an error");

    let response = result.unwrap();
    assert!(response.errors.is_none(), "Query returned errors");
    assert!(response.data.is_some(), "Query didn't return any data");
    let data = response.data.unwrap().conference;
    assert!(data.is_some(), "Conference was not set");
    let conf = data.unwrap();
    assert_eq!(conf.id, "1", "Returned the wrong conference");
    assert_eq!(
        conf.name, "Nextbuild 2018",
        "Returned the wrong conference name"
    );
    assert_eq!(
        response.debug_info.unwrap().source,
        ResultSource::Network,
        "Response didn't come from the server"
    );

    use artemis_test::add_conference::AddConference;
    // INVALIDATE CACHE
    let mutation_variables = artemis_test::add_conference::add_conference::Variables {
        name: "test_name".to_string(),
        city: Some("test_city".to_string())
    };
    let result = client.query(AddConference, mutation_variables).await;
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().debug_info.unwrap().source,
        ResultSource::Network
    );

    // CACHE SHOULD'VE BEEN INVALIDATED
    let result = client.query(GetConference, variables.clone()).await;

    assert!(result.is_ok(), "Query returned an error");

    let response = result.unwrap();
    assert!(response.errors.is_none(), "Query returned errors");
    assert!(response.data.is_some(), "Query didn't return any data");
    let data = response.data.unwrap().conference;
    assert!(data.is_some(), "Conference was not set");
    let conf = data.unwrap();
    assert_eq!(conf.id, "1", "Returned the wrong conference");
    assert_eq!(
        conf.name, "Nextbuild 2018",
        "Returned the wrong conference name"
    );
    assert_eq!(
        response.debug_info.unwrap().source,
        ResultSource::Network,
        "Response came from the cache"
    );
}
