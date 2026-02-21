import { useState } from "react";
import { Key, Plus, Trash2, RefreshCw, Eye, EyeOff, AlertTriangle, Clock, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useSecrets, useCreateSecret, useDeleteSecret, useRotateSecret } from "@/hooks/useDrift";
import { secretsApi } from "@/api/drift";
import type { Secret, CreateSecretInput, SecretType } from "@/types/drift";

export function SecretsVault() {
  const [page] = useState(1);
  const [showCreate, setShowCreate] = useState(false);
  const [revealedSecrets, setRevealedSecrets] = useState<Map<string, string>>(new Map());

  const { data, isLoading } = useSecrets({}, page);
  const createSecret = useCreateSecret();
  const deleteSecret = useDeleteSecret();
  const rotateSecret = useRotateSecret();

  const secrets = data?.secrets ?? [];
  const expiredCount = secrets.filter((s) => s.isExpired).length;
  const expiringSoonCount = secrets.filter(
    (s) => !s.isExpired && s.daysUntilExpiry !== null && s.daysUntilExpiry <= 7,
  ).length;

  const toggleReveal = async (id: string) => {
    if (revealedSecrets.has(id)) {
      const next = new Map(revealedSecrets);
      next.delete(id);
      setRevealedSecrets(next);
    } else {
      try {
        const { value } = await secretsApi.revealSecretValue(id);
        setRevealedSecrets((prev) => new Map(prev).set(id, value));
      } catch {
        alert("Failed to reveal secret — insufficient permissions");
      }
    }
  };

  return (
    <div className="space-y-6">
      {/* Summary */}
      {(expiredCount > 0 || expiringSoonCount > 0) && (
        <div className="flex gap-3">
          {expiredCount > 0 && (
            <div className="flex items-center gap-2 rounded-lg border border-red-400/30 bg-red-400/10 px-3 py-2 text-sm text-red-400">
              <AlertTriangle className="h-4 w-4" />
              {expiredCount} expired secret{expiredCount !== 1 ? "s" : ""}
            </div>
          )}
          {expiringSoonCount > 0 && (
            <div className="flex items-center gap-2 rounded-lg border border-yellow-400/30 bg-yellow-400/10 px-3 py-2 text-sm text-yellow-400">
              <Clock className="h-4 w-4" />
              {expiringSoonCount} expiring within 7 days
            </div>
          )}
        </div>
      )}

      {/* Create form */}
      {showCreate && (
        <div className="rounded-lg border border-gray-700 bg-gray-900 p-6">
          <CreateSecretForm
            onSubmit={(input) => {
              createSecret.mutate(input, { onSuccess: () => setShowCreate(false) });
            }}
            onCancel={() => setShowCreate(false)}
            isSubmitting={createSecret.isPending}
          />
        </div>
      )}

      {/* List header */}
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-gray-400">
          {data?.total ?? 0} secret{(data?.total ?? 0) !== 1 ? "s" : ""}
        </h3>
        {!showCreate && (
          <Button size="sm" onClick={() => setShowCreate(true)}>
            <Plus className="mr-2 h-4 w-4" />
            New Secret
          </Button>
        )}
      </div>

      {isLoading && <div className="py-8 text-center text-gray-500">Loading secrets...</div>}

      {!isLoading && secrets.length === 0 && !showCreate && (
        <div className="rounded-lg border border-dashed border-gray-700 py-12 text-center">
          <Key className="mx-auto mb-3 h-8 w-8 text-gray-600" />
          <p className="text-gray-400">No secrets stored</p>
          <p className="mt-1 text-sm text-gray-600">
            Add secrets to the vault for encrypted storage and rotation
          </p>
        </div>
      )}

      <div className="space-y-2">
        {secrets.map((secret) => (
          <SecretRow
            key={secret.id}
            secret={secret}
            revealedValue={revealedSecrets.get(secret.id)}
            onToggleReveal={() => toggleReveal(secret.id)}
            onDelete={() => deleteSecret.mutate(secret.id)}
            onRotate={(newValue) => rotateSecret.mutate({ id: secret.id, value: newValue })}
            isDeleting={deleteSecret.isPending}
          />
        ))}
      </div>
    </div>
  );
}

