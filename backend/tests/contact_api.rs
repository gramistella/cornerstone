use common::{ContactDto, Credentials, LoginResponse};
use reqwest::StatusCode;
mod helpers;
use crate::helpers::TEST_JWT_SECRET;
use backend::auth::Claims;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use once_cell::sync::Lazy;
use serde_json::json;

static TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO);
    subscriber.init();
});

#[tokio::test]
async fn test_register_login_logout_flow() {
    Lazy::force(&TRACING);

    // Arrange: Spawn the app and get a client
    let (addr, client, _db_pool) = helpers::spawn_app().await;

    let register_url = format!("http://{addr}/api/v1/register");
    let login_url = format!("http://{addr}/api/v1/login");
    let logout_url = format!("http://{addr}/api/v1/logout");

    let credentials = Credentials {
        email: "test_user@example.com".to_string(),
        password: "password123".to_string(),
    };

    // 1. Register a new user
    let response = client
        .post(&register_url)
        .json(&credentials)
        .send()
        .await
        .expect("Failed to execute register request.");

    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "Should succeed in registering a new user"
    );

    // 2. Registering the same user again should fail
    let response = client
        .post(&register_url)
        .json(&credentials)
        .send()
        .await
        .expect("Failed to execute second register request.");

    assert_eq!(
        response.status(),
        StatusCode::CONFLICT,
        "Should fail with conflict when registering existing user"
    );

    // 3. Log in with correct credentials
    let response = client
        .post(&login_url)
        .json(&credentials)
        .send()
        .await
        .expect("Failed to execute login request.");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should succeed with correct credentials"
    );
    let login_response: LoginResponse = response
        .json()
        .await
        .expect("Failed to parse login response");
    assert!(!login_response.access_token.is_empty());
    assert!(!login_response.refresh_token.is_empty());
    let access_token = login_response.access_token;

    // 4. Log in with incorrect password
    let bad_credentials = Credentials {
        email: "test_user@example.com".to_string(),
        password: "wrongpassword".to_string(),
    };
    let response = client
        .post(&login_url)
        .json(&bad_credentials)
        .send()
        .await
        .expect("Failed to execute bad login request.");
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Should fail with incorrect password"
    );

    // 5. Logout using the access token
    let response = client
        .post(&logout_url)
        .bearer_auth(access_token)
        .send()
        .await
        .expect("Failed to execute logout request.");

    assert_eq!(
        response.status(),
        StatusCode::NO_CONTENT,
        "Should successfully logout"
    );

    // 6. Verify logout invalidates refresh token (by trying to use refresh which should now be gone)
    let refresh_url = format!("http://{addr}/api/v1/refresh");
    let response = client
        .post(&refresh_url)
        .json(&json!({ "refresh_token": login_response.refresh_token }))
        .send()
        .await
        .expect("Failed to execute refresh request after logout.");

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Refresh token should be invalid after logout"
    );
}

