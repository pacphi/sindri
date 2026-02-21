# ── Stage 1: build ────────────────────────────────────────────────────────────
FROM node:24-alpine AS builder

RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

WORKDIR /app

# Copy workspace manifests first so dependency installation is cached
# independently of source changes.
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY apps/api/package.json ./apps/api/
COPY packages/shared/package.json ./packages/shared/
COPY packages/ui/package.json ./packages/ui/

RUN pnpm install --frozen-lockfile

# Copy source files
COPY apps/api/tsconfig.json ./apps/api/
COPY apps/api/src ./apps/api/src
COPY apps/api/prisma ./apps/api/prisma
COPY packages/shared ./packages/shared

# Generate Prisma client and compile TypeScript
RUN pnpm --filter=@sindri-console/api exec prisma generate
RUN pnpm --filter=@sindri-console/api run build

# ── Stage 2: runtime ──────────────────────────────────────────────────────────
FROM node:24-alpine

RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

WORKDIR /app

# Copy workspace manifests and lockfile so pnpm can restore the store layout
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY apps/api/package.json ./apps/api/
COPY packages/shared/package.json ./packages/shared/
COPY packages/ui/package.json ./packages/ui/

# Production-only install (no dev tools, no tsx, no tsc)
RUN pnpm install --frozen-lockfile --prod

# Copy compiled output and Prisma artifacts from the build stage
COPY --from=builder /app/apps/api/dist ./apps/api/dist
COPY --from=builder /app/apps/api/prisma ./apps/api/prisma

# Re-generate Prisma client against the production node_modules
RUN pnpm --filter=@sindri-console/api exec prisma generate

COPY packages/shared/src ./packages/shared/src

WORKDIR /app/apps/api

EXPOSE 3001

# Run pending migrations then start the compiled server
CMD ["sh", "-c", "npx prisma migrate deploy && node dist/index.js"]