function SecretRow({
  secret,
  revealedValue,
  onToggleReveal,
  onDelete,
  onRotate,
  isDeleting,
}: {
  secret: Secret;
  revealedValue?: string;
  onToggleReveal: () => void;
  onDelete: () => void;
  onRotate: (value: string) => void;
  isDeleting: boolean;
}) {
  const [showRotateForm, setShowRotateForm] = useState(false);
  const [newValue, setNewValue] = useState("");

  const typeColors: Record<SecretType, string> = {
    ENV_VAR: "text-blue-400 bg-blue-400/10",
    FILE: "text-purple-400 bg-purple-400/10",
    CERTIFICATE: "text-green-400 bg-green-400/10",
    API_KEY: "text-yellow-400 bg-yellow-400/10",
  };

  const isExpiringSoon =
    !secret.isExpired && secret.daysUntilExpiry !== null && secret.daysUntilExpiry <= 7;

  return (
    <div
      className={`rounded-lg border p-4 ${
        secret.isExpired
          ? "border-red-400/30 bg-red-400/5"
          : isExpiringSoon
            ? "border-yellow-400/30 bg-yellow-400/5"
            : "border-gray-700 bg-gray-900/50"
      }`}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2 flex-wrap">
            <Key className="h-4 w-4 shrink-0 text-gray-400" />
            <span className="font-medium text-white">{secret.name}</span>
            <span
              className={`rounded px-1.5 py-0.5 text-xs font-medium ${typeColors[secret.type]}`}
            >
              {secret.type}
            </span>
            {secret.isExpired && (
              <span className="rounded bg-red-400/20 px-1.5 py-0.5 text-xs text-red-400">
                Expired
              </span>
            )}
            {isExpiringSoon && (
              <span className="rounded bg-yellow-400/20 px-1.5 py-0.5 text-xs text-yellow-400">
                Expires in {secret.daysUntilExpiry}d
              </span>
            )}
          </div>
          {secret.description && <p className="mt-1 text-xs text-gray-500">{secret.description}</p>}
          <div className="mt-2 flex items-center gap-3 text-xs text-gray-600">
            {revealedValue !== undefined ? (
              <code className="rounded bg-gray-800 px-2 py-0.5 text-gray-300 font-mono break-all">
                {revealedValue}
              </code>
            ) : (
              <span className="font-mono">••••••••</span>
            )}
          </div>
          <div className="mt-1 flex items-center gap-3 text-xs text-gray-600">
            <span>Created {new Date(secret.createdAt).toLocaleDateString()}</span>
            {secret.lastRotatedAt && (
              <span>Rotated {new Date(secret.lastRotatedAt).toLocaleDateString()}</span>
            )}
            {secret.instanceId && <span>Instance: {secret.instanceId.slice(0, 8)}...</span>}
          </div>

          {showRotateForm && (
            <div className="mt-3 flex items-center gap-2">
              <input
                type="password"
                value={newValue}
                onChange={(e) => setNewValue(e.target.value)}
                placeholder="New secret value"
                className="flex-1 rounded border border-gray-600 bg-gray-800 px-3 py-1.5 text-sm text-white placeholder-gray-500 focus:border-indigo-500 focus:outline-none"
              />
              <button
                onClick={() => {
                  if (newValue) {
                    onRotate(newValue);
                    setNewValue("");
                    setShowRotateForm(false);
                  }
                }}
                className="rounded bg-indigo-600 px-3 py-1.5 text-xs text-white hover:bg-indigo-700"
              >
                Rotate
              </button>
              <button
                onClick={() => {
                  setShowRotateForm(false);
                  setNewValue("");
                }}
                className="rounded p-1.5 text-gray-400 hover:bg-gray-700"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </div>
          )}
        </div>

        <div className="flex items-center gap-1 shrink-0">
          <button
            onClick={onToggleReveal}
            className="rounded p-1.5 text-gray-400 hover:bg-gray-700 hover:text-white transition-colors"
            title={revealedValue !== undefined ? "Hide" : "Reveal"}
          >
            {revealedValue !== undefined ? (
              <EyeOff className="h-4 w-4" />
            ) : (
              <Eye className="h-4 w-4" />
            )}
          </button>
          <button
            onClick={() => setShowRotateForm((v) => !v)}
            className="rounded p-1.5 text-gray-400 hover:bg-gray-700 hover:text-white transition-colors"
            title="Rotate secret"
          >
            <RefreshCw className="h-4 w-4" />
          </button>
          <button
            onClick={onDelete}
            disabled={isDeleting}
            className="rounded p-1.5 text-gray-400 hover:bg-red-400/10 hover:text-red-400 transition-colors disabled:opacity-50"
            title="Delete secret"
          >
            <Trash2 className="h-4 w-4" />
          </button>
        </div>
      </div>
    </div>
  );
}

