# Khafi Gateway Kubernetes Deployment

Kubernetes manifests for deploying the Khafi Gateway platform to GKE with Traefik ingress.

**Domain:** `khafi.misbar.tech` / `api.khafi.misbar.tech`

## Quick Start (GCP)

```bash
# 1. Set project
export PROJECT_ID=your-gcp-project-id

# 2. Create GKE cluster
gcloud container clusters create-auto khafi-cluster \
  --region=us-central1 --project=$PROJECT_ID

# 3. Get credentials
gcloud container clusters get-credentials khafi-cluster \
  --region=us-central1 --project=$PROJECT_ID

# 4. Install Traefik CRDs
kubectl apply -f https://raw.githubusercontent.com/traefik/traefik/v3.2/docs/content/reference/dynamic-configuration/kubernetes-crd-definition-v1.yml

# 5. Create namespace and secrets
kubectl apply -f k8s/namespace.yaml
kubectl create secret generic khafi-secrets \
  --namespace=khafi \
  --from-literal=zcash_payment_address='YOUR_ADDRESS'

# 6. Update PROJECT_ID in overlay
sed -i "s/PROJECT_ID/$PROJECT_ID/g" k8s/overlays/gcp/kustomization.yaml

# 7. Build & push images, then deploy
kubectl apply -k k8s/overlays/gcp

# 8. Get LoadBalancer IP and configure DNS
kubectl get svc traefik -n khafi
```

See [docs/k8s.md](../docs/k8s.md) for detailed instructions.

## Directory Structure

```
k8s/
├── namespace.yaml              # Namespace: khafi
├── configmap.yaml              # Shared configuration
├── secrets.yaml.template       # Secrets template
├── ingress.yaml                # Traefik IngressRoutes + Middlewares
├── base/
│   └── kustomization.yaml      # Base kustomization
├── backend/
│   ├── kustomization.yaml
│   ├── redis.yaml              # StatefulSet with PVC
│   ├── image-id-registry.yaml  # Port 8083
│   ├── logic-compiler-api.yaml # Port 8082
│   ├── proof-generation-service.yaml # Port 8084
│   ├── zk-verification-service.yaml  # Port 50051 (gRPC)
│   ├── zcash-backend.yaml      # Port 8081
│   └── build-service.yaml      # Port 8085
├── frontend/
│   ├── kustomization.yaml
│   └── deployment.yaml         # Port 3000
├── traefik/
│   ├── kustomization.yaml
│   └── deployment.yaml         # Traefik + Let's Encrypt
└── overlays/
    ├── dev/                    # Local development
    ├── prod/                   # Generic production
    └── gcp/                    # GCP/GKE specific
```

## Overlays

| Overlay | Use Case |
|---------|----------|
| `k8s/overlays/dev` | Local development (minikube, kind) |
| `k8s/overlays/prod` | Generic production |
| `k8s/overlays/gcp` | GCP/GKE with Artifact Registry |

## Services

| Service | Port | Protocol | Exposed Path |
|---------|------|----------|--------------|
| Frontend | 3000 | HTTP | `khafi.misbar.tech/` |
| Logic Compiler API | 8082 | HTTP | `/api/compile`, `/api/deploy`, `/api/validate` |
| Build Service | 8085 | HTTP | `/api/build` |
| Proof Generation | 8084 | HTTP | `/api/prove` |
| Image ID Registry | 8083 | HTTP | `/api/registry` |
| Zcash Backend | 8081 | HTTP | `/api/payment` |
| ZK Verification | 50051 | gRPC | Internal |
| Redis | 6379 | TCP | Internal |

## Commands

```bash
# Deploy
kubectl apply -k k8s/overlays/gcp

# Check status
kubectl get pods -n khafi
kubectl get svc -n khafi
kubectl get ingressroute -n khafi

# Logs
kubectl logs -f deployment/traefik -n khafi
kubectl logs -f deployment/logic-compiler-api -n khafi

# Restart
kubectl rollout restart deployment/frontend -n khafi

# Delete
kubectl delete -k k8s/overlays/gcp
```
