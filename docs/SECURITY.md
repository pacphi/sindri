# Security Best Practices

Security guidelines for deploying and using Sindri.

## Security Model

Sindri's security model is based on:

1. **Isolated Environments** - Each deployment is isolated
2. **SSH-Based Access** - No password authentication
3. **Secrets Management** - Provider-specific secret handling
4. **User Separation** - Non-root developer user in containers
5. **Network Isolation** - Provider network policies
6. **SBOM Tracking** - Software bill of materials for auditing

## Authentication & Access

### SSH Key Management

**Best Practices:**

1. **Use Ed25519 keys:**

   ```bash
   ssh-keygen -t ed25519 -C "your@email.com"
   ```

2. **Use passphrase protection:**

   ```bash
   ssh-keygen -t ed25519 -C "your@email.com"
   # Enter passphrase when prompted
   ```

3. **Use ssh-agent:**

   ```bash
   eval "$(ssh-agent -s)"
   ssh-add ~/.ssh/id_ed25519
   ```

4. **Rotate keys regularly:**

   ```bash
   # Generate new key
   ssh-keygen -t ed25519 -C "your@email.com" -f ~/.ssh/id_ed25519_new

   # Update Fly.io
   flyctl ssh issue --agent -a my-app

   # Remove old key
   rm ~/.ssh/id_ed25519_old*
   ```

### Access Control

**Fly.io:**

```bash
# List authorized users
flyctl ssh issue --help

# Revoke access
# Remove user from Fly.io organization
```

**Kubernetes:**

```bash
# Use RBAC for access control
kubectl create rolebinding dev-access \
  --clusterrole=edit \
  --user=developer \
  --namespace=dev-envs
```

## Secrets Management

### Never Commit Secrets

**Bad:**

```yaml
# sindri.yaml - NEVER DO THIS
environment:
  ANTHROPIC_API_KEY: sk-ant-actual-key # WRONG!
```

**Good:**

```yaml
# sindri.yaml
# Secrets managed via provider mechanisms
```

### Fly.io Secrets

**Set secrets:**

```bash
flyctl secrets set ANTHROPIC_API_KEY=sk-ant-... -a my-app
flyctl secrets set GITHUB_TOKEN=ghp_... -a my-app
```

**Best practices:**

1. **Use per-environment secrets:**

   ```bash
   flyctl secrets set API_KEY=sk-ant-dev... -a my-dev-app
   flyctl secrets set API_KEY=sk-ant-prod... -a my-prod-app
   ```

2. **Rotate secrets regularly:**

   ```bash
   # Generate new API key
   # Update secret
   flyctl secrets set ANTHROPIC_API_KEY=sk-ant-new... -a my-app
   ```

3. **Audit secret usage:**

   ```bash
   # List secrets (values hidden)
   flyctl secrets list -a my-app
   ```

### Docker Secrets

**Use .env files:**

```bash
# .env (not committed)
ANTHROPIC_API_KEY=sk-ant-...
GITHUB_TOKEN=ghp_...
```

**Ensure .gitignore:**

```bash
# .gitignore
.env
.env.*
!.env.example
```

**Use docker-compose secrets:**

```yaml
# docker-compose.yml
services:
  sindri:
    env_file: .env
```

### Kubernetes Secrets

**Create secret:**

```bash
kubectl create secret generic sindri-secrets \
  --from-literal=ANTHROPIC_API_KEY=sk-ant-... \
  --from-literal=GITHUB_TOKEN=ghp_... \
  --namespace=dev-envs
```

**Use sealed secrets (recommended):**

```bash
# Install sealed-secrets controller
kubectl apply -f https://github.com/bitnami-labs/sealed-secrets/releases/download/v0.24.0/controller.yaml

# Create sealed secret
kubeseal --format=yaml < secret.yaml > sealed-secret.yaml

# Commit sealed-secret.yaml (encrypted)
git add sealed-secret.yaml
```

## Network Security

### Fly.io Network Isolation

**Default security:**

- Private networking within organization
- Public IP with firewall rules
- SSH port exposed (configurable)

**Restrict SSH access:**

```yaml
# fly.toml (generated)
[[services]]
  [[services.ports]]
    port = 10022
    handlers = ["tcp"]
    # Add IP allowlist
    allowlist = ["1.2.3.4/32", "5.6.7.8/32"]
```

**Use Fly.io private network:**

```toml
# fly.toml
[env]
  FLY_PRIVATE_NETWORK = "true"
```

### Kubernetes Network Policies

**Restrict ingress:**

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: sindri-policy
  namespace: dev-envs
spec:
  podSelector:
    matchLabels:
      app: sindri
  policyTypes:
    - Ingress
  ingress:
    - from:
        - podSelector:
            matchLabels:
              access: allowed
      ports:
        - protocol: TCP
          port: 22
