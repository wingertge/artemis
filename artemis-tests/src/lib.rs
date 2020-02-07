use artemis::GraphQLQuery;

mod queries;

pub(crate) type Long = i64;

pub fn test_artemis() {
    print!("-- check code generated properly    ");
    check_code_gen();
    print!("✔️\n")
}

fn check_code_gen() {
    use queries::get_conference::get_conference::{
        GetConferenceConference, GetConferenceConferenceTalks,
        GetConferenceConferenceTalksSpeakers, ResponseData, Variables
    };
    let variables = Variables { id: 1 };
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
    let response_data = ResponseData {
        conference: Some(conference)
    };

    let query = queries::get_conference::GetConference::build_query(variables);

    assert_eq!(query.variables.id, 1);
    assert_eq!(query.operation_name, "GetConference");
}
