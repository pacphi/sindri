# Sindri Extension Examples

Real examples from the Sindri codebase demonstrating different extension patterns.

## 1. Language Runtime (mise-based)

**Pattern:** Simple language runtime installation via mise
**Use case:** Node.js, Python, Go, Rust, Ruby

```yaml
# docker/lib/extensions/nodejs/extension.yaml
---
metadata:
  name: nodejs
  version: 1.0.0
  description: Node.js LTS via mise
  category: language
  dependencies: []

requirements:
  domains:
    - registry.npmjs.org
    - nodejs.org
  diskSpace: 600

install:
  method: mise
  mise:
    configFile: mise.toml
    reshimAfterInstall: true

configure:
  environment:
    - key: NODE_ENV
      value: development
      scope: bashrc

validate:
  commands:
    - name: node
      expectedPattern: "v\\d+\\.\\d+\\.\\d+"
    - name: npm

remove:
  mise:
    removeConfig: true
    tools: [nodejs]

bom:
  tools:
    - name: node
      version: dynamic
      source: mise
      type: runtime
      license: MIT
      homepage: https://nodejs.org
      purl: pkg:generic/nodejs
    - name: npm
      version: dynamic
      source: mise
      type: package-manager
      license: Artistic-2.0
      homepage: https://www.npmjs.com
      purl: pkg:npm/npm
```

**Key points:**

- Uses `mise` method with a `mise.toml` config file
- Validates both `node` and `npm` commands
- Sets `NODE_ENV` environment variable
- BOM tracks installed tools with their sources

---

## 2. Infrastructure Tool (apt-based)

**Pattern:** System package installation via APT repositories
**Use case:** Docker, system tools requiring root installation

```yaml
# docker/lib/extensions/docker/extension.yaml
---
metadata:
  name: docker
  version: 1.0.0
  description: Docker Engine and Compose
  category: infrastructure
  dependencies: []

requirements:
  domains:
    - download.docker.com
    - hub.docker.com
  diskSpace: 1000

install:
  method: apt
  apt:
    repositories:
      - gpgKey: https://download.docker.com/linux/ubuntu/gpg
        sources: >-
          deb [arch=amd64] https://download.docker.com/linux/ubuntu
          jammy stable
    packages:
      - docker-ce
      - docker-ce-cli
      - containerd.io
      - docker-compose-plugin

configure:
  environment:
    - key: DOCKER_BUILDKIT
      value: "1"
      scope: bashrc

validate:
  commands:
    - name: docker
      expectedPattern: "Docker version \\d+\\.\\d+\\.\\d+"

remove:
  confirmation: true
  apt:
    packages:
      - docker-ce
      - docker-ce-cli
      - containerd.io
      - docker-compose-plugin

bom:
  tools:
    - name: docker
      version: dynamic
      source: apt
      type: server
      license: Apache-2.0
      homepage: https://www.docker.com
      purl: pkg:deb/ubuntu/docker-ce
      cpe: cpe:2.3:a:docker:docker:*:*:*:*:*:*:*:*
```

**Key points:**

- Uses `apt` method with custom repository and GPG key
- Large disk requirement (1000MB)
- Requires confirmation before removal
- Includes CPE for vulnerability scanning

---

## 3. Development Tools with Dependencies

**Pattern:** Tools that depend on another extension
**Use case:** Language-specific development tools

```yaml
# docker/lib/extensions/nodejs-devtools/extension.yaml
---
metadata:
  name: nodejs-devtools
  version: 2.0.0
  description: >-
    TypeScript, ESLint, Prettier, and Node.js development tools via mise
    npm backend
  category: dev-tools
  author: Sindri Team
  dependencies:
    - nodejs

requirements:
  domains:
    - registry.npmjs.org
  diskSpace: 200
  secrets:
    - perplexity_api_key

install:
  method: mise
  mise:
    configFile: mise.toml
    reshimAfterInstall: true

configure:
  templates:
    - source: prettierrc.template
      destination: ~/templates/.prettierrc
      mode: overwrite
    - source: eslintrc.template
      destination: ~/templates/.eslintrc.json
      mode: overwrite
    - source: tsconfig.template
      destination: ~/templates/tsconfig.json
      mode: overwrite

validate:
  commands:
    - name: tsc
      expectedPattern: "Version \\d+\\.\\d+\\.\\d+"
    - name: ts-node
    - name: prettier
    - name: eslint
    - name: nodemon
    - name: goalie
  mise:
    tools:
      - npm:typescript
      - npm:prettier
      - npm:eslint
    minToolCount: 3

remove:
  mise:
    removeConfig: true
    tools:
      - npm:typescript
      - npm:ts-node
      - npm:nodemon
      - npm:prettier
      - npm:eslint
      - npm:@typescript-eslint/parser
      - npm:@typescript-eslint/eslint-plugin
      - npm:goalie
      - npm:research-swarm
  paths:
    - ~/templates/.prettierrc
    - ~/templates/.eslintrc.json
    - ~/templates/tsconfig.json

upgrade:
  strategy: automatic
  mise:
    upgradeAll: true

bom:
  tools:
    - name: typescript
      version: dynamic
      source: npm
      type: compiler
      license: Apache-2.0
      homepage: https://www.typescriptlang.org
      purl: pkg:npm/typescript
    - name: prettier
      version: dynamic
      source: npm
      type: cli-tool
      license: MIT
      homepage: https://prettier.io
      purl: pkg:npm/prettier
    - name: eslint
      version: dynamic
      source: npm
      type: cli-tool
      license: MIT
      homepage: https://eslint.org
      purl: pkg:npm/eslint
```

