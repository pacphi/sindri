import { Shield, Clock, Bell } from "lucide-react";

export function AlertSettings() {
  return (
    <div className="space-y-6">
      <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-6">
        <div className="flex items-center gap-3 mb-4">
          <Clock className="h-5 w-5 text-indigo-400" />
          <h3 className="font-medium text-white">Evaluation Schedule</h3>
        </div>
        <p className="text-sm text-gray-400">
          Alert rules are evaluated every 60 seconds against the latest instance metrics and
          lifecycle state. The evaluation interval is fixed and runs automatically in the
          background.
        </p>
      </div>

      <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-6">
        <div className="flex items-center gap-3 mb-4">
          <Bell className="h-5 w-5 text-indigo-400" />
          <h3 className="font-medium text-white">Notification Delivery</h3>
        </div>
        <p className="text-sm text-gray-400">
          Notifications are dispatched asynchronously after an alert fires. Each configured channel
          for the matching rule receives the notification. Failed deliveries are logged but do not
          block other channels.
        </p>
        <dl className="mt-4 grid grid-cols-2 gap-4 text-sm">
          <div>
            <dt className="text-gray-500">Webhook</dt>
            <dd className="text-gray-300">HMAC-SHA256 signed POST with alert payload</dd>
          </div>
          <div>
            <dt className="text-gray-500">Slack</dt>
            <dd className="text-gray-300">Color-coded message attachment with severity</dd>
          </div>
          <div>
            <dt className="text-gray-500">Email</dt>
            <dd className="text-gray-300">Integration pending SMTP configuration</dd>
          </div>
          <div>
            <dt className="text-gray-500">In-App</dt>
            <dd className="text-gray-300">Logged to server; real-time push coming soon</dd>
          </div>
        </dl>
      </div>

      <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-6">
        <div className="flex items-center gap-3 mb-4">
          <Shield className="h-5 w-5 text-indigo-400" />
          <h3 className="font-medium text-white">Deduplication & Cooldown</h3>
        </div>
        <p className="text-sm text-gray-400">
          Each alert rule has a configurable cooldown period (default 5 minutes). During the
          cooldown, the same alert will not fire again for the same instance. When a condition
          clears, the active alert is automatically resolved by the evaluation engine.
        </p>
      </div>
    </div>
  );
}
