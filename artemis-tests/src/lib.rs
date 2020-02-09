use crate::queries::add_conference::AddConference;
use artemis::{Client, ClientBuilder, GraphQLQuery, Middleware, ResultSource};

mod queries;

const URL: &str = "http://localhost:8080/graphql";

pub(crate) type Long = String;

pub async fn test_artemis() {
    print!("-- check code generated properly    ");
    check_code_gen();
    print!("✔️\n");
    print!("-- build client                     ");
    build_client();
    print!("✔️\n");
    print!("-- test query                       ");
    test_query().await;
    print!("✔️\n");
    print!("-- test cache                       ");
    test_cache().await;
    print!("✔️\n");
    print!("-- test cache invalidation          ");
    test_cache_invalidation().await;
    print!("✔️\n");
}

fn check_code_gen() {
    use queries::get_conference::get_conference::{
        GetConferenceConference, GetConferenceConferenceTalks,
        GetConferenceConferenceTalksSpeakers, ResponseData, Variables
    };
    let variables = Variables {
        id: "1".to_string()
    };
    let speakers = GetConferenceConferenceTalksSpeakers {
        name: "test_name".to_string()
    };
    let talks = GetConferenceConferenceTalks {
        id: "2".to_string(),
        title: "test_title".to_string(),
        speakers: Some(vec![speakers])
    };
    let conference = GetConferenceConference {
        id: "3".to_string(),
        name: "test_conf_name".to_string(),
        city: Some("test_city".to_string()),
        talks: Some(vec![talks])
    };
    let _response_data = ResponseData {
        conference: Some(conference)
    };

    let (query, _) = queries::get_conference::GetConference::build_query(variables);

    assert_eq!(query.variables.id, "1".to_string());
    assert_eq!(query.operation_name, "GetConference");
    // assert_eq!(meta.key, 1354603040u32); Apparently this is OS specific
}

fn build_client() -> Client<impl Middleware> {
    let builder = ClientBuilder::new(URL).with_default_middleware();

    builder.build()
}

async fn test_query() {
    let client = build_client();

    use queries::get_conference::{get_conference::*, GetConference};
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

async fn test_cache() {
    let client = build_client();

    use queries::get_conference::{get_conference::*, GetConference};
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

async fn test_cache_invalidation() {
    let client = build_client();

    use queries::get_conference::{get_conference::*, GetConference};
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

    // INVALIDATE CACHE
    let mutation_variables = queries::add_conference::add_conference::Variables {
        name: "test_name".to_string(),
        city: "test_city".to_string()
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