```

### Firewall Rules (Docker)

**Restrict Docker host access:**

```bash
# Allow only specific IPs to connect
sudo ufw allow from 1.2.3.4 to any port 22
sudo ufw enable
```

## Container Security

### Non-Root User

Sindri runs as `developer` user (UID 1001), not root.

**Verify:**

```bash
whoami  # developer
id      # uid=1001(developer)
```

### Read-Only System Files

System files in `/docker/lib` are owned by root and read-only.

**Verify:**

```bash
ls -la /docker/lib
# drwxr-xr-x root root
```

### Capability Restrictions

Containers run with minimal capabilities.

**Docker:**

```yaml
# docker-compose.yml
services:
  sindri:
    cap_drop:
      - ALL
    cap_add:
      - NET_BIND_SERVICE # Only if needed
```

**Kubernetes:**

```yaml
# securityContext
securityContext:
  capabilities:
    drop:
      - ALL
  readOnlyRootFilesystem: false # /workspace is writable
  runAsNonRoot: true
  runAsUser: 1001
```

## Supply Chain Security

### Software Bill of Materials (SBOM)

Track all installed software:

```bash
# Generate SBOM
./cli/extension-manager bom --format cyclonedx > sbom.cdx.json

# Scan for vulnerabilities
grype sbom:sbom.cdx.json

# Or with Trivy
trivy sbom sbom.cdx.json
```

### Image Scanning

**Scan Docker image:**

```bash
# Using Trivy
trivy image sindri:latest

# Using Grype
grype sindri:latest

# Using Docker Scout
docker scout cves sindri:latest
```

### Base Image Updates

**Keep base image updated:**

```dockerfile
# Dockerfile
FROM ubuntu:24.04  # Use specific version, not 'latest'
```

**Rebuild regularly:**

```bash
# Monthly or after security updates
pnpm build
```

## Dependency Security

### Extension Validation

**Validate extensions:**

```bash
# Validate all extensions
./cli/extension-manager validate-all

# Check BOM for vulnerabilities
./cli/extension-manager bom --format cyclonedx | grype sbom:-
```

### Package Registry Security

**Use trusted sources:**

- npm registry: `registry.npmjs.org`
- PyPI: `pypi.org`
- mise plugins: Official mise registry

**Avoid:**

- Unknown package sources
- Unverified binaries
- Unofficial mirrors

## Audit & Compliance

### Audit Logs

**Fly.io:**

```bash
# View activity logs
flyctl auth logs -a my-app
```

**Kubernetes:**

```bash
# Enable audit logging
# Configure kube-apiserver with --audit-log-path
```

**Container logs:**

```bash
# Extension installation logs
cat /workspace/.system/logs/*.log

# Command history
cat ~/.bash_history
```

### Compliance Scanning

**Generate compliance report:**

```bash
# SBOM in SPDX format
./cli/extension-manager bom --format spdx > sbom.spdx

# Scan against CVE database
grype sbom:sbom.spdx
```

### Security Hardening Checklist

**Deployment:**

- [ ] SSH key-based authentication only
- [ ] Secrets via provider mechanisms
- [ ] Network policies/firewall configured
- [ ] Non-root container user
- [ ] Read-only system files
- [ ] Resource limits set
- [ ] Auto-updates enabled (Fly.io)

**Operations:**

- [ ] Regular SSH key rotation
- [ ] Regular secret rotation
- [ ] SBOM generation and scanning
- [ ] Docker image updates
- [ ] Extension updates
- [ ] Audit log review
- [ ] Backup verification

**Development:**

- [ ] .env files in .gitignore
- [ ] No secrets in code
- [ ] SBOM tracked in version control
- [ ] Security scanning in CI/CD
- [ ] Code review process

## Incident Response

### Compromised Environment

**If you suspect compromise:**

1. **Isolate immediately:**

   ```bash
   # Fly.io: Stop machine
   flyctl machine stop <machine-id> -a my-app

   # Docker: Stop container
   docker-compose stop

   # Kubernetes: Scale to zero
   kubectl scale statefulset my-app --replicas=0 -n dev-envs
   ```

2. **Rotate secrets:**

   ```bash
   # Generate new API keys
   # Update secrets
   flyctl secrets set ANTHROPIC_API_KEY=sk-ant-new... -a my-app
   ```

3. **Rotate SSH keys:**

   ```bash
   # Generate new SSH key
   ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519_new

   # Update Fly.io
   flyctl ssh issue --agent -a my-app
   ```

4. **Review logs:**

   ```bash
   # Check for suspicious activity
   flyctl logs -a my-app

   # Check command history
   cat ~/.bash_history
   ```

5. **Recreate environment:**

   ```bash
   # Destroy compromised environment
   ./cli/sindri teardown

   # Redeploy clean environment
   ./cli/sindri deploy
   ```

## Vulnerability Disclosure

**Report security vulnerabilities:**

- Email: security@[repository-owner-domain]
- GitHub Security Advisories: Private disclosure
- Do not open public issues for security vulnerabilities

## Security Resources

- [OWASP Container Security](https://owasp.org/www-project-docker-top-10/)
- [CIS Docker Benchmark](https://www.cisecurity.org/benchmark/docker)
- [Kubernetes Security Best Practices](https://kubernetes.io/docs/concepts/security/)
- [Fly.io Security](https://fly.io/docs/reference/security/)

## Related Documentation

- [Bill of Materials](BOM.md)
- [Deployment Guide](DEPLOYMENT.md)
- [Configuration Reference](CONFIGURATION.md)
- [Troubleshooting](TROUBLESHOOTING.md)
