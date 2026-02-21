import { useState } from "react";
import { X, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

interface TaskNotificationsProps {
  notifyOnFailure: boolean;
  notifyOnSuccess: boolean;
  notifyEmails: string[];
  onNotifyOnFailureChange: (v: boolean) => void;
  onNotifyOnSuccessChange: (v: boolean) => void;
  onNotifyEmailsChange: (emails: string[]) => void;
}

export function TaskNotifications({
  notifyOnFailure,
  notifyOnSuccess,
  notifyEmails,
  onNotifyOnFailureChange,
  onNotifyOnSuccessChange,
  onNotifyEmailsChange,
}: TaskNotificationsProps) {
  const [newEmail, setNewEmail] = useState("");

  const addEmail = () => {
    const email = newEmail.trim();
    if (!email || notifyEmails.includes(email)) return;
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) return;
    onNotifyEmailsChange([...notifyEmails, email]);
    setNewEmail("");
  };

  const removeEmail = (email: string) => {
    onNotifyEmailsChange(notifyEmails.filter((e) => e !== email));
  };

  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <label className="text-sm font-medium">Notify when</label>
        <div className="space-y-2">
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={notifyOnFailure}
              onChange={(e) => onNotifyOnFailureChange(e.target.checked)}
              className="h-4 w-4 rounded border-border"
            />
            <span className="text-sm">Task fails</span>
          </label>
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={notifyOnSuccess}
              onChange={(e) => onNotifyOnSuccessChange(e.target.checked)}
              className="h-4 w-4 rounded border-border"
            />
            <span className="text-sm">Task succeeds</span>
          </label>
        </div>
      </div>

      {(notifyOnFailure || notifyOnSuccess) && (
        <div className="space-y-2">
          <label className="text-sm font-medium">Email recipients</label>
          <div className="flex gap-2">
            <Input
              type="email"
              value={newEmail}
              onChange={(e) => setNewEmail(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  addEmail();
                }
              }}
              placeholder="user@example.com"
            />
            <Button type="button" variant="outline" size="icon" onClick={addEmail}>
              <Plus className="h-4 w-4" />
            </Button>
          </div>
          {notifyEmails.length > 0 && (
            <div className="flex flex-wrap gap-1.5 mt-1">
              {notifyEmails.map((email) => (
                <span
                  key={email}
                  className="inline-flex items-center gap-1 rounded-full bg-muted px-2.5 py-1 text-xs"
                >
                  {email}
                  <button
                    type="button"
                    onClick={() => removeEmail(email)}
                    className="text-muted-foreground hover:text-foreground"
                  >
                    <X className="h-3 w-3" />
                  </button>
                </span>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