#[tokio::test]
async fn test_token_refresh() {
    Lazy::force(&TRACING);
    let (addr, client, _db_pool) = helpers::spawn_app().await;

    // 1. Register and Login to get tokens.
    // The helper function creates a user with email "test@example.com"
    helpers::get_auth_token(&addr, &client).await;

    // Now, login again to get the full login response with the refresh token
    let login_url = format!("http://{addr}/api/v1/login");
    let credentials = Credentials {
        email: "test@example.com".to_string(),
        password: "password123".to_string(),
    };
    let response = client
        .post(&login_url)
        .json(&credentials)
        .send()
        .await
        .unwrap();
    let original_tokens: LoginResponse = response.json().await.unwrap();

    // 2. Use the refresh token to get a new set of tokens
    let refresh_url = format!("http://{addr}/api/v1/refresh");
    let response = client
        .post(&refresh_url)
        .json(&json!({ "refresh_token": original_tokens.refresh_token }))
        .send()
        .await
        .expect("Failed to execute refresh request.");

    assert_eq!(response.status(), StatusCode::OK);
    let new_tokens: LoginResponse = response
        .json()
        .await
        .expect("Failed to parse refresh response");

    // Assert that we got new tokens
    assert_ne!(
        original_tokens.access_token, new_tokens.access_token,
        "Access token should be different after refresh"
    );
    assert_ne!(
        original_tokens.refresh_token, new_tokens.refresh_token,
        "Refresh token should be rotated and different after refresh"
    );

    // 3. Try to use the OLD refresh token again, which should fail
    let response = client
        .post(&refresh_url)
        .json(&json!({ "refresh_token": original_tokens.refresh_token }))
        .send()
        .await
        .expect("Failed to execute request with old refresh token.");
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Old refresh token should be invalidated after use"
    );

    // 4. Use an invalid/random refresh token
    let response = client
        .post(&refresh_url)
        .json(&json!({ "refresh_token": "invalid-token-string" }))
        .send()
        .await
        .expect("Failed to execute request with invalid refresh token.");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_contacts_crud_flow() {
    Lazy::force(&TRACING);

    // Arrange: Spawn the app and get an authenticated client
    let (addr, client, _db_pool) = helpers::spawn_app().await;
    let token = helpers::get_auth_token(&addr, &client).await;

    let contacts_url = format!("http://{addr}/api/v1/contacts");

    // 1. Initially, GET contacts should return an empty list
    let response = client
        .get(&contacts_url)
        .bearer_auth(&token)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status(), StatusCode::OK);
    let contacts: Vec<ContactDto> = response.json().await.unwrap();
    assert!(
        contacts.is_empty(),
        "Initially there should be no contacts."
    );

    // 2. CREATE a new contact
    let new_contact = ContactDto {
        id: None, // ID is generated by the DB
        name: "John Doe".to_string(),
        email: "john.doe@test.com".to_string(),
        age: 30,
        subscribed: true,
        contact_type: "Friend".to_string(),
    };

    let response = client
        .post(&contacts_url)
        .bearer_auth(&token)
        .json(&new_contact)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status(), StatusCode::CREATED);
    let created_contact: ContactDto = response.json().await.unwrap();
    assert_eq!(created_contact.name, new_contact.name);
    assert!(created_contact.id.is_some());

    let contact_id = created_contact.id.unwrap();
    let single_contact_url = format!("{contacts_url}/{contact_id}");

    // 3. GET the created contact by its ID
    let response = client
        .get(&single_contact_url)
        .bearer_auth(&token)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status(), StatusCode::OK);
    let fetched_contact: ContactDto = response.json().await.unwrap();
    assert_eq!(fetched_contact.id, Some(contact_id));
    assert_eq!(fetched_contact.name, "John Doe");

    // 4. UPDATE the contact
    let updated_contact_data = ContactDto {
        id: Some(contact_id),
        name: "John Smith".to_string(),           // Name changed
        email: "john.smith@test.com".to_string(), // Email changed
        age: 31,
        subscribed: false,
        contact_type: "Work".to_string(),
    };

    let response = client
        .put(&single_contact_url)
        .bearer_auth(&token)
        .json(&updated_contact_data)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status(), StatusCode::OK);
    let updated_contact_response: ContactDto = response.json().await.unwrap();
    assert_eq!(updated_contact_response.name, "John Smith");
    assert_eq!(updated_contact_response.age, 31);

    // 5. DELETE the contact
    let response = client
        .delete(&single_contact_url)
        .bearer_auth(&token)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // 6. Verify the contact is gone
    let response = client
        .get(&single_contact_url)
        .bearer_auth(&token)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_contacts_authorization() {
    Lazy::force(&TRACING);

    // Arrange: Spawn the app and get a client
    let (addr, client, _db_pool) = helpers::spawn_app().await;
    let register_url = format!("http://{addr}/api/v1/register");
    let login_url = format!("http://{addr}/api/v1/login");
    let contacts_url = format!("http://{addr}/api/v1/contacts");

    // 1. Create User A and get their token
    let user_a_credentials = Credentials {
        email: "user_a@example.com".to_string(),
        password: "password123".to_string(),
    };
    client
        .post(&register_url)
        .json(&user_a_credentials)
        .send()
        .await
        .unwrap();
    let login_res_a = client
        .post(&login_url)
        .json(&user_a_credentials)
        .send()
        .await
        .unwrap();
    let user_a_token = login_res_a
        .json::<LoginResponse>()
        .await
        .unwrap()
        .access_token;

    // 2. Create User B and get their token
    let user_b_credentials = Credentials {
        email: "user_b@example.com".to_string(),
        password: "password123".to_string(),
    };
    client
        .post(&register_url)
        .json(&user_b_credentials)
        .send()
        .await
        .unwrap();
    let login_res_b = client
        .post(&login_url)
        .json(&user_b_credentials)
        .send()
        .await
        .unwrap();
    let user_b_token = login_res_b
        .json::<LoginResponse>()
        .await
        .unwrap()
        .access_token;

    // 3. User A creates a contact
    let new_contact = ContactDto {
        id: None,
        name: "User A's Contact".to_string(),
        email: "user_a_contact@test.com".to_string(),
        age: 40,
        subscribed: false,
        contact_type: "Private".to_string(),
    };
    let response = client
        .post(&contacts_url)
        .bearer_auth(&user_a_token)
        .json(&new_contact)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created_contact: ContactDto = response.json().await.unwrap();
    let contact_id = created_contact.id.unwrap();
    let user_a_contact_url = format!("{contacts_url}/{contact_id}");

    // 4. User B attempts to access User A's contact and fails
    // GET
    let response = client
        .get(&user_a_contact_url)
        .bearer_auth(&user_b_token)
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "User B should not be able to GET User A's contact"
    );

    // PUT
    let response = client
        .put(&user_a_contact_url)
        .bearer_auth(&user_b_token)
        .json(&created_contact) // body content doesn't matter much here
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "User B should not be able to PUT User A's contact"
    );

    // DELETE
    let response = client
        .delete(&user_a_contact_url)
        .bearer_auth(&user_b_token)
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "User B should not be able to DELETE User A's contact"
    );

    // 5. Sanity check: Verify User A can still access their contact
    let response = client
        .get(&user_a_contact_url)
        .bearer_auth(&user_a_token)
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "User A should still be able to access their own contact"
    );
}

