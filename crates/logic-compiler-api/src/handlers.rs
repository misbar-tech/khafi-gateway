//! API request handlers for logic compiler operations

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use logic_compiler::{BusinessRulesDSL, CodeGenerator, DslParser};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::AppState;

/// Request to validate DSL
#[derive(Debug, Deserialize)]
pub struct ValidateRequest {
    /// JSON DSL specification
    pub dsl: serde_json::Value,
}

/// Response from validation
#[derive(Debug, Serialize)]
pub struct ValidateResponse {
    /// Whether the DSL is valid
    pub valid: bool,

    /// Error message if invalid
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Parsed DSL structure if valid
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed_dsl: Option<BusinessRulesDSL>,
}

/// Request to compile DSL
#[derive(Debug, Deserialize)]
pub struct CompileRequest {
    /// JSON DSL specification
    pub dsl: serde_json::Value,
}

/// Response from compilation
#[derive(Debug, Serialize)]
pub struct CompileResponse {
    /// Whether compilation succeeded
    pub success: bool,

    /// Generated Rust code (guest program)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// Error message if compilation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request to generate SDK package
#[derive(Debug, Deserialize)]
pub struct GenerateSdkRequest {
    /// JSON DSL specification
    pub dsl: serde_json::Value,
}

/// Response from SDK generation
#[derive(Debug, Serialize)]
pub struct GenerateSdkResponse {
    /// Whether SDK generation succeeded
    pub success: bool,

    /// SDK package ID for download
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_id: Option<String>,

    /// Error message if generation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request to deploy DSL to gateway
#[derive(Debug, Deserialize)]
pub struct DeployRequest {
    /// JSON DSL specification
    pub dsl: serde_json::Value,

    /// Customer identifier
    pub customer_id: String,
}

/// Response from deployment
#[derive(Debug, Serialize)]
pub struct DeployResponse {
    /// Whether deployment succeeded
    pub success: bool,

    /// Customer ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,

    /// Image ID of deployed guest program
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_id: Option<String>,

    /// API endpoint URL for proof generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<String>,

    /// Error message if deployment failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Template metadata
#[derive(Debug, Serialize)]
pub struct TemplateInfo {
    /// Template identifier
    pub name: String,

    /// Human-readable title
    pub title: String,

    /// Description of what this template does
    pub description: String,

    /// Use case category
    pub category: String,
}

/// List of available templates
#[derive(Debug, Serialize)]
pub struct TemplatesResponse {
    pub templates: Vec<TemplateInfo>,
}

/// API Error type
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": self.message
        });

        (self.status, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: err.to_string(),
        }
    }
}

/// Health check endpoint
pub async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "logic-compiler-api"
    }))
}

/// Validate DSL without compiling
pub async fn validate_handler(
    Json(payload): Json<ValidateRequest>,
) -> Result<Json<ValidateResponse>, ApiError> {
    info!("Validating DSL");

    // Convert Value to JSON string
    let dsl_json = serde_json::to_string(&payload.dsl).map_err(|e| ApiError {
        status: StatusCode::BAD_REQUEST,
        message: format!("Invalid JSON: {}", e),
    })?;

    // Parse and validate
    match DslParser::parse_str(&dsl_json) {
        Ok(parsed_dsl) => {
            info!("DSL validation successful");
            Ok(Json(ValidateResponse {
                valid: true,
                error: None,
                parsed_dsl: Some(parsed_dsl),
            }))
        }
        Err(e) => {
            info!("DSL validation failed: {}", e);
            Ok(Json(ValidateResponse {
                valid: false,
                error: Some(e.to_string()),
                parsed_dsl: None,
            }))
        }
    }
}

