# Northflank Platform Research Findings

> Research conducted: 2026-02-16
> Sources: Northflank official documentation, API reference, changelog, npm registry

---

## Table of Contents

1. [Platform Overview](#1-platform-overview)
2. [REST API Reference](#2-rest-api-reference)
3. [CLI Tool](#3-cli-tool)
4. [JavaScript Client](#4-javascript-client)
5. [Deployment Features](#5-deployment-features)
6. [Compute Plans](#6-compute-plans)
7. [GPU Support](#7-gpu-support)
8. [Storage and Volumes](#8-storage-and-volumes)
9. [Networking and Ports](#9-networking-and-ports)
10. [Health Checks](#10-health-checks)
11. [Auto-Scaling](#11-auto-scaling)
12. [Secrets and Environment Variables](#12-secrets-and-environment-variables)
13. [Managed Databases (Addons)](#13-managed-databases-addons)
14. [Infrastructure as Code](#14-infrastructure-as-code)
15. [Advanced Features](#15-advanced-features)
16. [Regions](#16-regions)
17. [Pricing](#17-pricing)
18. [Integration Considerations for Sindri](#18-integration-considerations-for-sindri)

---

## 1. Platform Overview

Northflank is a full-stack cloud platform (PaaS) that provides build, deploy, and manage capabilities for containerized applications. It runs on Kubernetes under the hood and supports both Northflank's managed cloud and Bring Your Own Cloud (BYOC) deployment models.

**Key differentiators:**

- Full-stack PaaS with managed Kubernetes
- Native GPU support (H100, B200, A100, L4, etc.)
- Built-in CI/CD pipeline
- Managed databases (PostgreSQL, MySQL, MongoDB, Redis, etc.)
- Infrastructure-as-Code with bidirectional GitOps
- BYOC support for AWS, GCP, Azure, OCI, Civo, CoreWeave, bare-metal
- 16+ managed cloud regions globally

---

## 2. REST API Reference

### Base URL

```
https://api.northflank.com/v1
```

### Authentication

- Uses JSON Web Tokens (JWT) / Bearer token authentication
- Token created in Northflank web UI under user or team account settings
- Header format: `Authorization: Bearer <NORTHFLANK_API_TOKEN>`

### Rate Limits

- Default: **1000 requests per hour**
- Resets one hour after the first request
- Higher limits available via support@northflank.com

### Request Format

- Content-Type: `application/json`
- All request bodies are JSON

### Response Format

- JSON responses
- Standard HTTP status codes

### Major API Resource Categories

| Resource Category  | Base Path                            | Key Operations                                                                         |
| ------------------ | ------------------------------------ | -------------------------------------------------------------------------------------- |
| **Projects**       | `/v1/projects`                       | Create/update, list, get, delete                                                       |
| **Services**       | `/v1/projects/{projectId}/services`  | Create (deployment/build/combined), get, update, delete, scale, pause, resume, restart |
| **Jobs**           | `/v1/projects/{projectId}/jobs`      | Create, get, update, delete, start build, scale                                        |
| **Addons**         | `/v1/projects/{projectId}/addons`    | Create, list, get, delete, backup                                                      |
| **Volumes**        | `/v1/projects/{projectId}/volumes`   | Create, get, backup, restore, attach, detach                                           |
| **Secrets**        | `/v1/projects/{projectId}/secrets`   | Create groups, manage variables, inject                                                |
| **Domains**        | `/v1/domains`                        | Register, assign to service/subdomain, import certificates                             |
| **Pipelines**      | `/v1/projects/{projectId}/pipelines` | Create, manage release flows                                                           |
| **Templates**      | `/v1/templates`                      | Create, run, manage IaC templates                                                      |
| **Logs & Metrics** | Various                              | Query logs, configure sinks                                                            |

### Key API Endpoints

#### Create Project

```
POST /v1/projects
```

```json
{
  "name": "my-project",
  "description": "Project description",
  "region": "us-east",
  "color": "#00FF00"
}
```

Note: Region cannot be changed after creation.

#### Create Deployment Service (External Image)

```
POST /v1/projects/{projectId}/services/deployment
```

```json
{
  "name": "my-service",
  "description": "Service description",
  "billing": {
    "deploymentPlan": "nf-compute-200"
  },
  "deployment": {
    "instances": 1,
    "external": {
      "imagePath": "nginx:latest",
      "registryProvider": "dockerhub",
      "privateImage": false
    }
  },
  "ports": [
    {
      "name": "http",
      "internalPort": 80,
      "public": true,
      "protocol": "HTTP"
    }
  ]
}
```

For private registries, add credentials:

```json
{
  "deployment": {
    "external": {
      "imagePath": "registry.example.com/myapp:latest",
      "credentials": "credential-id",
      "privateImage": true
    }
  }
}
```

#### Create Combined Service (Build + Deploy from Git)

```
POST /v1/projects/{projectId}/services/combined
```

```json
{
  "name": "my-app",
  "description": "Build and deploy service",
  "billing": {
    "deploymentPlan": "nf-compute-100-2"
  },
  "vcsData": {
    "projectUrl": "https://github.com/user/repo",
    "projectType": "github",
    "projectBranch": "main"
  },
  "buildConfiguration": {
    "dockerfile": "./Dockerfile",
    "buildContext": "/"
  },
  "deployment": {
    "instances": 1
  },
  "ports": [
    {
      "name": "http",
      "internalPort": 3000,
      "public": true,
      "protocol": "HTTP"
    }
  ]
}
```

#### Update Service Deployment

```
PATCH /v1/projects/{projectId}/services/{serviceId}/deployment
```

#### Scale Service

```
POST /v1/projects/{projectId}/services/{serviceId}/scale
```

```json
{
  "instances": 3,
  "deploymentPlan": "nf-compute-200"
}
```

#### Get Service

```
GET /v1/projects/{projectId}/services/{serviceId}
```

#### List Services

```
GET /v1/projects/{projectId}/services
```

#### List Addons

```
GET /v1/projects/{projectId}/addons
```

#### Volume Backup

```
POST /v1/projects/{projectId}/volumes/{volumeId}/backups
```

#### Get Volume Backups

```
GET /v1/projects/{projectId}/volumes/{volumeId}/backups
```

#### Get Job Runtime Environment

```
GET /v1/projects/{projectId}/jobs/{jobId}/runtime-environment
```

Parameters: `show` = `this` | `inherited` | `all`

### Example cURL Request

```bash
curl -X POST \
  https://api.northflank.com/v1/projects/{projectId}/services/deployment \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${NORTHFLANK_API_TOKEN}" \
  -d '{
    "name": "my-service",
    "billing": { "deploymentPlan": "nf-compute-200" },
    "deployment": {
      "instances": 1,
      "external": {
        "imagePath": "nginx:latest",
        "registryProvider": "dockerhub"
      }
    },
    "ports": [
      { "name": "http", "internalPort": 80, "public": true, "protocol": "HTTP" }
    ]
  }'
```

---

## 3. CLI Tool

### Package

- npm package: `@northflank/cli`
- Latest version: **0.10.15** (as of Feb 2026)
- Node.js based CLI

### Installation

```bash
# Global install via npm
npm i @northflank/cli -g

# Global install via yarn
yarn global add @northflank/cli

# Run without installation (npx)
npx @northflank/cli
```

### Authentication

```bash
# Interactive login (opens browser)
northflank login

# Login with token directly
northflank login --token <NORTHFLANK_API_TOKEN>
```

### Available Commands

| Command               | Description                                                  |
| --------------------- | ------------------------------------------------------------ |
| `northflank login`    | Connect CLI to your Northflank account                       |
| `northflank context`  | Retrieve and update local settings (default project, etc.)   |
| `northflank contexts` | List all saved contexts                                      |
| `northflank list`     | List Northflank resources (projects, services, jobs, addons) |
| `northflank create`   | Create resources (projects, services, jobs)                  |
| `northflank get`      | Get detailed information about resources                     |
| `northflank delete`   | Delete resources                                             |
| `northflank scale`    | Scale resources (instances, plans)                           |
| `northflank update`   | Update resource properties                                   |
| `northflank restart`  | Restart resources                                            |
| `northflank pause`    | Pause resources                                              |
| `northflank resume`   | Resume paused resources                                      |
| `northflank forward`  | Port-forwarding for services and addons                      |
| `northflank exec`     | Open a shell in a container                                  |
| `northflank help`     | Display help information                                     |

### CLI Usage Examples

```bash
# List all projects
northflank list projects

# Create a deployment service
northflank create service deployment \
  --project my-project \
  --name my-service \
  --image nginx:latest \
  --plan nf-compute-200

# Get service details
northflank get service details \
  --project my-project \
  --service my-service

# Scale a service
northflank scale service \
  --project my-project \
  --service my-service \
  --instances 3

# Pause a service
northflank pause service \
  --project my-project \
  --service my-service

# Resume a service
northflank resume service \
  --project my-project \
  --service my-service

# Port-forward to a service
northflank forward service \
  --project my-project \
  --service my-service

# Forward all (proxy remote databases/services locally)
northflank forward all

# Exec into a container
northflank exec \
  --project my-project \
  --service my-service

# List volumes
northflank list volumes \
  --project my-project

# Restart a service
northflank restart service \
  --project my-project \
  --service my-service
```

### CLI Features

- Fully interactive mode (asks for parameters if not provided)
- Direct parameter passing for scripting/automation
- Port-forwarding without public exposure
- Container exec/terminal access
- RBAC-aware (respects team permissions)
- Context management for switching between projects/teams

---

## 4. JavaScript Client

### Package

- npm package: `@northflank/js-client`

### Installation

```bash
npm i @northflank/js-client
# or
yarn add @northflank/js-client
```

### Usage Example

```javascript
const { ApiClient, ApiClientInMemoryContextProvider } = require("@northflank/js-client");

const contextProvider = new ApiClientInMemoryContextProvider();
await contextProvider.addContext({
  name: "default",
  token: process.env.NORTHFLANK_API_TOKEN,
});

const client = new ApiClient(contextProvider);

// Create a deployment service
const result = await client.create.service.deployment({
  parameters: {
    projectId: "my-project",
  },
  data: {
    name: "my-service",
    description: "My deployment",
    billing: {
      deploymentPlan: "nf-compute-200",
    },
    deployment: {
      instances: 1,
      external: {
        imagePath: "nginx:latest",
        registryProvider: "dockerhub",
      },
    },
    ports: [
      {
        name: "http",
        internalPort: 80,
        public: true,
        protocol: "HTTP",
      },
    ],
  },
});

// Scale a service
await client.scale.service({
  parameters: {
    projectId: "my-project",
    serviceId: "my-service",
  },
  data: {
    instances: 3,
    deploymentPlan: "nf-compute-200",
  },
});
```

---

## 5. Deployment Features

### Service Types

Northflank offers four main workload types:

| Service Type           | Description                                             | CI/CD          | Use Case                                   |
| ---------------------- | ------------------------------------------------------- | -------------- | ------------------------------------------ |
| **Deployment Service** | Runs a container image (external or from build service) | CD only        | Run pre-built images from any registry     |
| **Build Service**      | Builds images from Git repositories                     | CI only        | Build once, deploy to multiple services    |
| **Combined Service**   | Build + Deploy in one service                           | CI + CD        | Simple build-and-deploy from Git           |
| **Job**                | Ephemeral/scheduled workloads                           | Optional CI/CD | Cron jobs, one-off tasks, batch processing |

### External Image Deployment

- Deploy from any public or private Docker registry
- Supported registries: DockerHub, GHCR, ECR, Google Artifact Registry, Azure ACR, and any registry with `.dockerconfig`
- Registry credentials can be saved and referenced by ID
- Auto-deployment on image tag updates (when CD enabled)

### Build + Deploy (Combined Service)

- Build from Dockerfile or Buildpacks
- Linked to Git repositories (GitHub, GitLab, Bitbucket)
- Automatic builds on push to configured branch
- BuildKit-based builds with layer caching
- Can push built images to your own private registry

### Container Registry Integration

- Push Northflank container builds directly to your own private registry
- Support for all major registries

---

## 6. Compute Plans

### CPU-Based Compute Plans

| Plan Name            | vCPU | Memory (MB) | Hourly Rate | Monthly Rate |
| -------------------- | ---- | ----------- | ----------- | ------------ |
| `nf-compute-10`      | 0.1  | 256         | $0.004      | $2.70        |
| `nf-compute-20`      | 0.2  | 512         | $0.008      | $5.40        |
| `nf-compute-50`      | 0.5  | 1024        | $0.017      | $12.00       |
| `nf-compute-100-1`   | 1.0  | 1024        | $0.025      | $18.00       |
| `nf-compute-100-2`   | 1.0  | 2048        | $0.033      | $24.00       |
| `nf-compute-100-4`   | 1.0  | 4096        | $0.050      | $36.00       |
| `nf-compute-200`     | 2.0  | 4096        | $0.067      | $48.00       |
| `nf-compute-200-8`   | 2.0  | 8192        | $0.100      | $72.00       |
| `nf-compute-200-16`  | 2.0  | 16384       | $0.167      | $120.00      |
| `nf-compute-400`     | 4.0  | 8192        | $0.133      | $96.00       |
| `nf-compute-400-16`  | 4.0  | 16384       | $0.200      | $144.00      |
| `nf-compute-800-8`   | 8.0  | 8192        | $0.200      | $144.00      |
| `nf-compute-800-16`  | 8.0  | 16384       | $0.267      | $192.00      |
| `nf-compute-800-24`  | 8.0  | 24576       | $0.333      | $240.00      |
| `nf-compute-800-32`  | 8.0  | 32768       | $0.400      | $288.00      |
| `nf-compute-800-40`  | 8.0  | 40960       | $0.467      | $336.00      |
| `nf-compute-1200-24` | 12.0 | 24576       | $0.400      | $288.00      |
| `nf-compute-1600-32` | 16.0 | 32768       | $0.533      | $384.00      |
| `nf-compute-2000-40` | 20.0 | 40960       | $0.667      | $480.00      |

### Plan Naming Convention

Format: `nf-compute-{vCPU*100}[-{memoryMB/1024}]`

- `nf-compute-200` = 2 vCPU, default memory (4096 MB)
- `nf-compute-200-8` = 2 vCPU, 8192 MB memory
- When memory suffix omitted, uses default ratio

### Resource Range

- CPU: 0.1 vCPU to 20+ vCPU (up to 32 vCPU documented)
- Memory: 256 MB to 40960 MB (up to 256 GB)

---

## 7. GPU Support

### Available GPU Types

| GPU Model            | VRAM   | Hourly Rate  | Availability                              |
| -------------------- | ------ | ------------ | ----------------------------------------- |
| **NVIDIA L4**        | 24 GB  | N/A (varies) | Asia-NE, Asia-SE, Europe-West, US regions |
| **NVIDIA A100 40GB** | 40 GB  | $1.42/hr     | Select regions                            |
| **NVIDIA A100 80GB** | 80 GB  | $1.76/hr     | Select regions                            |
| **NVIDIA H100**      | 80 GB  | $2.74/hr     | Broad availability (6+ regions)           |
| **NVIDIA H200**      | 141 GB | N/A (varies) | EU-West-NL, US-Central, US-East, US-West  |
| **NVIDIA B200**      | 180 GB | $5.87/hr     | Asia-NE, Asia-SE, EU-West-NL, US-East     |
| **AMD MI300X**       | N/A    | N/A          | Available (details TBC)                   |
| **Habana Gaudi**     | N/A    | N/A          | Available (details TBC)                   |

### GPU Features (as of August 2025 release)

- Native GPU workload support in PaaS
- Fractional GPU allocation supported
- GPU stack templates for quick deployment
- Workload scheduling optimized for GPU
- Region/cluster filtering based on GPU availability
- GPU usage requires pre-purchased credits
- Automated spot GPU orchestration (up to 90% cost reduction)

---

## 8. Storage and Volumes

### Persistent Volumes

- SSD-backed persistent storage
- Configurable size (up to 1.5 TB as of August 2025)
- Multiple volumes can be attached to the same service
- Volumes can have multiple container mount locations
- Volumes can be detached/attached and moved between services
- Available for deployment and combined services

### Volume Configuration

```json
{
  "name": "my-volume",
  "size": 10,
  "mountPaths": [
    {
      "containerMountPath": "/data",
      "volumeMountPath": "/"
    }
  ]
}
```

### Volume Operations (API)

| Operation     | Endpoint                                                   |
| ------------- | ---------------------------------------------------------- |
| Create volume | `POST /v1/projects/{projectId}/volumes`                    |
| Backup volume | `POST /v1/projects/{projectId}/volumes/{volumeId}/backups` |
| Get backups   | `GET /v1/projects/{projectId}/volumes/{volumeId}/backups`  |
| Attach volume | Via UI/CLI (attach to service)                             |
| Detach volume | Via UI/CLI (detach from service)                           |

### Limitations

- Adding a volume limits the service to **1 instance**
- During restarts with a volume, the running container is always terminated before the new one starts (regardless of health check settings)

---

## 9. Networking and Ports

### Port Configuration

```json
{
  "ports": [
    {
      "name": "http",
      "internalPort": 8080,
      "public": true,
      "protocol": "HTTP"
    },
    {
      "name": "grpc",
      "internalPort": 50051,
      "public": false,
      "protocol": "TCP"
    }
  ]
}
```

### Supported Protocols

- HTTP / HTTPS (automatic TLS via Let's Encrypt)
- TCP
- UDP

### Endpoint Types

- **Public endpoints**: Accessible from the internet, auto-provisioned TLS
- **Private endpoints**: Only accessible within the Northflank project (inter-service communication)
- **Multi-project networking**: Enable cross-project private communication via Advanced options

### Custom Domains

- Register custom domains
- Assign to services/subdomains
- Path-based routing supported
- Wildcard domains supported (BYOC)
- Automatic TLS certificates via Let's Encrypt
- Import your own domain certificates

### Domain API Example

```bash
curl -X POST \
  https://api.northflank.com/v1/domains/{domain}/subdomains/{subdomain}/assign \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${NORTHFLANK_API_TOKEN}" \
  -d '{
    "serviceId": "my-service",
    "projectId": "my-project",
    "portName": "http"
  }'
```

---

## 10. Health Checks

### Probe Types

| Probe Type    | Purpose                                                                         |
| ------------- | ------------------------------------------------------------------------------- |
| **Liveness**  | Checks if container is alive; restarts if failing                               |
| **Readiness** | Checks if container is ready for traffic; removes from load balancer if failing |
| **Startup**   | Delays liveness/readiness probes until startup succeeds; for slow-starting apps |

### Check Methods

| Method   | Description                      | Configuration                          |
| -------- | -------------------------------- | -------------------------------------- |
| **HTTP** | HTTP GET to an endpoint          | Path, port, protocol                   |
| **TCP**  | TCP connection check             | Port                                   |
| **CMD**  | Execute command inside container | Command string (passes if exit code 0) |

### Configuration Fields

```json
{
  "healthChecks": {
    "liveness": {
      "protocol": "HTTP",
      "path": "/healthz",
      "port": 80,
      "initialDelaySeconds": 10,
      "periodSeconds": 15,
      "timeoutSeconds": 5,
      "failureThreshold": 3,
      "successThreshold": 1
    },
    "readiness": {
      "protocol": "TCP",
      "port": 80,
      "initialDelaySeconds": 10,
      "periodSeconds": 10,
      "timeoutSeconds": 5,
      "failureThreshold": 3,
      "successThreshold": 1
    },
    "startup": {
      "protocol": "CMD",
      "command": "curl -f http://localhost/ready || exit 1",
      "initialDelaySeconds": 0,
      "periodSeconds": 10,
      "failureThreshold": 30,
      "successThreshold": 1
    }
  }
}
```

### Health Check Behavior

- Health checks ensure traffic only routes to healthy containers
- Unhealthy containers are automatically replaced
- Enables zero-downtime deployments (readiness gates)
- Startup probes delay liveness/readiness until app is initialized

---

## 11. Auto-Scaling

### Horizontal Auto-Scaling

- Scale based on CPU and/or memory utilization thresholds
- Metrics checked every **15 seconds**
- Scale-up: Immediate when thresholds exceeded
- Scale-down: **5-minute cooldown window** to prevent flapping

### Configuration

```json
{
  "autoscaling": {
    "enabled": true,
    "minInstances": 1,
    "maxInstances": 10,
    "metrics": {
      "cpu": {
        "targetPercentage": 70
      },
      "memory": {
        "targetPercentage": 80
      }
    }
  }
}
```

### Manual Scaling

- Scale instances from 0 to N via UI, CLI, or API
- Scale to 0 instances makes service unavailable but preserves configuration
- CI/CD continues to build when scaled to 0; deploys when scaled back up

### CLI Scaling

```bash
northflank scale service \
  --project my-project \
  --service my-service \
  --instances 5 \
  --plan nf-compute-400
```

---

## 12. Secrets and Environment Variables

### Variable Types

| Type                  | Scope                                  | Injected At             |
| --------------------- | -------------------------------------- | ----------------------- |
| **Build Arguments**   | Individual service/job or secret group | Build time              |
| **Runtime Variables** | Individual service/job or secret group | Runtime (env vars)      |
| **Secret Files**      | ConfigMaps and secret files            | Runtime (mounted files) |

### Secret Groups

- Collections of runtime variables and/or build arguments
- Inherited by services and jobs in a project
- Can be restricted to specific services/jobs
- Direct service/job variables override group variables with the same name

### Database Secrets

- Database connection strings auto-generated and injectable
- Connect database addon secrets directly to workloads

### API Parameters

- `show=this`: Only secrets saved directly to the entity
- `show=inherited`: Only secrets inherited from linked secret groups
- Default: Returns both

---

## 13. Managed Databases (Addons)

### Supported Database Types

| Database       | Type              | HA Support |
| -------------- | ----------------- | ---------- |
| **PostgreSQL** | Relational        | Yes        |
| **MySQL**      | Relational        | Yes        |
| **MongoDB**    | Document          | Yes        |
| **Redis**      | Key-Value / Cache | Yes        |
| **MinIO**      | Object Storage    | Yes        |
| **RabbitMQ**   | Message Queue     | Yes        |

### Features

- Automated management: logs, metrics, backups, restores
- High availability configuration
- Public or private networking
- Free TLS for public endpoints via Let's Encrypt
- Disk size up to 1.5 TB (as of August 2025)
- Read replicas supported
- PostgreSQL extensions: h3-pg, pg_partman, and more
- BYOA (Bring Your Own Addon) for custom addon types

---

## 14. Infrastructure as Code

### Templates

- JSON-based template format
- Define entire stacks (projects, services, addons, secrets, etc.)
- Dynamic arguments and functions
- References between resources in templates
- JSON Schema support for IDE hints and validation
- Template runs triggered via UI, API, CLI, or Git push

### GitOps

- Bidirectional GitOps support
- Changes in Northflank UI push to Git repository
- Changes in Git repository apply to Northflank
- Linked to specific branch in repository
- Supports GitHub, GitLab, Bitbucket

### GitHub Actions Integration

- Official GitHub Actions support
- Trigger template runs from CI/CD pipelines

---

## 15. Advanced Features

### Pause / Resume

- Pause running services to stop resource consumption
- Resume to restart with same configuration
- Available via UI, CLI, and API

```bash
# CLI
northflank pause service --project my-project --service my-service
northflank resume service --project my-project --service my-service
```

### BYOC (Bring Your Own Cloud)

- Supported clouds: AWS, GCP, Azure, OCI, Civo, CoreWeave
- On-premises and bare-metal support
- 600+ regions available through BYOC
- Resources deployed within your VPC
- Configurable security groups and network policies
- AWS: Public subnets need internet gateway, private need NAT gateway
- Azure: Custom vnet subnets and Cilium overlay mode support

### Multi-Region Deployments

- Create projects in different regions
- Multi-project networking for cross-region communication
- Each project operates independently in its region
- Configure in Advanced options

### AI Copilot (August 2025+)

- Built-in AI assistant for platform questions
- Helps with usage patterns and platform primitives

### Container Exec / Terminal

- Interactive shell access to running containers
- Available via UI, CLI, and API

```bash
northflank exec --project my-project --service my-service
```

### Port Forwarding

- Forward remote services/databases to local machine
- No public exposure required

```bash
northflank forward service --project my-project --service my-service
northflank forward all  # Forward all in current context
```

### Data Transfer

- Transfer data to and from containers directly

### Logging

- Built-in log aggregation
- Hosted Loki for log storage
- Supports S3 and GCP storage backends
- Configure custom log sinks (HTTP endpoints)

### RBAC

- Role-based access control for teams
- API roles with granular permissions
- Project-level permission scoping

---

## 16. Regions

### Northflank Managed Cloud Regions (16 regions)

| Region Slug               | Location                                |
| ------------------------- | --------------------------------------- |
| `africa-south`            | Africa - South                          |
| `asia-east`               | Asia - East                             |
| `asia-northeast`          | Asia - Northeast                        |
| `asia-southeast`          | Asia - Southeast                        |
| `australia-southeast`     | Australia - Southeast                   |
| `canada-central`          | Canada - Central                        |
| `europe-west`             | Europe - West (London)                  |
| `europe-west-frankfurt`   | Europe - West - Frankfurt               |
| `europe-west-netherlands` | Europe - West - Netherlands (Amsterdam) |
| `europe-west-zurich`      | Europe - West - Zurich                  |
| `southamerica-east`       | South America - East                    |
| `us-central`              | US - Central                            |
| `us-east`                 | US - East                               |
| `us-east-ohio`            | US - East - Ohio                        |
| `us-west`                 | US - West                               |
| `us-west-california`      | US - West - California                  |

### BYOC Providers and Regions

| Provider           | Region Count |
| ------------------ | ------------ |
| AWS                | 25+          |
| GCP                | 35+          |
| Azure              | 60+          |
| OCI (Oracle)       | Available    |
| Civo               | Available    |
| CoreWeave          | Available    |
| Bare-metal/On-prem | Custom       |

---

## 17. Pricing

### Tiers

| Tier                             | Description                                       |
| -------------------------------- | ------------------------------------------------- |
| **Free Sandbox**                 | Limited resources for testing                     |
| **Self-Service (Pay-as-you-go)** | Usage-based billing, no commitment                |
| **Enterprise**                   | Custom SLAs, white-label, dedicated support, BYOC |

### Billing Model

- Transparent, usage-based billing
- Per-second billing for compute
- No unexpected costs; scale horizontally and vertically
- GPU usage requires pre-purchased credits

### Volume/Storage Pricing

- SSD-backed volumes
- Addon disk up to 1.5 TB
- Specific per-GB pricing available on pricing page

---

## 18. Integration Considerations for Sindri

### API Integration Pattern

For Sindri's Northflank adapter, the recommended integration approach:

1. **Authentication**: Store Northflank API token in Sindri secrets/config
2. **Project Management**: Create/manage Northflank projects per Sindri deployment
3. **Service Deployment**: Use deployment service API for external images (most common pattern)
4. **Resource Configuration**: Map Sindri resource tiers to `nf-compute-*` plan names
5. **Health Checks**: Configure liveness/readiness probes matching application requirements
6. **Networking**: Configure ports as public/private based on service type
7. **Environment Variables**: Use runtime variables API for secrets injection
8. **Scaling**: Implement auto-scaling rules or manual scaling via API
9. **Monitoring**: Use logs/metrics API for observability

### Key API Calls for Adapter

```
POST /v1/projects                                    # Create project
POST /v1/projects/{id}/services/deployment           # Deploy service
PATCH /v1/projects/{id}/services/{sid}/deployment     # Update deployment
POST /v1/projects/{id}/services/{sid}/scale           # Scale service
GET  /v1/projects/{id}/services/{sid}                 # Get service status
POST /v1/projects/{id}/services/{sid}/pause           # Pause service
POST /v1/projects/{id}/services/{sid}/resume          # Resume service
POST /v1/projects/{id}/services/{sid}/restart         # Restart service
DELETE /v1/projects/{id}/services/{sid}               # Delete service
GET  /v1/projects/{id}/services                       # List services
```

### Configuration Mapping (sindri.yaml)

```yaml
provider: northflank
northflank:
  region: us-east
  project_name: sindri-deployment
  service:
    name: my-app
    type: deployment # deployment | combined | build
    plan: nf-compute-200 # compute plan
    instances: 1
    image: registry/myapp:latest
    registry_credentials: credential-id # optional for private registries
    ports:
      - name: http
        internal_port: 8080
        public: true
        protocol: HTTP
    health_checks:
      liveness:
        protocol: HTTP
        path: /healthz
        port: 8080
        initial_delay: 10
      readiness:
        protocol: TCP
        port: 8080
        initial_delay: 5
    autoscaling:
      enabled: true
      min_instances: 1
      max_instances: 10
      cpu_threshold: 70
      memory_threshold: 80
    volumes:
      - name: data
        size: 10
        mount_path: /data
    environment:
      NODE_ENV: production
      PORT: "8080"
  gpu:
    type: H100 # L4, A100-40, A100-80, H100, H200, B200
    count: 1
```

### Error Handling

- Rate limit: 1000 req/hr -- implement backoff/retry
- Region immutability: Validate region before project creation
- Volume single-instance constraint: Warn users when volumes + scaling conflict
- GPU availability: Check region GPU support before deployment

---

## Sources

- [Northflank Documentation](https://northflank.com/docs/)
- [Northflank API Reference](https://northflank.com/docs/v1/api/introduction)
- [Northflank API - Use the API](https://northflank.com/docs/v1/api/use-the-api)
- [Northflank API - Create Deployment Service](https://northflank.com/docs/v1/api/services/create-deployment-service)
- [Northflank API - Create Combined Service](https://northflank.com/docs/v1/api/services/create-combined-service)
- [Northflank API - Update Service Deployment](https://northflank.com/docs/v1/api/services/update-service-deployment)
- [Northflank API - Use the CLI](https://northflank.com/docs/v1/api/use-the-cli)
- [Northflank API - Use the JavaScript Client](https://northflank.com/docs/v1/api/use-the-javascript-client)
- [@northflank/cli npm package](https://www.npmjs.com/package/@northflank/cli)
- [@northflank/js-client npm package](https://www.npmjs.com/package/@northflank/js-client)
- [Northflank Cloud Providers](https://northflank.com/cloud/northflank)
- [Northflank Regions](https://northflank.com/cloud/northflank/regions)
- [Northflank Pricing](https://northflank.com/pricing)
- [Northflank Features - Run](https://northflank.com/features/run)
- [Northflank Features - BYOC](https://northflank.com/features/bring-your-own-cloud)
- [Northflank Features - Databases](https://northflank.com/features/databases)
- [Northflank Features - Templates](https://northflank.com/features/templates)
- [Northflank Autoscale Deployments](https://northflank.com/docs/v1/application/scale/autoscale-deployments)
- [Northflank Scale Instances](https://northflank.com/docs/v1/application/scale/scale-instances)
- [Northflank Health Checks](https://northflank.com/docs/v1/application/observe/configure-health-checks)
- [Northflank Volumes](https://northflank.com/docs/v1/application/databases-and-persistence/add-a-volume)
- [Northflank Secrets](https://northflank.com/docs/v1/application/secure/inject-secrets)
- [Northflank IaC](https://northflank.com/docs/v1/application/infrastructure-as-code/infrastructure-as-code)
- [Northflank GitOps](https://northflank.com/docs/v1/application/infrastructure-as-code/gitops-on-northflank)
- [Northflank August 2025 Release](https://northflank.com/changelog/platform-august-2025-release)
- [Northflank CLI Changelog](https://northflank.com/changelog/northflank-cli-extended-api)
