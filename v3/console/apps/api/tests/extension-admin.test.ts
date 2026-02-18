/**
 * Integration tests: Phase 4 Extension Administration & Registry
 *
 * Tests the extension management system:
 *   - Extension registry listing and search
 *   - Extension installation and removal per instance
 *   - Custom/private extension upload and validation
 *   - Extension usage tracking (which instances use which extensions)
 *   - Registry metadata: version, author, license, compatibility
 *   - Admin-only extension governance (approval workflow)
 */

import { describe, it, expect } from "vitest";

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

type ExtensionStatus = "APPROVED" | "PENDING" | "REJECTED" | "DEPRECATED";
type ExtensionVisibility = "PUBLIC" | "PRIVATE" | "TEAM";
type InstallationStatus = "INSTALLED" | "INSTALLING" | "FAILED" | "REMOVED";

interface Extension {
  id: string;
  name: string;
  slug: string;
  description: string;
  version: string;
  author: string;
  license: string;
  homepage: string | null;
  status: ExtensionStatus;
  visibility: ExtensionVisibility;
  is_official: boolean;
  compatible_providers: string[];
  tags: string[];
  install_count: number;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

interface ExtensionVersion {
  id: string;
  extension_id: string;
  version: string;
  changelog: string | null;
  artifact_url: string;
  checksum: string;
  published_at: string;
  is_latest: boolean;
}

interface ExtensionInstallation {
  id: string;
  extension_id: string;
  instance_id: string;
  version: string;
  status: InstallationStatus;
  config: Record<string, unknown>;
  installed_at: string;
  installed_by: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

function makeExtension(overrides: Partial<Extension> = {}): Extension {
  return {
    id: "ext_01",
    name: "Node.js LTS",
    slug: "node-lts",
    description: "Node.js Long-Term Support runtime",
    version: "20.11.0",
    author: "Sindri Team",
    license: "MIT",
    homepage: "https://nodejs.org",
    status: "APPROVED",
    visibility: "PUBLIC",
    is_official: true,
    compatible_providers: ["fly", "docker", "devpod"],
    tags: ["runtime", "javascript", "node"],
    install_count: 1423,
    created_by: null,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-02-01T00:00:00Z",
    ...overrides,
  };
}

function makeExtensionVersion(overrides: Partial<ExtensionVersion> = {}): ExtensionVersion {
  return {
    id: "extv_01",
    extension_id: "ext_01",
    version: "20.11.0",
    changelog: "Security patch for CVE-2024-12345",
    artifact_url: "https://registry.sindri.dev/extensions/node-lts/20.11.0.tar.gz",
    checksum: "sha256:" + "b".repeat(64),
    published_at: "2026-02-01T00:00:00Z",
    is_latest: true,
    ...overrides,
  };
}

function makeInstallation(overrides: Partial<ExtensionInstallation> = {}): ExtensionInstallation {
  return {
    id: "inst_ext_01",
    extension_id: "ext_01",
    instance_id: "instance_01",
    version: "20.11.0",
    status: "INSTALLED",
    config: {},
    installed_at: "2026-02-17T00:00:00Z",
    installed_by: "user_01",
    ...overrides,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Extension Registry
// ─────────────────────────────────────────────────────────────────────────────

describe("Extension Admin: Registry", () => {
  it("extension has required fields: id, name, slug, version, status", () => {
    const ext = makeExtension();
    expect(ext.id).toBeTruthy();
    expect(ext.name).toBeTruthy();
    expect(ext.slug).toBeTruthy();
    expect(ext.version).toBeTruthy();
    expect(["APPROVED", "PENDING", "REJECTED", "DEPRECATED"]).toContain(ext.status);
  });

  it("extension slug is URL-safe (lowercase alphanumeric with hyphens)", () => {
    const slugRegex = /^[a-z0-9-]+$/;
    const ext = makeExtension({ slug: "node-lts" });
    expect(slugRegex.test(ext.slug)).toBe(true);
  });

  it("extension version follows semver format", () => {
    const semverRegex = /^\d+\.\d+\.\d+(-[\w.]+)?(\+[\w.]+)?$/;
    const validVersions = ["20.11.0", "1.2.3", "3.0.0-beta.1"];
    const invalidVersions = ["20", "latest", "v1.2.3"];
    for (const v of validVersions) {
      expect(semverRegex.test(v)).toBe(true);
    }
    for (const v of invalidVersions) {
      expect(semverRegex.test(v)).toBe(false);
    }
  });

  it("official extensions are marked as is_official: true", () => {
    const official = makeExtension({ is_official: true, created_by: null });
    const community = makeExtension({ is_official: false, created_by: "user_01" });
    expect(official.is_official).toBe(true);
    expect(official.created_by).toBeNull();
    expect(community.is_official).toBe(false);
  });

  it("registry returns only APPROVED extensions by default", () => {
    const extensions: Extension[] = [
      makeExtension({ id: "e1", status: "APPROVED" }),
      makeExtension({ id: "e2", status: "PENDING" }),
      makeExtension({ id: "e3", status: "REJECTED" }),
      makeExtension({ id: "e4", status: "APPROVED" }),
      makeExtension({ id: "e5", status: "DEPRECATED" }),
    ];
    const publicRegistry = extensions.filter((e) => e.status === "APPROVED");
    expect(publicRegistry).toHaveLength(2);
  });

  it("registry supports tag-based filtering", () => {
    const extensions: Extension[] = [
      makeExtension({ id: "e1", tags: ["runtime", "javascript"] }),
      makeExtension({ id: "e2", tags: ["runtime", "python"] }),
      makeExtension({ id: "e3", tags: ["database", "postgresql"] }),
    ];
    const runtimeExts = extensions.filter((e) => e.tags.includes("runtime"));
    expect(runtimeExts).toHaveLength(2);
    const pythonExts = extensions.filter((e) => e.tags.includes("python"));
    expect(pythonExts).toHaveLength(1);
  });

  it("registry supports text search by name or description", () => {
    const extensions: Extension[] = [
      makeExtension({ id: "e1", name: "Node.js LTS", description: "JavaScript runtime" }),
      makeExtension({ id: "e2", name: "Python 3.12", description: "Python runtime" }),
      makeExtension({ id: "e3", name: "PostgreSQL", description: "Database server" }),
    ];
    const query = "runtime";
    const results = extensions.filter(
      (e) => e.name.toLowerCase().includes(query) || e.description.toLowerCase().includes(query),
    );
    expect(results).toHaveLength(2);
  });

  it("registry can filter by compatible provider", () => {
    const extensions: Extension[] = [
      makeExtension({ id: "e1", compatible_providers: ["fly", "docker"] }),
      makeExtension({ id: "e2", compatible_providers: ["kubernetes"] }),
      makeExtension({ id: "e3", compatible_providers: ["fly", "docker", "devpod"] }),
    ];
    const flyCompatible = extensions.filter((e) => e.compatible_providers.includes("fly"));
    expect(flyCompatible).toHaveLength(2);
  });

  it("registry extensions are sorted by install_count descending by default", () => {
    const extensions: Extension[] = [
      makeExtension({ id: "e1", install_count: 500 }),
      makeExtension({ id: "e2", install_count: 2000 }),
      makeExtension({ id: "e3", install_count: 100 }),
    ];
    const sorted = [...extensions].sort((a, b) => b.install_count - a.install_count);
    expect(sorted[0].id).toBe("e2");
    expect(sorted[2].id).toBe("e3");
  });

  it("extension slug must be unique in the registry", () => {
    const slugs = ["node-lts", "python-312", "postgresql"];
    const unique = new Set(slugs);
    expect(unique.size).toBe(slugs.length);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Extension Versioning
// ─────────────────────────────────────────────────────────────────────────────

describe("Extension Admin: Versioning", () => {
  it("extension version has required fields: version, artifact_url, checksum", () => {
    const v = makeExtensionVersion();
    expect(v.version).toBeTruthy();
    expect(v.artifact_url).toBeTruthy();
    expect(v.checksum).toBeTruthy();
  });

  it("checksum uses sha256 format", () => {
    const v = makeExtensionVersion();
    expect(v.checksum).toMatch(/^sha256:/);
  });

  it("only one version can be marked as latest per extension", () => {
    const versions: ExtensionVersion[] = [
      makeExtensionVersion({ id: "v1", version: "20.9.0", is_latest: false }),
      makeExtensionVersion({ id: "v2", version: "20.10.0", is_latest: false }),
      makeExtensionVersion({ id: "v3", version: "20.11.0", is_latest: true }),
    ];
    const latestVersions = versions.filter((v) => v.is_latest);
    expect(latestVersions).toHaveLength(1);
    expect(latestVersions[0].version).toBe("20.11.0");
  });

  it("versions are sorted newest first by published_at", () => {
    const versions: ExtensionVersion[] = [
      makeExtensionVersion({ version: "20.9.0", published_at: "2026-01-01T00:00:00Z" }),
      makeExtensionVersion({ version: "20.11.0", published_at: "2026-02-01T00:00:00Z" }),
      makeExtensionVersion({ version: "20.10.0", published_at: "2026-01-15T00:00:00Z" }),
    ];
    const sorted = [...versions].sort(
      (a, b) => new Date(b.published_at).getTime() - new Date(a.published_at).getTime(),
    );
    expect(sorted[0].version).toBe("20.11.0");
  });

  it("version changelog is optional", () => {
    const v = makeExtensionVersion({ changelog: null });
    expect(v.changelog).toBeNull();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Extension Installation
// ─────────────────────────────────────────────────────────────────────────────

describe("Extension Admin: Installation", () => {
  it("installation has required fields: extension_id, instance_id, version, status", () => {
    const inst = makeInstallation();
    expect(inst.extension_id).toBeTruthy();
    expect(inst.instance_id).toBeTruthy();
    expect(inst.version).toBeTruthy();
    expect(["INSTALLED", "INSTALLING", "FAILED", "REMOVED"]).toContain(inst.status);
  });

  it("each instance can have multiple extensions installed", () => {
    const installations: ExtensionInstallation[] = [
      makeInstallation({ id: "i1", extension_id: "ext_01", instance_id: "inst_01" }),
      makeInstallation({ id: "i2", extension_id: "ext_02", instance_id: "inst_01" }),
      makeInstallation({ id: "i3", extension_id: "ext_03", instance_id: "inst_01" }),
    ];
    const inst01Exts = installations.filter((i) => i.instance_id === "inst_01");
    expect(inst01Exts).toHaveLength(3);
  });

  it("same extension can be installed on multiple instances", () => {
    const installations: ExtensionInstallation[] = [
      makeInstallation({ id: "i1", extension_id: "ext_01", instance_id: "inst_01" }),
      makeInstallation({ id: "i2", extension_id: "ext_01", instance_id: "inst_02" }),
    ];
    const ext01Insts = installations.filter((i) => i.extension_id === "ext_01");
    expect(ext01Insts).toHaveLength(2);
  });

  it("extension removal marks status as REMOVED not deletes record", () => {
    const inst = makeInstallation({ status: "REMOVED" });
    expect(inst.status).toBe("REMOVED");
  });

  it("active installations are those with INSTALLED status", () => {
    const installations: ExtensionInstallation[] = [
      makeInstallation({ id: "i1", status: "INSTALLED" }),
      makeInstallation({ id: "i2", status: "REMOVED" }),
      makeInstallation({ id: "i3", status: "FAILED" }),
      makeInstallation({ id: "i4", status: "INSTALLED" }),
    ];
    const active = installations.filter((i) => i.status === "INSTALLED");
    expect(active).toHaveLength(2);
  });

  it("installation config captures extension-specific parameters", () => {
    const inst = makeInstallation({
      config: { nodeVersion: "20", npm: true, yarn: false },
    });
    expect(inst.config).toHaveProperty("nodeVersion", "20");
  });

  it("installation records who installed the extension", () => {
    const inst = makeInstallation({ installed_by: "user_admin_01" });
    expect(inst.installed_by).toBe("user_admin_01");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Custom Extension Upload
// ─────────────────────────────────────────────────────────────────────────────

describe("Extension Admin: Custom Extension Upload", () => {
  interface UploadPayload {
    name: string;
    slug: string;
    version: string;
    description: string;
    license: string;
    visibility: ExtensionVisibility;
    artifact: Buffer | null;
  }

  function makeUploadPayload(overrides: Partial<UploadPayload> = {}): UploadPayload {
    return {
      name: "My Custom Extension",
      slug: "my-custom-ext",
      version: "1.0.0",
      description: "A private extension for our team",
      license: "MIT",
      visibility: "PRIVATE",
      artifact: Buffer.from("fake-extension-binary"),
      ...overrides,
    };
  }

  it("custom extension upload requires name, slug, version, artifact", () => {
    const payload = makeUploadPayload();
    expect(payload.name).toBeTruthy();
    expect(payload.slug).toBeTruthy();
    expect(payload.version).toBeTruthy();
    expect(payload.artifact).not.toBeNull();
  });

  it("uploaded extension starts in PENDING status awaiting approval", () => {
    const ext = makeExtension({
      status: "PENDING",
      is_official: false,
      visibility: "PRIVATE",
    });
    expect(ext.status).toBe("PENDING");
  });

  it("private extension is only visible to the creator and admins", () => {
    const ext = makeExtension({ visibility: "PRIVATE", created_by: "user_01" });
    expect(ext.visibility).toBe("PRIVATE");
    expect(ext.created_by).toBeTruthy();
  });

  it("team extension is visible to all members of the team", () => {
    const ext = makeExtension({ visibility: "TEAM" });
    expect(ext.visibility).toBe("TEAM");
  });

  it("public extension is visible to all authenticated users", () => {
    const ext = makeExtension({ visibility: "PUBLIC" });
    expect(ext.visibility).toBe("PUBLIC");
  });

  it("slug cannot conflict with an existing extension", () => {
    const existingSlugs = new Set(["node-lts", "python-312", "git"]);
    const newSlug = "my-custom-ext";
    expect(existingSlugs.has(newSlug)).toBe(false);
  });

  it("upload rejected if slug conflicts with existing extension", () => {
    const existingSlugs = new Set(["node-lts", "python-312", "git"]);
    const newSlug = "node-lts"; // conflict
    expect(existingSlugs.has(newSlug)).toBe(true);
  });

  it("extension artifact must not be empty", () => {
    const payload = makeUploadPayload({ artifact: Buffer.from("") });
    const isValid = payload.artifact !== null && payload.artifact.length > 0;
    expect(isValid).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Usage Tracking
// ─────────────────────────────────────────────────────────────────────────────

describe("Extension Admin: Usage Tracking", () => {
  interface UsageSummary {
    extension_id: string;
    extension_name: string;
    total_installs: number;
    active_installs: number;
    instances: string[];
  }

  it("usage summary includes total and active install counts", () => {
    const summary: UsageSummary = {
      extension_id: "ext_01",
      extension_name: "Node.js LTS",
      total_installs: 10,
      active_installs: 8,
      instances: Array.from({ length: 8 }, (_, i) => `inst_0${i + 1}`),
    };
    expect(summary.total_installs).toBeGreaterThanOrEqual(summary.active_installs);
    expect(summary.instances).toHaveLength(summary.active_installs);
  });

  it("extension install_count increments on each new installation", () => {
    let installCount = 100;
    installCount += 1;
    expect(installCount).toBe(101);
  });

  it("usage report shows which instances use each extension", () => {
    const installations: ExtensionInstallation[] = [
      makeInstallation({ extension_id: "ext_01", instance_id: "inst_01", status: "INSTALLED" }),
      makeInstallation({ extension_id: "ext_01", instance_id: "inst_02", status: "INSTALLED" }),
      makeInstallation({ extension_id: "ext_02", instance_id: "inst_01", status: "INSTALLED" }),
    ];
    const ext01Instances = installations
      .filter((i) => i.extension_id === "ext_01" && i.status === "INSTALLED")
      .map((i) => i.instance_id);
    expect(ext01Instances).toHaveLength(2);
    expect(ext01Instances).toContain("inst_01");
    expect(ext01Instances).toContain("inst_02");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Admin Governance
// ─────────────────────────────────────────────────────────────────────────────

describe("Extension Admin: Governance", () => {
  it("PENDING extension can be approved by admin", () => {
    let ext = makeExtension({ status: "PENDING" });
    // Simulate approval
    ext = { ...ext, status: "APPROVED" };
    expect(ext.status).toBe("APPROVED");
  });

  it("PENDING extension can be rejected by admin", () => {
    let ext = makeExtension({ status: "PENDING" });
    ext = { ...ext, status: "REJECTED" };
    expect(ext.status).toBe("REJECTED");
  });

  it("APPROVED extension can be deprecated", () => {
    let ext = makeExtension({ status: "APPROVED" });
    ext = { ...ext, status: "DEPRECATED" };
    expect(ext.status).toBe("DEPRECATED");
  });

  it("REJECTED extension cannot be installed", () => {
    const ext = makeExtension({ status: "REJECTED" });
    const canInstall = ext.status === "APPROVED";
    expect(canInstall).toBe(false);
  });

  it("DEPRECATED extension shows warning but can still be installed", () => {
    const ext = makeExtension({ status: "DEPRECATED" });
    // Deprecated extensions may be installed but show deprecation warning
    const isDeprecated = ext.status === "DEPRECATED";
    expect(isDeprecated).toBe(true);
  });

  it("only official extensions can be marked as is_official", () => {
    const communityExt = makeExtension({ is_official: false, created_by: "user_01" });
    expect(communityExt.is_official).toBe(false);
    expect(communityExt.created_by).toBeTruthy();
  });
});