/// Compile DSL to guest program code
pub async fn compile_handler(
    Json(payload): Json<CompileRequest>,
) -> Result<Json<CompileResponse>, ApiError> {
    info!("Compiling DSL");

    // Convert Value to JSON string
    let dsl_json = serde_json::to_string(&payload.dsl).map_err(|e| ApiError {
        status: StatusCode::BAD_REQUEST,
        message: format!("Invalid JSON: {}", e),
    })?;

    // Parse DSL
    let parsed_dsl = match DslParser::parse_str(&dsl_json) {
        Ok(dsl) => dsl,
        Err(e) => {
            error!("Failed to parse DSL: {}", e);
            return Ok(Json(CompileResponse {
                success: false,
                code: None,
                error: Some(format!("DSL validation failed: {}", e)),
            }));
        }
    };

    // Generate code
    let generator = CodeGenerator::new(parsed_dsl);
    match generator.generate() {
        Ok(code) => {
            info!("Code generation successful");
            Ok(Json(CompileResponse {
                success: true,
                code: Some(code),
                error: None,
            }))
        }
        Err(e) => {
            error!("Code generation failed: {}", e);
            Ok(Json(CompileResponse {
                success: false,
                code: None,
                error: Some(format!("Code generation failed: {}", e)),
            }))
        }
    }
}

/// Generate complete SDK package
pub async fn generate_sdk_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<GenerateSdkRequest>,
) -> Result<Json<GenerateSdkResponse>, ApiError> {
    info!("Generating SDK package");

    // Convert Value to JSON string
    let dsl_json = serde_json::to_string(&payload.dsl).map_err(|e| ApiError {
        status: StatusCode::BAD_REQUEST,
        message: format!("Invalid JSON: {}", e),
    })?;

    // Parse DSL
    let parsed_dsl = match DslParser::parse_str(&dsl_json) {
        Ok(dsl) => dsl,
        Err(e) => {
            error!("Failed to parse DSL: {}", e);
            return Ok(Json(GenerateSdkResponse {
                success: false,
                sdk_id: None,
                error: Some(format!("DSL validation failed: {}", e)),
            }));
        }
    };

    // Generate unique SDK ID
    let sdk_id = Uuid::new_v4().to_string();
    let sdk_dir = state.sdk_output_dir.join(&sdk_id);

    // Generate SDK package
    let use_case = parsed_dsl.use_case.clone();
    let generator = CodeGenerator::new(parsed_dsl);
    match generator.generate_sdk_package(&sdk_dir) {
        Ok(_) => {
            // Save use_case for better filename on download
            let _ = std::fs::write(sdk_dir.join("use_case.txt"), &use_case);

            info!("SDK package generated: {}", sdk_id);
            Ok(Json(GenerateSdkResponse {
                success: true,
                sdk_id: Some(sdk_id),
                error: None,
            }))
        }
        Err(e) => {
            error!("SDK generation failed: {}", e);
            // Clean up failed directory
            let _ = std::fs::remove_dir_all(&sdk_dir);
            Ok(Json(GenerateSdkResponse {
                success: false,
                sdk_id: None,
                error: Some(format!("SDK generation failed: {}", e)),
            }))
        }
    }
}

/// Download SDK package as tarball
pub async fn download_sdk_handler(
    State(state): State<Arc<AppState>>,
    Path(sdk_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Downloading SDK package: {}", sdk_id);

    let sdk_dir = state.sdk_output_dir.join(&sdk_id);

    if !sdk_dir.exists() {
        return Err(ApiError {
            status: StatusCode::NOT_FOUND,
            message: format!("SDK package not found: {}", sdk_id),
        });
    }

    // Try to read use_case from metadata file for better filename
    let use_case = std::fs::read_to_string(sdk_dir.join("use_case.txt"))
        .ok()
        .and_then(|s| Some(s.trim().replace(' ', "-").to_lowercase()))
        .unwrap_or_else(|| "sdk".to_string());

    let filename = format!("{}-sdk.tar.gz", use_case);

    // Create tarball
    let tarball_path = state.sdk_output_dir.join(format!("{}.tar.gz", sdk_id));
    create_tarball(&sdk_dir, &tarball_path)?;

    // Read tarball
    let tarball_data = std::fs::read(&tarball_path).map_err(|e| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("Failed to read tarball: {}", e),
    })?;

    // Clean up tarball after reading
    let _ = std::fs::remove_file(&tarball_path);

    // Return with proper headers
    use axum::body::Body;
    use axum::response::Response;
    use axum::http::header;

    let content_disposition = format!("attachment; filename=\"{}\"", filename);

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/gzip")
        .header(header::CONTENT_DISPOSITION, content_disposition)
        .body(Body::from(tarball_data))
        .unwrap())
}

