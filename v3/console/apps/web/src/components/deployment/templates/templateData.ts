export type TemplateCategory =
  | "ml-ai"
  | "full-stack"
  | "systems"
  | "enterprise"
  | "cloud-native"
  | "data-engineering";

// Structurally compatible with DeploymentTemplate from @sindri-console/shared.
// When that package is added as a web app dependency, this can be replaced with:
//   import type { DeploymentTemplate } from '@sindri-console/shared'
//   export interface Template extends DeploymentTemplate { tags: string[] }
export interface Template {
  id: string;
  name: string;
  slug: string;
  category: string;
  description: string;
  yaml_content: string;
  extensions: string[];
  provider_recommendations: string[];
  is_official: boolean;
  created_by: string | null;
  created_at: string;
  updated_at: string;
  // UI-only: display tags not persisted in the database
  tags: string[];
}

export const TEMPLATE_CATEGORIES: Record<TemplateCategory, { label: string; color: string }> = {
  "ml-ai": { label: "ML / AI", color: "bg-purple-500/10 text-purple-400 border-purple-500/20" },
  "full-stack": { label: "Full-Stack", color: "bg-blue-500/10 text-blue-400 border-blue-500/20" },
  systems: { label: "Systems", color: "bg-orange-500/10 text-orange-400 border-orange-500/20" },
  enterprise: { label: "Enterprise", color: "bg-green-500/10 text-green-400 border-green-500/20" },
  "cloud-native": {
    label: "Cloud Native",
    color: "bg-cyan-500/10 text-cyan-400 border-cyan-500/20",
  },
  "data-engineering": {
    label: "Data Engineering",
    color: "bg-yellow-500/10 text-yellow-400 border-yellow-500/20",
  },
};

const NOW = new Date().toISOString();

