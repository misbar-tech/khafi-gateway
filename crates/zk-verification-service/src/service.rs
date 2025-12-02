//! Authorization service implementation for Envoy ExtAuth

use khafi_common::{Nullifier, Receipt};
use std::collections::HashMap;
use tonic::{Request, Response, Status};

use crate::config::Config;
use crate::nullifier::NullifierChecker;
use crate::payment::PaymentChecker;

// Include the generated protobuf code
pub mod proto {
    tonic::include_proto!("envoy.service.auth.v3");
}

use proto::{
    authorization_server::{Authorization, AuthorizationServer},
    CheckRequest, CheckResponse, StatusCode,
};

/// Authorization service for ZK proof verification
pub struct AuthorizationService {
    nullifier_checker: NullifierChecker,
    payment_checker: PaymentChecker,
    config: Config,
}

impl AuthorizationService {
    /// Create a new authorization service
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let nullifier_checker = NullifierChecker::new(&config.redis_url)?;
        let payment_checker = PaymentChecker::new(&config.redis_url, config.payment.clone())?;

        Ok(Self {
            nullifier_checker,
            payment_checker,
            config,
        })
    }

    /// Convert this service into a tonic gRPC server
    pub fn into_service(self) -> AuthorizationServer<Self> {
        AuthorizationServer::new(self)
    }

    /// Verify a RISC Zero proof
    ///
    /// # Arguments
    /// * `receipt_hex` - Hex-encoded Receipt bytes
    ///
    /// # Returns
    /// * `Ok(nullifier)` - Proof verified successfully, returns the nullifier
    /// * `Err(Status)` - Verification failed
    async fn verify_proof(&self, receipt_hex: &str) -> Result<Nullifier, Status> {
        // Decode hex-encoded receipt bytes
        let receipt_bytes = hex::decode(receipt_hex)
            .map_err(|e| Status::invalid_argument(format!("Invalid receipt hex: {}", e)))?;

        // Deserialize Receipt
        let (receipt, _): (Receipt, usize) =
            bincode::serde::decode_from_slice(&receipt_bytes, bincode::config::standard())
                .map_err(|e| {
                    Status::invalid_argument(format!("Failed to deserialize receipt: {}", e))
                })?;

        // Verify proof and decode outputs in one step
        let outputs = receipt
            .verify_and_decode(&self.config.image_id)
            .map_err(|e| {
                tracing::warn!("Proof verification failed: {}", e);
                Status::permission_denied(format!("Proof verification failed: {}", e))
            })?;

        // Check compliance result
        if !outputs.compliance_result {
            tracing::warn!("Business logic validation failed");
            return Err(Status::permission_denied(
                "Business logic validation failed",
            ));
        }

        tracing::debug!(
            "Proof verified successfully, nullifier: {}",
            outputs.nullifier.to_hex()
        );

        Ok(outputs.nullifier)
    }
}