function CreateSecretForm({
  onSubmit,
  onCancel,
  isSubmitting,
}: {
  onSubmit: (input: CreateSecretInput) => void;
  onCancel: () => void;
  isSubmitting: boolean;
}) {
  const [form, setForm] = useState<CreateSecretInput>({
    name: "",
    type: "ENV_VAR",
    value: "",
  });

  const secretTypes: Array<{ value: SecretType; label: string }> = [
    { value: "ENV_VAR", label: "Environment Variable" },
    { value: "API_KEY", label: "API Key" },
    { value: "CERTIFICATE", label: "Certificate" },
    { value: "FILE", label: "File" },
  ];

  return (
    <div className="space-y-4">
      <h3 className="font-medium text-white">New Secret</h3>
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label className="mb-1 block text-xs font-medium text-gray-400">Name *</label>
          <input
            type="text"
            value={form.name}
            onChange={(e) => setForm((f) => ({ ...f, name: e.target.value }))}
            placeholder="MY_SECRET_KEY"
            className="w-full rounded border border-gray-600 bg-gray-800 px-3 py-2 text-sm text-white placeholder-gray-500 focus:border-indigo-500 focus:outline-none"
          />
        </div>
        <div>
          <label className="mb-1 block text-xs font-medium text-gray-400">Type</label>
          <select
            value={form.type}
            onChange={(e) => setForm((f) => ({ ...f, type: e.target.value as SecretType }))}
            className="w-full rounded border border-gray-600 bg-gray-800 px-3 py-2 text-sm text-white focus:border-indigo-500 focus:outline-none"
          >
            {secretTypes.map((t) => (
              <option key={t.value} value={t.value}>
                {t.label}
              </option>
            ))}
          </select>
        </div>
        <div className="col-span-2">
          <label className="mb-1 block text-xs font-medium text-gray-400">Value *</label>
          <input
            type="password"
            value={form.value}
            onChange={(e) => setForm((f) => ({ ...f, value: e.target.value }))}
            placeholder="Secret value (encrypted at rest)"
            className="w-full rounded border border-gray-600 bg-gray-800 px-3 py-2 text-sm text-white placeholder-gray-500 focus:border-indigo-500 focus:outline-none"
          />
        </div>
        <div className="col-span-2">
          <label className="mb-1 block text-xs font-medium text-gray-400">Description</label>
          <input
            type="text"
            value={form.description ?? ""}
            onChange={(e) => setForm((f) => ({ ...f, description: e.target.value || undefined }))}
            placeholder="Optional description"
            className="w-full rounded border border-gray-600 bg-gray-800 px-3 py-2 text-sm text-white placeholder-gray-500 focus:border-indigo-500 focus:outline-none"
          />
        </div>
        <div>
          <label className="mb-1 block text-xs font-medium text-gray-400">Expires At</label>
          <input
            type="datetime-local"
            value={form.expiresAt?.slice(0, 16) ?? ""}
            onChange={(e) =>
              setForm((f) => ({
                ...f,
                expiresAt: e.target.value ? new Date(e.target.value).toISOString() : undefined,
              }))
            }
            className="w-full rounded border border-gray-600 bg-gray-800 px-3 py-2 text-sm text-white focus:border-indigo-500 focus:outline-none"
          />
        </div>
      </div>
      <div className="flex gap-3">
        <Button
          size="sm"
          onClick={() => onSubmit(form)}
          disabled={!form.name || !form.value || isSubmitting}
        >
          {isSubmitting ? "Creating..." : "Create Secret"}
        </Button>
        <Button size="sm" variant="outline" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}
