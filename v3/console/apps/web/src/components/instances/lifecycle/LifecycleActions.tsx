import { useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { useQueryClient } from "@tanstack/react-query";
import { Copy, RefreshCw, PauseCircle, PlayCircle, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { CloneInstanceDialog } from "./CloneInstanceDialog";
import { RedeployDialog } from "./RedeployDialog";
import { SuspendDialog } from "./SuspendDialog";
import { ResumeDialog } from "./ResumeDialog";
import { DestroyDialog } from "./DestroyDialog";
import type { Instance } from "@/types/instance";

interface LifecycleActionsProps {
  instance: Instance;
}

export function LifecycleActions({ instance }: LifecycleActionsProps) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [cloneOpen, setCloneOpen] = useState(false);
  const [redeployOpen, setRedeployOpen] = useState(false);
  const [suspendOpen, setSuspendOpen] = useState(false);
  const [resumeOpen, setResumeOpen] = useState(false);
  const [destroyOpen, setDestroyOpen] = useState(false);

  function handleCloneSuccess(clonedId: string) {
    void navigate({ to: "/instances/$id", params: { id: clonedId } });
  }

  function handleLifecycleSuccess() {
    void queryClient.invalidateQueries({ queryKey: ["instances"] });
  }

  function handleDestroySuccess() {
    void queryClient.invalidateQueries({ queryKey: ["instances"] });
    void navigate({ to: "/instances" });
  }

  const canSuspend = instance.status === "RUNNING";
  const canResume = instance.status === "SUSPENDED";
  const canDestroy = ["RUNNING", "SUSPENDED", "STOPPED", "ERROR"].includes(instance.status);

  return (
    <>
      <div className="flex items-center gap-2 flex-wrap">
        <Button variant="outline" size="sm" onClick={() => setRedeployOpen(true)} className="gap-2">
          <RefreshCw className="h-3.5 w-3.5" />
          Redeploy
        </Button>
        <Button variant="outline" size="sm" onClick={() => setCloneOpen(true)} className="gap-2">
          <Copy className="h-3.5 w-3.5" />
          Clone
        </Button>

        {canSuspend && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => setSuspendOpen(true)}
            className="gap-2 text-amber-600 hover:text-amber-700 hover:border-amber-300"
          >
            <PauseCircle className="h-3.5 w-3.5" />
            Suspend
          </Button>
        )}

        {canResume && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => setResumeOpen(true)}
            className="gap-2 text-green-600 hover:text-green-700 hover:border-green-300"
          >
            <PlayCircle className="h-3.5 w-3.5" />
            Resume
          </Button>
        )}

        {canDestroy && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => setDestroyOpen(true)}
            className="gap-2 text-destructive hover:text-destructive hover:border-destructive/50"
          >
            <Trash2 className="h-3.5 w-3.5" />
            Destroy
          </Button>
        )}
      </div>

      <CloneInstanceDialog
        instance={instance}
        open={cloneOpen}
        onClose={() => setCloneOpen(false)}
        onSuccess={handleCloneSuccess}
      />

      <RedeployDialog
        instance={instance}
        open={redeployOpen}
        onClose={() => setRedeployOpen(false)}
      />

      <SuspendDialog
        instance={instance}
        open={suspendOpen}
        onOpenChange={setSuspendOpen}
        onSuccess={handleLifecycleSuccess}
      />

      <ResumeDialog
        instance={instance}
        open={resumeOpen}
        onOpenChange={setResumeOpen}
        onSuccess={handleLifecycleSuccess}
      />

      <DestroyDialog
        instance={instance}
        open={destroyOpen}
        onOpenChange={setDestroyOpen}
        onSuccess={handleDestroySuccess}
      />
    </>
  );
}
