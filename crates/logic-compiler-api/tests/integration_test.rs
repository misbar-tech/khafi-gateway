//! Integration tests for Logic Compiler API

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use logic_compiler_api::{create_router, AppState};
use serde_json::json;
use tower::ServiceExt; // for `oneshot`

/// Helper to create test app with temporary directories
fn create_test_app() -> (axum::Router, tempfile::TempDir, tempfile::TempDir) {
    let sdk_output_dir = tempfile::tempdir().unwrap();
    let templates_dir = tempfile::tempdir().unwrap();

    let state = AppState::new(
        sdk_output_dir.path().to_path_buf(),
        templates_dir.path().to_path_buf(),
    );

    let app = create_router(state);

    (app, sdk_output_dir, templates_dir)
}

#[tokio::test]
async fn test_health_check() {
    let (app, _sdk_dir, _templates_dir) = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "healthy");
    assert_eq!(json["service"], "logic-compiler-api");
}

#[tokio::test]
async fn test_validate_valid_dsl() {
    let (app, _sdk_dir, _templates_dir) = create_test_app();

    let dsl = json!({
        "use_case": "age_verification",
        "description": "Simple age check",
        "version": "1.0",
        "private_inputs": {
            "user_data": {
                "type": "object",
                "fields": {
                    "date_of_birth": "string"
                }
            }
        },
        "public_params": {
            "min_age": "u32"
        },
        "validation_rules": [
            {
                "type": "age_verification",
                "description": "Check minimum age",
                "dob_field": "date_of_birth",
                "min_age": 18
            }
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/validate")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&json!({ "dsl": dsl })).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["valid"], true);
    assert!(json["error"].is_null());
    assert!(json["parsed_dsl"].is_object());
}

#[tokio::test]
async fn test_validate_invalid_dsl() {
    let (app, _sdk_dir, _templates_dir) = create_test_app();

    // DSL with no validation rules (invalid)
    let dsl = json!({
        "use_case": "test",
        "private_inputs": {},
        "public_params": {},
        "validation_rules": []
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/validate")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&json!({ "dsl": dsl })).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["valid"], false);
    assert!(json["error"].is_string());
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("At least one validation rule"));
}

#[tokio::test]
async fn test_compile_valid_dsl() {
    let (app, _sdk_dir, _templates_dir) = create_test_app();

    let dsl = json!({
        "use_case": "age_verification",
        "description": "Simple age check",
        "version": "1.0",
        "private_inputs": {
            "user_data": {
                "type": "object",
                "fields": {
                    "date_of_birth": "string"
                }
            }
        },
        "public_params": {
            "min_age": "u32"
        },
        "validation_rules": [
            {
                "type": "age_verification",
                "description": "Check minimum age",
                "dob_field": "date_of_birth",
                "min_age": 18
            }
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/compile")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&json!({ "dsl": dsl })).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert!(json["code"].is_string());
    assert!(json["error"].is_null());

    // Verify generated code contains expected elements
    let code = json["code"].as_str().unwrap();
    assert!(code.contains("risc0_zkvm"));
    assert!(code.contains("main()"));
}

#[tokio::test]
async fn test_compile_invalid_dsl() {
    let (app, _sdk_dir, _templates_dir) = create_test_app();

    // DSL with no validation rules
    let dsl = json!({
        "use_case": "test",
        "private_inputs": {},
        "public_params": {},
        "validation_rules": []
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/compile")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&json!({ "dsl": dsl })).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], false);
    assert!(json["code"].is_null());
    assert!(json["error"].is_string());
}

#[tokio::test]
async fn test_generate_sdk() {
    let (app, sdk_dir, _templates_dir) = create_test_app();

    let dsl = json!({
        "use_case": "age_verification",
        "description": "Simple age check",
        "version": "1.0",
        "private_inputs": {
            "user_data": {
                "type": "object",
                "fields": {
                    "date_of_birth": "string"
                }
            }
        },
        "public_params": {
            "min_age": "u32"
        },
        "validation_rules": [
            {
                "type": "age_verification",
                "description": "Check minimum age",
                "dob_field": "date_of_birth",
                "min_age": 18
            }
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/sdk/generate")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&json!({ "dsl": dsl })).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert!(json["sdk_id"].is_string());
    assert!(json["error"].is_null());

    // Verify SDK directory was created
    let sdk_id = json["sdk_id"].as_str().unwrap();
    let sdk_path = sdk_dir.path().join(sdk_id);
    assert!(sdk_path.exists());

    // Verify SDK structure
    assert!(sdk_path.join("methods/guest/src/main.rs").exists());
    assert!(sdk_path.join("methods/guest/Cargo.toml").exists());
    assert!(sdk_path.join("methods/Cargo.toml").exists());
    assert!(sdk_path.join("methods/build.rs").exists());
}

#[tokio::test]
async fn test_list_templates_empty() {
    let (app, _sdk_dir, _templates_dir) = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/templates")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["templates"].is_array());
    assert_eq!(json["templates"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_get_template_not_found() {
    let (app, _sdk_dir, _templates_dir) = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/templates/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
