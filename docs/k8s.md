# Kubernetes Deployment Guide (GCP/GKE)

This guide covers deploying the Khafi Gateway platform to Google Kubernetes Engine (GKE).

**Domain:** `khafi.misbar.tech` / `api.khafi.misbar.tech`

## Prerequisites

- GCP account with billing enabled
- `gcloud` CLI installed and configured
- `kubectl` installed
- Docker installed (for building images)

## Architecture

```
                              ┌─────────────────────────────┐
                              │      GCP Load Balancer      │
                              │   khafi.misbar.tech :443    │
                              └─────────────┬───────────────┘
                                            │
                              ┌─────────────▼───────────────┐
                              │         Traefik             │
                              │   (IngressController)       │
                              │   + Let's Encrypt TLS       │
                              └─────────────┬───────────────┘
                                            │
              ┌─────────────────────────────┼─────────────────────────────┐
              │                             │                             │
              ▼                             ▼                             ▼
    ┌─────────────────┐        ┌───────────────────┐        ┌──────────────────┐
    │    Frontend     │        │  Logic Compiler   │        │  Build Service   │
    │    (nginx)      │        │      API          │        │                  │
    │   Port 3000     │        │    Port 8082      │        │    Port 8085     │
    └─────────────────┘        └────────┬──────────┘        └────────┬─────────┘
                                        │                            │
                                        ▼                            ▼
                               ┌────────────────────────────────────────────┐
                               │           Image ID Registry                │
                               │               Port 8083                    │
                               └────────────────────┬───────────────────────┘
                                                    │
    ┌───────────────────────────────────────────────┼───────────────────────────┐
    │                                               │                           │
    ▼                                               ▼                           ▼
┌───────────────────┐                 ┌──────────────────┐          ┌─────────────────┐
│ Proof Generation  │                 │  ZK Verification │          │  Zcash Backend  │
│    Service        │                 │     Service      │          │                 │
│   Port 8084       │                 │   Port 50051     │          │   Port 8081     │
└─────────┬─────────┘                 └────────┬─────────┘          └────────┬────────┘
          │                                    │                             │
          └────────────────────────────────────┼─────────────────────────────┘
                                               │
                                               ▼
                                      ┌─────────────────┐
                                      │     Redis       │
                                      │  StatefulSet    │
                                      │   Port 6379     │
                                      └─────────────────┘
```

## Quick Start

### 1. Set Up GCP Project

```bash
# Set your project
export PROJECT_ID=your-gcp-project-id
gcloud config set project $PROJECT_ID

# Enable required APIs
gcloud services enable container.googleapis.com
gcloud services enable artifactregistry.googleapis.com
gcloud services enable compute.googleapis.com
```

### 2. Create GKE Cluster

```bash
# Create Autopilot cluster (recommended)
gcloud container clusters create-auto khafi-cluster \
  --region=us-central1 \
  --project=$PROJECT_ID

# Or create Standard cluster with more control
gcloud container clusters create khafi-cluster \
  --region=us-central1 \
  --num-nodes=3 \
  --machine-type=e2-standard-4 \
  --enable-autoscaling \
  --min-nodes=2 \
  --max-nodes=10 \
  --project=$PROJECT_ID

# Get credentials
gcloud container clusters get-credentials khafi-cluster \
  --region=us-central1 \
  --project=$PROJECT_ID
```

### 3. Create Artifact Registry

```bash
# Create repository
gcloud artifacts repositories create khafi \
  --repository-format=docker \
  --location=us \
  --project=$PROJECT_ID

# Configure Docker auth
gcloud auth configure-docker us-docker.pkg.dev
```

### 4. Build and Push Images