**Key points:**

- Depends on `nodejs` extension (installed first)
- Uses templates for configuration files
- Validates multiple commands
- Uses mise tool validation with minimum count
- Cleans up template files on removal
- Automatic upgrade strategy

---

## 4. Script-Based Installation

**Pattern:** Custom installation script for complex setups
**Use case:** Tools requiring custom installation logic

```yaml
# docker/lib/extensions/playwright/extension.yaml
---
metadata:
  name: playwright
  version: 2.0.0
  description: Playwright browser automation framework with Chromium
  category: dev-tools
  author: Sindri Team
  dependencies:
    - nodejs

requirements:
  domains:
    - registry.npmjs.org
    - playwright.azureedge.net
  diskSpace: 1000

install:
  method: script
  script:
    path: install.sh
    timeout: 900

configure:
  templates:
    - source: playwright-config.template
      destination: ~/playwright.config.ts
      mode: overwrite
    - source: test-spec.template
      destination: ~/tests/example.spec.ts
      mode: overwrite
    - source: tsconfig.template
      destination: ~/tsconfig.json
      mode: overwrite

validate:
  commands:
    - name: npx
      versionFlag: playwright --version

remove:
  paths:
    - ~/node_modules/playwright
    - ~/node_modules/@playwright
    - ~/.cache/ms-playwright

upgrade:
  strategy: manual
  script:
    path: upgrade.sh
    timeout: 600

bom:
  tools:
    - name: playwright
      version: dynamic
      source: npm
      type: framework
      license: Apache-2.0
      homepage: https://playwright.dev
      purl: pkg:npm/@playwright/test
```

**Key points:**

- Uses custom `install.sh` script with long timeout (900s)
- Custom validation flag: `npx playwright --version`
- Manual upgrade strategy with separate script
- Removes cache directories on uninstall

---

## Directory Structure Examples

### Minimal Extension (mise-based)

```text
docker/lib/extensions/my-language/
├── extension.yaml
└── mise.toml
```

### Full Extension (script-based)

```text
docker/lib/extensions/my-tool/
├── extension.yaml
├── mise.toml              # Optional
├── scripts/
│   ├── install.sh
│   ├── upgrade.sh
│   └── uninstall.sh
└── templates/
    ├── config.template
    └── rc.template
```

---

## Registry Entry Examples

```yaml
# docker/lib/registry.yaml
extensions:
  # Language runtime - no dependencies
  nodejs:
    category: language
    description: Node.js LTS runtime
    dependencies: []
    protected: false

  # Dev tool - depends on language
  nodejs-devtools:
    category: dev-tools
    description: TypeScript, ESLint, Prettier
    dependencies:
      - nodejs
    protected: false

  # Infrastructure - standalone
  docker:
    category: infrastructure
    description: Docker Engine and Compose
    dependencies: []
    protected: false

  # Protected system extension
  mise-config:
    category: base
    description: mise package manager configuration
    dependencies: []
    protected: true
```

---

## mise.toml Examples

### Simple Language Tool

```toml
# docker/lib/extensions/nodejs/mise.toml
[tools]
node = "lts"
```

### Multiple npm Tools

```toml
# docker/lib/extensions/nodejs-devtools/mise.toml
# IMPORTANT: Use pinned major.minor versions instead of "latest"
# "latest" requires npm registry queries which can timeout and poison
# subsequent mise operations if they fail.
[tools]
"npm:typescript" = "5.9"
"npm:ts-node" = "10.9"
"npm:nodemon" = "3.1"
"npm:prettier" = "3.6"
"npm:eslint" = "9"
```

### Specific Versions

```toml
[tools]
python = "3.12"
"pipx:poetry" = "1.8.0"
```

---

## Install Script Example

```bash
#!/usr/bin/env bash
# docker/lib/extensions/playwright/scripts/install.sh
set -euo pipefail

echo "Installing Playwright..."

# Navigate to home directory
cd "$HOME"

# Initialize npm if needed
if [[ ! -f package.json ]]; then
    npm init -y
fi

# Install Playwright
npm install --save-dev @playwright/test

# Install browsers (Chromium only for size)
npx playwright install chromium
npx playwright install-deps chromium

echo "Playwright installed successfully"
```

---

## Template Example

```json
// docker/lib/extensions/nodejs-devtools/templates/eslintrc.template
{
  "root": true,
  "parser": "@typescript-eslint/parser",
  "plugins": ["@typescript-eslint"],
  "extends": ["eslint:recommended", "plugin:@typescript-eslint/recommended"],
  "env": {
    "node": true,
    "es2022": true
  },
  "rules": {
    "no-unused-vars": "off",
    "@typescript-eslint/no-unused-vars": "error"
  }
}
```

---

## Validation Patterns

### Basic Version Check

```yaml
validate:
  commands:
    - name: node
      expectedPattern: "v\\d+\\.\\d+\\.\\d+"
```

### Custom Version Flag

```yaml
validate:
  commands:
    - name: npx
      versionFlag: playwright --version
```

### Multiple Tools with mise

```yaml
validate:
  commands:
    - name: tsc
    - name: eslint
    - name: prettier
  mise:
    tools:
      - npm:typescript
      - npm:eslint
    minToolCount: 2
```

### Script Validation

```yaml
validate:
  script:
    path: scripts/validate.sh
    timeout: 60
```