#[tokio::test]
async fn test_protected_routes_require_auth() {
    Lazy::force(&TRACING);

    // Arrange: Spawn the app and get a client
    let (addr, client, _db_pool) = helpers::spawn_app().await;

    // Define protected routes and their methods
    let routes = vec![
        (
            reqwest::Method::POST,
            format!("http://{addr}/api/v1/logout"),
        ),
        (
            reqwest::Method::GET,
            format!("http://{addr}/api/v1/contacts"),
        ),
        (
            reqwest::Method::POST,
            format!("http://{addr}/api/v1/contacts"),
        ),
        (
            reqwest::Method::GET,
            format!("http://{addr}/api/v1/contacts/1"),
        ),
        (
            reqwest::Method::PUT,
            format!("http://{addr}/api/v1/contacts/1"),
        ),
        (
            reqwest::Method::DELETE,
            format!("http://{addr}/api/v1/contacts/1"),
        ),
    ];

    for (method, url) in routes {
        // Act: Send request without auth header
        let response = client.request(method.clone(), &url).send().await.unwrap();

        // Assert: We get a 401 Unauthorized because the token is missing
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "Endpoint {url} should be protected and return 401 without auth"
        );
    }
}

#[tokio::test]
async fn test_invalid_and_expired_tokens() {
    Lazy::force(&TRACING);

    // Arrange: Spawn the app
    let (addr, client, _db_pool) = helpers::spawn_app().await;
    let protected_url = format!("http://{addr}/api/v1/contacts");

    // Scenario 1: Using a completely invalid/malformed token
    let response = client
        .get(&protected_url)
        .bearer_auth("this-is-not-a-valid-jwt")
        .send()
        .await
        .unwrap();

    // Assert: The middleware rejects the malformed token
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Should reject a malformed JWT"
    );

    // Scenario 2: Using a valid but expired token
    // First, register a user so we have a valid user ID (sub claim) for our token
    let register_url = format!("http://{addr}/api/v1/register");
    let credentials = Credentials {
        email: "expired_token_user@example.com".to_string(),
        password: "password123".to_string(),
    };
    // This user will get id=1 in the context of this test's database
    client
        .post(&register_url)
        .json(&credentials)
        .send()
        .await
        .unwrap();

    // Manually create an expired token
    let expiration = Utc::now()
        .checked_sub_signed(Duration::seconds(30)) // Set expiry to 30 seconds in the past, this might need adjustment based on the validation leeway
        .expect("Failed to create timestamp")
        .timestamp();

    let claims = Claims {
        sub: "1".to_string(), // `sub` claim for the user we just created
        exp: expiration as usize,
        nonce: "test-nonce".to_string(),
    };
    // The test secret is hardcoded in `helpers::spawn_app`
    let secret = EncodingKey::from_secret(TEST_JWT_SECRET.as_ref());
    let expired_token = encode(&Header::default(), &claims, &secret).unwrap();

    // Act: Send request with the expired token
    let response = client
        .get(&protected_url)
        .bearer_auth(expired_token)
        .send()
        .await
        .unwrap();

    // Assert: The middleware rejects the expired token
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Should reject an expired JWT"
    );
}

