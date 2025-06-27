use axum::{
    body::Body,
    http::{Request, StatusCode},
};
// NEW: Import the BodyExt trait for the .collect() method
use http_body_util::BodyExt;
use tower::ServiceExt; // for `oneshot`
use std::sync::{Arc, Mutex};
use shared::Contact;

use backend::web_server::{create_router, AppState};

#[tokio::test]
async fn test_get_contacts_endpoint() {
    // 1. ARRANGE
    let test_contacts = vec![
        Contact { id: 1, name: "Test User".into(), email: "test@example.com".into() },
    ];
    let app_state: AppState = Arc::new(Mutex::new(test_contacts.clone()));
    let app = create_router(app_state);

    // 2. ACT
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/contacts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 3. ASSERT
    assert_eq!(response.status(), StatusCode::OK);

    // UPDATED: This is the new, correct way to read the response body to bytes.
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();

    let contacts_from_api: Vec<Contact> = serde_json::from_slice(&body_bytes)
        .expect("Failed to deserialize contacts from API response");

    assert_eq!(contacts_from_api, test_contacts);
}


#[tokio::test]
async fn test_create_contact_endpoint() {
    // 1. ARRANGE: Start with an empty in-memory "database".
    let app_state: AppState = Arc::new(Mutex::new(vec![]));
    let app = create_router(app_state.clone()); // Clone for router, keep original for assertion

    // Define the new contact to be sent in the request body.
    let new_contact_payload = Contact {
        id: 0, // ID is ignored by the server, which will assign a new one.
        name: "Bender Rodriguez".into(),
        email: "bender@planetexpress.com".into(),
    };

    // 2. ACT: Send a POST request to create the contact.
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/contacts")
                .header("Content-Type", "application/json") // Set the content type
                .body(Body::from(
                    // Serialize the payload into a JSON body
                    serde_json::to_vec(&new_contact_payload).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // 3. ASSERT
    assert_eq!(response.status(), StatusCode::OK);

    // Assert that the response body contains the newly created contact with an ID.
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let created_contact: Contact = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(created_contact.id, 1); // Server should assign the first ID.
    assert_eq!(created_contact.name, new_contact_payload.name);
    assert_eq!(created_contact.email, new_contact_payload.email);

    // Assert that the application state was actually modified.
    let final_state = app_state.lock().unwrap();
    assert_eq!(final_state.len(), 1);
    assert_eq!(final_state[0].id, 1);
}

#[tokio::test]
async fn test_get_contacts_empty_list() {
    // 1. ARRANGE: Start with an empty state.
    let app_state: AppState = Arc::new(Mutex::new(vec![]));
    let app = create_router(app_state);

    // 2. ACT: Request the list of contacts.
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/contacts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 3. ASSERT
    assert_eq!(response.status(), StatusCode::OK);

    // Assert that the body is an empty JSON array `[]`.
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let contacts_from_api: Vec<Contact> = serde_json::from_slice(&body_bytes).unwrap();

    assert!(contacts_from_api.is_empty());
}