#[tonic::async_trait]
impl Authorization for AuthorizationService {
    /// Check authorization based on ZK proof and nullifier
    ///
    /// The verification flow is:
    /// 1. Check nullifier replay (fast, prevents wasted computation)
    /// 2. If payment required: verify payment exists and reserve it
    /// 3. Verify ZK proof (expensive)
    /// 4. Verify nullifier consistency between header and proof
    /// 5. Return success with nullifier in response metadata
    async fn check(
        &self,
        request: Request<CheckRequest>,
    ) -> Result<Response<CheckResponse>, Status> {
        let req = request.into_inner();

        tracing::debug!(
            "Received authorization check request for path: {:?}",
            req.path
        );

        // Extract x-zk-receipt header
        let receipt_hex = req.headers.get("x-zk-receipt").ok_or_else(|| {
            tracing::warn!("Missing x-zk-receipt header");
            Status::unauthenticated("Missing x-zk-receipt header")
        })?;

        // Extract x-zk-nullifier header
        let nullifier_hex = req.headers.get("x-zk-nullifier").ok_or_else(|| {
            tracing::warn!("Missing x-zk-nullifier header");
            Status::unauthenticated("Missing x-zk-nullifier header")
        })?;

        // Parse nullifier
        let nullifier = Nullifier::from_hex(nullifier_hex).map_err(|e| {
            tracing::warn!("Invalid nullifier format: {}", e);
            Status::invalid_argument(format!("Invalid nullifier format: {}", e))
        })?;

        // Check for replay attack (must do this BEFORE proof verification to save computation)
        let is_new = self
            .nullifier_checker
            .check_and_set(&nullifier)
            .await
            .map_err(|e| {
                tracing::error!("Redis error: {}", e);
                Status::unavailable(format!("Nullifier checker unavailable: {}", e))
            })?;

        if !is_new {
            tracing::warn!("Nullifier replay detected: {}", nullifier.to_hex());
            return Ok(Response::new(CheckResponse {
                status: StatusCode::Unauthenticated as i32,
                message: "Nullifier replay detected".to_string(),
                metadata: Default::default(),
            }));
        }

        // Check if payment verification is required
        let payment_reserved = if self.payment_checker.is_required() {
            tracing::debug!("Payment verification required for nullifier: {}", nullifier.to_hex());

            // Verify payment exists and meets requirements
            match self.payment_checker.check_payment(&nullifier).await {
                Ok(payment_info) => {
                    tracing::info!(
                        "Payment verified: {} zatoshis, tx: {}",
                        payment_info.amount,
                        payment_info.tx_id
                    );
                }
                Err(e) => {
                    tracing::warn!("Payment verification failed: {}", e);
                    return Ok(Response::new(CheckResponse {
                        status: StatusCode::PermissionDenied as i32,
                        message: format!("Payment verification failed: {}", e),
                        metadata: Default::default(),
                    }));
                }
            }

            // Reserve the payment with TTL
            if let Err(e) = self.payment_checker.reserve_payment(&nullifier).await {
                tracing::warn!("Failed to reserve payment: {}", e);
                return Ok(Response::new(CheckResponse {
                    status: StatusCode::PermissionDenied as i32,
                    message: format!("Failed to reserve payment: {}", e),
                    metadata: Default::default(),
                }));
            }

            true
        } else {
            false
        };

        // Verify the proof
        let verified_nullifier = match self.verify_proof(receipt_hex).await {
            Ok(n) => n,
            Err(status) => {
                // Proof verification failed - release payment reservation if we made one
                if payment_reserved {
                    if let Err(e) = self.payment_checker.release_reservation(&nullifier).await {
                        tracing::error!("Failed to release payment reservation: {}", e);
                    }
                }
                return Ok(Response::new(CheckResponse {
                    status: StatusCode::PermissionDenied as i32,
                    message: status.message().to_string(),
                    metadata: Default::default(),
                }));
            }
        };

        // Verify the nullifier from the proof matches the one in the header
        if verified_nullifier.0 != nullifier.0 {
            tracing::warn!("Nullifier mismatch: header != proof");
            // Release payment reservation if we made one
            if payment_reserved {
                if let Err(e) = self.payment_checker.release_reservation(&nullifier).await {
                    tracing::error!("Failed to release payment reservation: {}", e);
                }
            }
            return Ok(Response::new(CheckResponse {
                status: StatusCode::PermissionDenied as i32,
                message: "Nullifier mismatch between header and proof".to_string(),
                metadata: Default::default(),
            }));
        }

        // All checks passed - confirm payment usage
        if payment_reserved {
            if let Err(e) = self.payment_checker.confirm_payment(&nullifier).await {
                tracing::error!("Failed to confirm payment: {}", e);
                // Payment confirmation failure is not fatal - the reservation will expire
                // and the payment can be retried. Log the error but proceed.
            }
        }

        tracing::info!(
            "Authorization successful for nullifier: {}",
            nullifier.to_hex()
        );

        // Create response metadata with nullifier for downstream services
        let mut metadata = HashMap::new();
        metadata.insert(
            "x-payment-nullifier".to_string(),
            nullifier.to_hex(),
        );

        Ok(Response::new(CheckResponse {
            status: StatusCode::Ok as i32,
            message: "Proof verified successfully".to_string(),
            metadata,
        }))
    }
}