```bash
REGISTRY=us-docker.pkg.dev/$PROJECT_ID/khafi

# Build all images
docker build -t $REGISTRY/frontend:latest -f frontend/Dockerfile frontend/
docker build -t $REGISTRY/logic-compiler-api:latest -f crates/logic-compiler-api/Dockerfile .
docker build -t $REGISTRY/image-id-registry:latest -f crates/image-id-registry/Dockerfile .
docker build -t $REGISTRY/proof-generation-service:latest -f crates/proof-generation-service/Dockerfile .
docker build -t $REGISTRY/zk-verification-service:latest -f crates/zk-verification-service/Dockerfile .
docker build -t $REGISTRY/zcash-backend:latest -f crates/zcash-backend/Dockerfile .
docker build -t $REGISTRY/build-service:latest -f crates/build-service/Dockerfile .

# Push all images
for img in frontend logic-compiler-api image-id-registry proof-generation-service zk-verification-service zcash-backend build-service; do
  docker push $REGISTRY/$img:latest
done
```

### 5. Install Traefik CRDs

```bash
# Install Traefik Custom Resource Definitions
kubectl apply -f https://raw.githubusercontent.com/traefik/traefik/v3.2/docs/content/reference/dynamic-configuration/kubernetes-crd-definition-v1.yml
```

### 6. Configure Secrets

```bash
# Create namespace first
kubectl apply -f k8s/namespace.yaml

# Create secrets
kubectl create secret generic khafi-secrets \
  --namespace=khafi \
  --from-literal=zcash_payment_address='u1your_zcash_unified_address'

# Optional: Create Traefik dashboard auth
# Generate password: htpasswd -nb admin yourpassword
kubectl create secret generic traefik-dashboard-auth \
  --namespace=khafi \
  --from-literal=users='admin:$apr1$...'
```

### 7. Update Kustomization with Project ID

```bash
# Edit the GCP overlay to use your project ID
sed -i "s/PROJECT_ID/$PROJECT_ID/g" k8s/overlays/gcp/kustomization.yaml
```

### 8. Deploy

```bash
# Deploy everything
kubectl apply -k k8s/overlays/gcp

# Watch deployment progress
kubectl get pods -n khafi -w
```

### 9. Configure DNS

```bash
# Get the LoadBalancer external IP
kubectl get svc traefik -n khafi -o jsonpath='{.status.loadBalancer.ingress[0].ip}'

# Add DNS records in your DNS provider (e.g., Cloudflare, Google Cloud DNS):
# A record: khafi.misbar.tech -> <EXTERNAL_IP>
# A record: api.khafi.misbar.tech -> <EXTERNAL_IP>
# A record: traefik.khafi.misbar.tech -> <EXTERNAL_IP> (optional, for dashboard)
```

### 10. Verify Deployment

```bash
# Check all pods are running
kubectl get pods -n khafi

# Check services
kubectl get svc -n khafi

# Check IngressRoutes
kubectl get ingressroute -n khafi

# Test health endpoint (after DNS propagation)
curl https://api.khafi.misbar.tech/health
```

## Domain Configuration

| Subdomain | Purpose |
|-----------|---------|
| `khafi.misbar.tech` | Frontend web application |
| `api.khafi.misbar.tech` | Backend API services |
| `traefik.khafi.misbar.tech` | Traefik dashboard (optional) |

## Services

| Service | Internal Port | API Path |
|---------|--------------|----------|
| Frontend | 3000 | `/` (khafi.misbar.tech) |
| Logic Compiler API | 8082 | `/api/compile`, `/api/deploy`, `/api/validate` |
| Build Service | 8085 | `/api/build` |
| Proof Generation | 8084 | `/api/prove` |
| Image ID Registry | 8083 | `/api/registry` |
| Zcash Backend | 8081 | `/api/payment` |
| ZK Verification | 50051 (gRPC) | Internal only |
| Redis | 6379 | Internal only |

## Traefik Features

- **Automatic TLS:** Let's Encrypt certificates via HTTP challenge
- **HTTP to HTTPS redirect:** All HTTP traffic redirected to HTTPS
- **Rate limiting:** 100 req/min average, 200 burst on API endpoints
- **CORS:** Configured for frontend origin
- **Security headers:** XSS protection, frame deny, content-type nosniff