/// List available templates
pub async fn list_templates_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<TemplatesResponse>, ApiError> {
    info!("Listing templates");

    let templates_dir = &state.templates_dir;
    let mut templates = Vec::new();

    // Read all JSON files in templates directory
    if templates_dir.exists() {
        let entries = std::fs::read_dir(templates_dir).map_err(|e| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Failed to read templates directory: {}", e),
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                // Parse template to get metadata
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(dsl) = DslParser::parse_str(&content) {
                        templates.push(TemplateInfo {
                            name: path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            title: dsl.use_case.clone(),
                            description: dsl.description.clone(),
                            category: categorize_use_case(&dsl.use_case),
                        });
                    }
                }
            }
        }
    }

    Ok(Json(TemplatesResponse { templates }))
}

/// Deploy DSL to gateway - builds guest program and registers with Image ID Registry
pub async fn deploy_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DeployRequest>,
) -> Result<Json<DeployResponse>, ApiError> {
    info!("Deploying DSL for customer: {}", payload.customer_id);

    // Convert Value to JSON string
    let dsl_json = serde_json::to_string(&payload.dsl).map_err(|e| ApiError {
        status: StatusCode::BAD_REQUEST,
        message: format!("Invalid JSON: {}", e),
    })?;

    // Parse DSL
    let parsed_dsl = match DslParser::parse_str(&dsl_json) {
        Ok(dsl) => dsl,
        Err(e) => {
            error!("Failed to parse DSL: {}", e);
            return Ok(Json(DeployResponse {
                success: false,
                customer_id: None,
                image_id: None,
                api_endpoint: None,
                error: Some(format!("DSL validation failed: {}", e)),
            }));
        }
    };

    // Generate unique deployment ID
    let deployment_id = Uuid::new_v4().to_string();
    let deployment_dir = state.sdk_output_dir.join(&deployment_id);

    // Generate SDK package (which includes the guest program)
    let generator = CodeGenerator::new(parsed_dsl.clone());
    if let Err(e) = generator.generate_sdk_package(&deployment_dir) {
        error!("SDK generation failed: {}", e);
        return Ok(Json(DeployResponse {
            success: false,
            customer_id: None,
            image_id: None,
            api_endpoint: None,
            error: Some(format!("SDK generation failed: {}", e)),
        }));
    }

    // Build the guest program using cargo risczero build
    info!("Building guest program for customer: {}", payload.customer_id);
    let methods_dir = deployment_dir.join("methods");
    let build_result = std::process::Command::new("cargo")
        .arg("risczero")
        .arg("build")
        .current_dir(&methods_dir)
        .output()
        .map_err(|e| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Failed to execute build: {}", e),
        })?;

    if !build_result.status.success() {
        let stderr = String::from_utf8_lossy(&build_result.stderr);
        error!("Build failed: {}", stderr);
        // Clean up failed directory
        let _ = std::fs::remove_dir_all(&deployment_dir);
        return Ok(Json(DeployResponse {
            success: false,
            customer_id: None,
            image_id: None,
            api_endpoint: None,
            error: Some(format!("Build failed: {}", stderr)),
        }));
    }

    info!("Build successful for customer: {}", payload.customer_id);

    // Extract Image ID from the built ELF
    // The ELF should be in methods/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/guest
    let guest_elf_path = methods_dir
        .join("target/riscv-guest/riscv32im-risc0-zkvm-elf/release/guest");

    if !guest_elf_path.exists() {
        error!("Guest ELF not found at expected path");
        let _ = std::fs::remove_dir_all(&deployment_dir);
        return Ok(Json(DeployResponse {
            success: false,
            customer_id: None,
            image_id: None,
            api_endpoint: None,
            error: Some("Guest ELF not found after build".to_string()),
        }));
    }

    // Compute Image ID from ELF
    let elf_bytes = std::fs::read(&guest_elf_path).map_err(|e| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("Failed to read guest ELF: {}", e),
    })?;

    // Use RISC Zero to compute the Image ID
    use risc0_zkvm::compute_image_id;
    let image_id_bytes = compute_image_id(&elf_bytes).map_err(|e| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("Failed to compute image ID: {}", e),
    })?;
    let image_id = hex::encode(image_id_bytes);

    info!("Computed Image ID: {} for customer: {}", image_id, payload.customer_id);

    // Register with Image ID Registry
    let registry_url = std::env::var("REGISTRY_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8083".to_string());

    let registry_payload = serde_json::json!({
        "customer_id": payload.customer_id,
        "image_id": image_id,
        "guest_program_path": guest_elf_path.to_string_lossy(),
        "metadata": {
            "use_case": parsed_dsl.use_case,
            "description": parsed_dsl.description,
            "version": parsed_dsl.version
        }
    });

    let client = reqwest::Client::new();
    let registry_response = client
        .post(format!("{}/api/deployments", registry_url))
        .json(&registry_payload)
        .send()
        .await
        .map_err(|e| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Failed to register with Image ID Registry: {}", e),
        })?;

    if !registry_response.status().is_success() {
        let error_text = registry_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        error!("Failed to register deployment: {}", error_text);
        // Clean up deployment directory
        let _ = std::fs::remove_dir_all(&deployment_dir);
        return Ok(Json(DeployResponse {
            success: false,
            customer_id: None,
            image_id: None,
            api_endpoint: None,
            error: Some(format!("Failed to register deployment: {}", error_text)),
        }));
    }

    info!("Successfully deployed for customer: {}", payload.customer_id);

    // Construct API endpoint URL
    let gateway_url = std::env::var("GATEWAY_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    let api_endpoint = format!("{}/api/prove", gateway_url);

    Ok(Json(DeployResponse {
        success: true,
        customer_id: Some(payload.customer_id.clone()),
        image_id: Some(image_id),
        api_endpoint: Some(api_endpoint),
        error: None,
    }))
}

