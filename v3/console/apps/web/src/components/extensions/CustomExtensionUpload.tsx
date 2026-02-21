import { useState } from "react";
import { Upload, Package, X, CheckCircle, AlertCircle, Tag, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useCreateExtension } from "@/hooks/useExtensions";
import type { CreateExtensionInput, ExtensionScope } from "@/types/extension";

interface CustomExtensionUploadProps {
  onClose: () => void;
  onSuccess?: () => void;
}

const CATEGORIES = ["language", "tool", "framework", "database", "ai", "infrastructure", "other"];

export function CustomExtensionUpload({ onClose, onSuccess }: CustomExtensionUploadProps) {
  const [form, setForm] = useState<Partial<CreateExtensionInput>>({
    scope: "PRIVATE",
    tags: [],
    dependencies: [],
  });
  const [tagInput, setTagInput] = useState("");
  const [depInput, setDepInput] = useState("");
  const [errors, setErrors] = useState<Record<string, string>>({});

  const createExtension = useCreateExtension();

  const update = (field: keyof CreateExtensionInput, value: unknown) => {
    setForm((f) => ({ ...f, [field]: value }));
    setErrors((e) => ({ ...e, [field]: "" }));
  };

  const addTag = () => {
    const tag = tagInput.trim().toLowerCase();
    if (tag && !form.tags?.includes(tag)) {
      update("tags", [...(form.tags ?? []), tag]);
    }
    setTagInput("");
  };

  const removeTag = (tag: string) => {
    update("tags", form.tags?.filter((t) => t !== tag) ?? []);
  };

  const addDep = () => {
    const dep = depInput.trim().toLowerCase();
    if (dep && !form.dependencies?.includes(dep)) {
      update("dependencies", [...(form.dependencies ?? []), dep]);
    }
    setDepInput("");
  };

  const removeDep = (dep: string) => {
    update("dependencies", form.dependencies?.filter((d) => d !== dep) ?? []);
  };

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {};
    if (!form.name?.match(/^[a-z0-9_-]+$/)) {
      newErrors.name = "Name must be lowercase alphanumeric with hyphens/underscores";
    }
    if (!form.display_name) newErrors.display_name = "Display name is required";
    if (!form.description) newErrors.description = "Description is required";
    if (!form.category) newErrors.category = "Category is required";
    if (!form.version?.match(/^\d+\.\d+\.\d+/)) {
      newErrors.version = "Version must follow semver (e.g. 1.0.0)";
    }
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async () => {
    if (!validate()) return;

    try {
      await createExtension.mutateAsync(form as CreateExtensionInput);
      onSuccess?.();
      onClose();
    } catch {
      setErrors({ _global: "Failed to create extension. The name may already exist." });
    }
  };

  const Field = ({
    label,
    name,
    children,
  }: {
    label: string;
    name: string;
    children: React.ReactNode;
  }) => (
    <div className="flex flex-col gap-1">
      <label className="text-xs text-gray-400">{label}</label>
      {children}
      {errors[name] && (
        <p className="text-xs text-red-400">
          <AlertCircle className="mr-1 inline h-3 w-3" />
          {errors[name]}
        </p>
      )}
    </div>
  );

  return (
    <div className="flex flex-col gap-5">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg border border-white/10 bg-white/5">
            <Upload className="h-4 w-4 text-gray-400" />
          </div>
          <div>
            <h2 className="text-base font-semibold text-white">Upload Custom Extension</h2>
            <p className="text-xs text-gray-400">
              Register a private extension for your organization
            </p>
          </div>
        </div>
        <button onClick={onClose} className="rounded p-1 text-gray-400 hover:text-white">
          <X className="h-4 w-4" />
        </button>
      </div>

      {errors._global && (
        <div className="flex items-center gap-2 rounded-lg border border-red-500/20 bg-red-500/10 px-3 py-2 text-sm text-red-400">
          <AlertCircle className="h-4 w-4 shrink-0" />
          {errors._global}
        </div>
      )}

      {/* Form */}
      <div className="grid grid-cols-2 gap-4">
        <Field label="Extension Name *" name="name">
          <Input
            placeholder="my-extension"
            value={form.name ?? ""}
            onChange={(e) => update("name", e.target.value)}
            className={errors.name ? "border-red-500/50" : ""}
          />
        </Field>
        <Field label="Display Name *" name="display_name">
          <Input
            placeholder="My Extension"
            value={form.display_name ?? ""}
            onChange={(e) => update("display_name", e.target.value)}
            className={errors.display_name ? "border-red-500/50" : ""}
          />
        </Field>
      </div>

      <Field label="Description *" name="description">
        <textarea
          className={`min-h-20 w-full rounded-md border bg-transparent px-3 py-2 text-sm text-white placeholder:text-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-500 ${
            errors.description ? "border-red-500/50" : "border-white/10"
          }`}
          placeholder="Describe what this extension does..."
          value={form.description ?? ""}
          onChange={(e) => update("description", e.target.value)}
        />
      </Field>

      <div className="grid grid-cols-3 gap-4">
        <Field label="Category *" name="category">
          <select
            className={`h-9 w-full rounded-md border bg-gray-900 px-3 text-sm text-white focus:outline-none focus:ring-1 focus:ring-blue-500 ${
              errors.category ? "border-red-500/50" : "border-white/10"
            }`}
            value={form.category ?? ""}
            onChange={(e) => update("category", e.target.value)}
          >
            <option value="">Select...</option>
            {CATEGORIES.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Version *" name="version">
          <Input
            placeholder="1.0.0"
            value={form.version ?? ""}
            onChange={(e) => update("version", e.target.value)}
            className={errors.version ? "border-red-500/50" : ""}
          />
        </Field>
        <Field label="Scope" name="scope">
          <select
            className="h-9 w-full rounded-md border border-white/10 bg-gray-900 px-3 text-sm text-white focus:outline-none focus:ring-1 focus:ring-blue-500"
            value={form.scope ?? "PRIVATE"}
            onChange={(e) => update("scope", e.target.value as ExtensionScope)}
          >
            <option value="PRIVATE">Private</option>
            <option value="INTERNAL">Internal</option>
            <option value="PUBLIC">Public</option>
          </select>
        </Field>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <Field label="Author" name="author">
          <Input
            placeholder="author name or org"
            value={form.author ?? ""}
            onChange={(e) => update("author", e.target.value)}
          />
        </Field>
        <Field label="License" name="license">
          <Input
            placeholder="MIT, Apache-2.0, ..."
            value={form.license ?? ""}
            onChange={(e) => update("license", e.target.value)}
          />
        </Field>
      </div>

      {/* Tags */}
      <div className="flex flex-col gap-2">
        <label className="text-xs text-gray-400">Tags</label>
        <div className="flex gap-2">
          <Input
            placeholder="Add tag..."
            value={tagInput}
            onChange={(e) => setTagInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && addTag()}
            className="flex-1"
          />
          <Button variant="outline" size="sm" onClick={addTag}>
            <Plus className="h-3.5 w-3.5" />
          </Button>
        </div>
        {form.tags && form.tags.length > 0 && (
          <div className="flex flex-wrap gap-1.5">
            {form.tags.map((tag) => (
              <span
                key={tag}
                className="inline-flex items-center gap-1 rounded-full border border-white/10 px-2.5 py-0.5 text-xs text-gray-400"
              >
                <Tag className="h-2.5 w-2.5" />
                {tag}
                <button
                  onClick={() => removeTag(tag)}
                  className="ml-0.5 text-gray-600 hover:text-gray-300"
                >
                  <X className="h-2.5 w-2.5" />
                </button>
              </span>
            ))}
          </div>
        )}
      </div>

      {/* Dependencies */}
      <div className="flex flex-col gap-2">
        <label className="text-xs text-gray-400">Dependencies</label>
        <div className="flex gap-2">
          <Input
            placeholder="extension-name..."
            value={depInput}
            onChange={(e) => setDepInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && addDep()}
            className="flex-1"
          />
          <Button variant="outline" size="sm" onClick={addDep}>
            <Plus className="h-3.5 w-3.5" />
          </Button>
        </div>
        {form.dependencies && form.dependencies.length > 0 && (
          <div className="flex flex-wrap gap-1.5">
            {form.dependencies.map((dep) => (
              <span
                key={dep}
                className="inline-flex items-center gap-1 rounded border border-white/10 px-2 py-0.5 text-xs text-gray-300"
              >
                <Package className="h-2.5 w-2.5 text-gray-500" />
                {dep}
                <button
                  onClick={() => removeDep(dep)}
                  className="ml-0.5 text-gray-600 hover:text-gray-300"
                >
                  <X className="h-2.5 w-2.5" />
                </button>
              </span>
            ))}
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="flex items-center justify-end gap-3 border-t border-white/10 pt-4">
        <Button variant="outline" onClick={onClose}>
          Cancel
        </Button>
        <Button onClick={handleSubmit} disabled={createExtension.isPending}>
          {createExtension.isPending ? (
            <>
              <div className="mr-2 h-3.5 w-3.5 animate-spin rounded-full border-b border-white" />
              Registering...
            </>
          ) : (
            <>
              <CheckCircle className="mr-2 h-3.5 w-3.5" />
              Register Extension
            </>
          )}
        </Button>
      </div>
    </div>
  );
}
