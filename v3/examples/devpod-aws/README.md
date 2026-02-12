# DevPod AWS Example

Remote dev environment on AWS EC2 managed by DevPod.

## Usage

```bash
sindri init --from examples/devpod-aws
sindri deploy
```

## What This Configures

- DevPod provider backed by AWS EC2 (`c5.xlarge` spot instance)
- 8 GB memory, 4 CPUs, 50 GB disk in `us-west-2`
- `systems` profile (Rust, Go, Docker, infra-tools) plus cloud-tools
- AWS credentials injected from environment variables
- SSH key mounted into the container

## Prerequisites

- DevPod CLI installed
- AWS credentials configured (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
- Container registry accessible for image push (e.g., `ghcr.io`)