/// Get a specific template by name
pub async fn get_template_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("Getting template: {}", name);

    let template_path = state.templates_dir.join(format!("{}.json", name));

    if !template_path.exists() {
        return Err(ApiError {
            status: StatusCode::NOT_FOUND,
            message: format!("Template not found: {}", name),
        });
    }

    let content = std::fs::read_to_string(&template_path).map_err(|e| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("Failed to read template: {}", e),
    })?;

    let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("Failed to parse template: {}", e),
    })?;

    Ok(Json(json))
}

/// Helper: Create tarball from directory
fn create_tarball(
    source_dir: &std::path::Path,
    output_path: &std::path::Path,
) -> Result<(), ApiError> {
    use std::fs::File;

    let tar_gz = File::create(output_path).map_err(|e| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("Failed to create tarball: {}", e),
    })?;

    let enc = flate2::write::GzEncoder::new(tar_gz, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);

    tar.append_dir_all(".", source_dir).map_err(|e| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("Failed to create tar archive: {}", e),
    })?;

    tar.finish().map_err(|e| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("Failed to finish tar archive: {}", e),
    })?;

    Ok(())
}

/// Helper: Categorize use case into a category
fn categorize_use_case(use_case: &str) -> String {
    let lower = use_case.to_lowercase();
    if lower.contains("age") || lower.contains("identity") {
        "Identity Verification".to_string()
    } else if lower.contains("pharma") || lower.contains("prescription") {
        "Healthcare".to_string()
    } else if lower.contains("shipping") || lower.contains("manifest") {
        "Supply Chain".to_string()
    } else if lower.contains("finance") || lower.contains("payment") {
        "Financial".to_string()
    } else {
        "General".to_string()
    }
}