#[tokio::test]
async fn test_validation_errors() {
    Lazy::force(&TRACING);
    let (addr, client, _db_pool) = helpers::spawn_app().await;
    let token = helpers::get_auth_token(&addr, &client).await;

    // Test case 1: Register with invalid email
    let register_url = format!("http://{addr}/api/v1/register");
    let invalid_email_creds = json!({
        "email": "not-an-email",
        "password": "longenoughpassword"
    });
    let response = client
        .post(&register_url)
        .json(&invalid_email_creds)
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "Should fail with invalid email format"
    );

    // Test case 2: Register with short password
    let short_password_creds = json!({
        "email": "another@user.com",
        "password": "short"
    });
    let response = client
        .post(&register_url)
        .json(&short_password_creds)
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "Should fail with short password"
    );

    // Test case 3: Create contact with empty name
    let contacts_url = format!("http://{addr}/api/v1/contacts");
    let invalid_contact = json!({
        "name": "",
        "email": "some.contact@test.com",
        "age": 25,
        "subscribed": false,
        "contact_type": "Work"
    });
    let response = client
        .post(&contacts_url)
        .bearer_auth(&token)
        .json(&invalid_contact)
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "Should fail with empty contact name"
    );
}

#[tokio::test]
async fn test_contacts_pagination() {
    Lazy::force(&TRACING);
    let (addr, client, _db_pool) = helpers::spawn_app().await;
    let token = helpers::get_auth_token(&addr, &client).await;
    let contacts_url = format!("http://{addr}/api/v1/contacts");

    // Create 15 contacts
    for i in 0..15 {
        let contact = ContactDto {
            id: None,
            name: format!("Contact {i}"),
            email: format!("contact{i}@test.com"),
            age: 30 + i,
            subscribed: i % 2 == 0,
            contact_type: "Test".to_string(),
        };
        let response = client
            .post(&contacts_url)
            .bearer_auth(&token)
            .json(&contact)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    // Test Page 1: should have 10 items
    let response = client
        .get(format!("{contacts_url}?page=1&per_page=10"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let page1_contacts: Vec<ContactDto> = response.json().await.unwrap();
    assert_eq!(
        page1_contacts.len(),
        10,
        "Page 1 should contain 10 contacts"
    );

    // Test Page 2: should have 5 items
    let response = client
        .get(format!("{contacts_url}?page=2&per_page=10"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let page2_contacts: Vec<ContactDto> = response.json().await.unwrap();
    assert_eq!(page2_contacts.len(), 5, "Page 2 should contain 5 contacts");

    // Test Page 3: should have 0 items
    let response = client
        .get(format!("{contacts_url}?page=3&per_page=10"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let page3_contacts: Vec<ContactDto> = response.json().await.unwrap();
    assert!(page3_contacts.is_empty(), "Page 3 should be empty");
}