export const TEMPLATES: Template[] = [
  {
    id: "python-ml-stack",
    name: "Python ML Stack",
    slug: "python-ml-stack",
    description:
      "Full machine learning environment with Jupyter notebooks, TensorFlow, PyTorch, and CUDA GPU acceleration. Ideal for model training and experimentation.",
    category: "ml-ai",
    tags: ["python", "jupyter", "tensorflow", "pytorch", "cuda", "gpu", "ml", "ai"],
    extensions: ["python3", "jupyter", "tensorflow", "pytorch", "cuda-toolkit", "vscode"],
    provider_recommendations: ["fly", "devpod"],
    is_official: true,
    created_by: null,
    created_at: NOW,
    updated_at: NOW,
    yaml_content: `name: python-ml-stack
description: Python ML environment with Jupyter, TensorFlow, PyTorch, and CUDA

extensions:
  - python3
  - jupyter
  - tensorflow
  - pytorch
  - cuda-toolkit
  - vscode

provider:
  fly:
    region: sea
    vm_size: performance-2x
    memory: 4096
    gpu: a100-40gb

workspace:
  volume_size: 50
  home: /home/dev

env:
  JUPYTER_PORT: "8888"
  CUDA_VISIBLE_DEVICES: "0"
  TF_CPP_MIN_LOG_LEVEL: "1"

ports:
  - port: 8888
    description: Jupyter Lab

console:
  endpoint: \${SINDRI_CONSOLE_URL}
  api_key: \${SINDRI_CONSOLE_API_KEY}
  heartbeat_interval: 30s
`,
  },
  {
    id: "fullstack-typescript",
    name: "Full-Stack TypeScript",
    slug: "fullstack-typescript",
    description:
      "Modern TypeScript development with Node.js LTS, Deno, and Bun runtimes, PostgreSQL and Redis clients, and Docker-in-Docker for containerized workflows.",
    category: "full-stack",
    tags: ["typescript", "node", "deno", "bun", "postgresql", "redis", "docker"],
    extensions: [
      "node-lts",
      "deno",
      "bun",
      "postgresql-client",
      "redis-client",
      "docker-in-docker",
    ],
    provider_recommendations: ["fly", "docker", "devpod"],
    is_official: true,
    created_by: null,
    created_at: NOW,
    updated_at: NOW,
    yaml_content: `name: fullstack-typescript
description: Full-Stack TypeScript with Node LTS, Deno, Bun, PostgreSQL, Redis, and Docker

extensions:
  - node-lts
  - deno
  - bun
  - postgresql-client
  - redis-client
  - docker-in-docker

provider:
  fly:
    region: sea
    vm_size: shared-cpu-2x
    memory: 1024

workspace:
  volume_size: 20
  home: /home/dev

env:
  NODE_ENV: development
  PNPM_HOME: /home/dev/.local/share/pnpm

console:
  endpoint: \${SINDRI_CONSOLE_URL}
  api_key: \${SINDRI_CONSOLE_API_KEY}
  heartbeat_interval: 30s
`,
  },
  {
    id: "rust-systems",
    name: "Rust Systems",
    slug: "rust-systems",
    description:
      "High-performance systems programming environment with the full Rust toolchain, Cargo package manager, LLVM compiler infrastructure, and build essentials.",
    category: "systems",
    tags: ["rust", "cargo", "llvm", "systems", "wasm", "embedded"],
    extensions: ["rust", "cargo", "llvm", "build-essential"],
    provider_recommendations: ["fly", "docker", "devpod"],
    is_official: true,
    created_by: null,
    created_at: NOW,
    updated_at: NOW,
    yaml_content: `name: rust-systems
description: Rust systems programming with full toolchain, LLVM, and build essentials

extensions:
  - rust
  - cargo
  - llvm
  - build-essential

provider:
  fly:
    region: sea
    vm_size: performance-2x
    memory: 2048

workspace:
  volume_size: 30
  home: /home/dev

env:
  RUST_LOG: info
  CARGO_HOME: /home/dev/.cargo
  RUSTUP_HOME: /home/dev/.rustup

console:
  endpoint: \${SINDRI_CONSOLE_URL}
  api_key: \${SINDRI_CONSOLE_API_KEY}
  heartbeat_interval: 30s
`,
  },
  {
    id: "java-enterprise",
    name: "Java Enterprise",
    slug: "java-enterprise",
    description:
      "Enterprise Java development with Java 17 LTS, Maven and Gradle build tools, Spring Boot CLI, and PostgreSQL client for production-grade applications.",
    category: "enterprise",
    tags: ["java", "maven", "gradle", "spring", "postgresql", "enterprise", "jvm"],
    extensions: ["java-17", "maven", "gradle", "spring-boot-cli", "postgresql-client"],
    provider_recommendations: ["fly", "devpod", "kubernetes"],
    is_official: true,
    created_by: null,
    created_at: NOW,
    updated_at: NOW,
    yaml_content: `name: java-enterprise
description: Enterprise Java with Java 17, Maven, Gradle, Spring Boot CLI, and PostgreSQL

extensions:
  - java-17
  - maven
  - gradle
  - spring-boot-cli
  - postgresql-client

provider:
  fly:
    region: iad
    vm_size: shared-cpu-2x
    memory: 2048

workspace:
  volume_size: 20
  home: /home/dev

env:
  JAVA_HOME: /usr/lib/jvm/java-17-openjdk-amd64
  MAVEN_OPTS: "-Xmx512m"
  GRADLE_USER_HOME: /home/dev/.gradle

console:
  endpoint: \${SINDRI_CONSOLE_URL}
  api_key: \${SINDRI_CONSOLE_API_KEY}
  heartbeat_interval: 30s
`,
  },
  {
    id: "go-microservices",
    name: "Go Microservices",
    slug: "go-microservices",
    description:
      "Cloud-native Go development environment with Docker-in-Docker, Kubernetes CLI, and Helm for building and deploying microservices at scale.",
    category: "cloud-native",
    tags: ["go", "golang", "kubernetes", "helm", "docker", "microservices", "cloud-native"],
    extensions: ["golang", "docker-in-docker", "kubectl", "helm"],
    provider_recommendations: ["fly", "kubernetes", "devpod"],
    is_official: true,
    created_by: null,
    created_at: NOW,
    updated_at: NOW,
    yaml_content: `name: go-microservices
description: Go microservices with Docker, Kubernetes, and Helm for cloud-native development

extensions:
  - golang
  - docker-in-docker
  - kubectl
  - helm

provider:
  fly:
    region: sea
    vm_size: shared-cpu-2x
    memory: 1024

workspace:
  volume_size: 20
  home: /home/dev

env:
  GOPATH: /home/dev/go
  GOBIN: /home/dev/go/bin
  CGO_ENABLED: "0"

console:
  endpoint: \${SINDRI_CONSOLE_URL}
  api_key: \${SINDRI_CONSOLE_API_KEY}
  heartbeat_interval: 30s
`,
  },
  {
    id: "data-engineering",
    name: "Data Engineering",
    slug: "data-engineering",
    description:
      "Complete data pipeline environment with Python, Apache Spark, Kafka, Apache Airflow for orchestration, and dbt for data transformation workflows.",
    category: "data-engineering",
    tags: ["python", "spark", "kafka", "airflow", "dbt", "data", "pipeline", "etl"],
    extensions: ["python3", "apache-spark", "kafka", "airflow", "dbt"],
    provider_recommendations: ["fly", "kubernetes", "devpod"],
    is_official: true,
    created_by: null,
    created_at: NOW,
    updated_at: NOW,
    yaml_content: `name: data-engineering
description: Data pipeline stack with Python, Spark, Kafka, Airflow, and dbt

extensions:
  - python3
  - apache-spark
  - kafka
  - airflow
  - dbt

provider:
  fly:
    region: sea
    vm_size: performance-4x
    memory: 8192

workspace:
  volume_size: 100
  home: /home/dev

env:
  SPARK_HOME: /opt/spark
  AIRFLOW_HOME: /home/dev/airflow
  KAFKA_BOOTSTRAP_SERVERS: "localhost:9092"
  DBT_PROFILES_DIR: /home/dev/.dbt

ports:
  - port: 8080
    description: Airflow Webserver
  - port: 4040
    description: Spark UI

console:
  endpoint: \${SINDRI_CONSOLE_URL}
  api_key: \${SINDRI_CONSOLE_API_KEY}
  heartbeat_interval: 30s
`,
  },
];