## Resource Requirements

| Service | Memory Req | Memory Limit | CPU Req | CPU Limit |
|---------|-----------|--------------|---------|-----------|
| Frontend | 64Mi | 128Mi | 50m | 100m |
| Logic Compiler API | 256Mi | 1Gi | 250m | 1000m |
| Image ID Registry | 64Mi | 256Mi | 50m | 250m |
| Proof Generation | 2Gi | 8Gi | 1000m | 4000m |
| Build Service | 4Gi | 16Gi | 2000m | 8000m |
| ZK Verification | 128Mi | 512Mi | 100m | 500m |
| Zcash Backend | 128Mi | 512Mi | 100m | 500m |
| Redis | 128Mi | 512Mi | 100m | 500m |
| Traefik | 64Mi | 256Mi | 100m | 500m |

## Scaling

```bash
# Manual scaling
kubectl scale deployment/frontend --replicas=5 -n khafi

# Horizontal Pod Autoscaler
kubectl autoscale deployment/logic-compiler-api \
  --min=2 --max=10 --cpu-percent=70 -n khafi
```

## Monitoring

### Logs

```bash
# All pods
kubectl logs -l app.kubernetes.io/part-of=khafi-gateway -n khafi

# Specific service
kubectl logs -f deployment/logic-compiler-api -n khafi

# Traefik access logs
kubectl logs -f deployment/traefik -n khafi
```

### Port Forward for Debugging

```bash
# Access service locally
kubectl port-forward svc/logic-compiler-api 8082:8082 -n khafi

# Traefik dashboard
kubectl port-forward svc/traefik-dashboard 8080:8080 -n khafi
```

## Troubleshooting

### Certificate Issues

```bash
# Check Traefik logs for ACME errors
kubectl logs deployment/traefik -n khafi | grep -i acme

# Verify certificate storage
kubectl exec deployment/traefik -n khafi -- cat /data/acme.json
```

### DNS Not Resolving

```bash
# Verify LoadBalancer has external IP
kubectl get svc traefik -n khafi

# If pending, check GCP quotas and firewall rules
gcloud compute addresses list
```

### Pod Crashes

```bash
# Check events
kubectl get events -n khafi --sort-by='.lastTimestamp'

# Describe pod
kubectl describe pod <pod-name> -n khafi

# Previous logs
kubectl logs <pod-name> -n khafi --previous
```

## Cost Optimization

1. **Use Autopilot** for automatic resource optimization
2. **Use preemptible VMs** for build-service (non-critical)
3. **Enable cluster autoscaler** to scale down when idle
4. **Use committed use discounts** for steady workloads

## File Structure

```
k8s/
├── namespace.yaml
├── configmap.yaml
├── secrets.yaml.template
├── ingress.yaml              # Traefik IngressRoutes
├── base/
│   └── kustomization.yaml
├── backend/
│   ├── kustomization.yaml
│   ├── redis.yaml
│   ├── image-id-registry.yaml
│   ├── logic-compiler-api.yaml
│   ├── proof-generation-service.yaml
│   ├── zk-verification-service.yaml
│   ├── zcash-backend.yaml
│   └── build-service.yaml
├── frontend/
│   ├── kustomization.yaml
│   └── deployment.yaml
├── traefik/
│   ├── kustomization.yaml
│   └── deployment.yaml
└── overlays/
    ├── dev/
    │   └── kustomization.yaml
    ├── prod/
    │   └── kustomization.yaml
    └── gcp/
        └── kustomization.yaml  # <-- Use this for GCP
```

## Quick Commands Reference

```bash
# Deploy to GCP
kubectl apply -k k8s/overlays/gcp

# Check status
kubectl get all -n khafi

# View logs
kubectl logs -f deployment/traefik -n khafi

# Restart deployment
kubectl rollout restart deployment/frontend -n khafi

# Delete everything
kubectl delete -k k8s/overlays/gcp
```
