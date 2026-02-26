import { useState, useEffect } from "react";
import { useCampaignStore } from "../store/campaignStore";
import { Plus, Play, Pause, Square, Archive, Users } from "lucide-react";

export function CampaignManager() {
  const { campaigns, loadCampaigns, createCampaign, controlCampaign, selectCampaign } = useCampaignStore();
  const [isCreating, setIsCreating] = useState(false);
  const [newCampaignName, setNewCampaignName] = useState("");
  const [newCampaignDesc, setNewCampaignDesc] = useState("");

  useEffect(() => {
    loadCampaigns();
  }, []);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newCampaignName.trim()) return;
    
    try {
      await createCampaign(newCampaignName, newCampaignDesc);
      setNewCampaignName("");
      setNewCampaignDesc("");
      setIsCreating(false);
    } catch (err) {
      alert("Failed to create campaign: " + err);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">Campaign Manager</h1>
          <p className="text-gray-400 mt-1">Manage red team engagement campaigns</p>
        </div>
        <button
          onClick={() => setIsCreating(true)}
          className="flex items-center gap-2 px-4 py-2 bg-red-600 hover:bg-red-700 text-white font-medium rounded-lg transition-colors"
        >
          <Plus className="w-5 h-5" />
          New Campaign
        </button>
      </div>

      {isCreating && (
        <div className="bg-gray-800 rounded-xl p-6 border border-gray-700">
          <h2 className="text-lg font-semibold text-white mb-4">Create New Campaign</h2>
          <form onSubmit={handleCreate} className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">
                Campaign Name
              </label>
              <input
                type="text"
                value={newCampaignName}
                onChange={(e) => setNewCampaignName(e.target.value)}
                className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:ring-2 focus:ring-red-500 focus:border-transparent"
                placeholder="e.g., Q1 External Assessment"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">
                Description
              </label>
              <textarea
                value={newCampaignDesc}
                onChange={(e) => setNewCampaignDesc(e.target.value)}
                className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:ring-2 focus:ring-red-500 focus:border-transparent"
                rows={3}
                placeholder="Campaign objectives and scope..."
              />
            </div>
            <div className="flex gap-3">
              <button
                type="submit"
                className="px-4 py-2 bg-red-600 hover:bg-red-700 text-white font-medium rounded-lg transition-colors"
              >
                Create Campaign
              </button>
              <button
                type="button"
                onClick={() => setIsCreating(false)}
                className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white font-medium rounded-lg transition-colors"
              >
                Cancel
              </button>
            </div>
          </form>
        </div>
      )}

      <div className="grid gap-4">
        {campaigns.map((campaign) => (
          <CampaignCard
            key={campaign.id}
            campaign={campaign}
            onControl={controlCampaign}
            onSelect={selectCampaign}
          />
        ))}
      </div>
    </div>
  );
}

function CampaignCard({
  campaign,
  onControl,
  onSelect,
}: {
  campaign: any;
  onControl: (id: string, action: any) => Promise<void>;
  onSelect: (id: string) => Promise<void>;
}) {
  const statusColors: Record<number, string> = {
    0: "bg-gray-500",
    1: "bg-green-500",
    2: "bg-yellow-500",
    3: "bg-orange-500",
    4: "bg-gray-600",
  };

  const statusLabels: Record<number, string> = {
    0: "Planning",
    1: "Active",
    2: "Paused",
    3: "Closing",
    4: "Archived",
  };

  const canStart = campaign.status === 0 || campaign.status === 2;
  const canPause = campaign.status === 1;
  const canClose = campaign.status === 1 || campaign.status === 2;
  const canArchive = campaign.status === 3;

  return (
    <div className="bg-gray-800 rounded-xl p-6 border border-gray-700">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-3 mb-2">
            <div className={`w-3 h-3 rounded-full ${statusColors[campaign.status]}`} />
            <h3 className="text-xl font-semibold text-white">{campaign.name}</h3>
            <span className="px-2 py-1 bg-gray-700 rounded text-xs text-gray-300">
              {statusLabels[campaign.status]}
            </span>
          </div>
          <p className="text-gray-400 mb-4">{campaign.description || "No description"}</p>
          
          <div className="flex items-center gap-6 text-sm text-gray-400">
            <div className="flex items-center gap-2">
              <Users className="w-4 h-4" />
              <span>Operator: {campaign.operator_id}</span>
            </div>
            <div>
              Created: {new Date(campaign.created_at).toLocaleDateString()}
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {canStart && (
            <button
              onClick={() => onControl(campaign.id, "start")}
              className="p-2 bg-green-600/20 hover:bg-green-600/30 text-green-400 rounded-lg transition-colors"
              title="Start Campaign"
            >
              <Play className="w-5 h-5" />
            </button>
          )}
          {canPause && (
            <button
              onClick={() => onControl(campaign.id, "pause")}
              className="p-2 bg-yellow-600/20 hover:bg-yellow-600/30 text-yellow-400 rounded-lg transition-colors"
              title="Pause Campaign"
            >
              <Pause className="w-5 h-5" />
            </button>
          )}
          {canClose && (
            <button
              onClick={() => onControl(campaign.id, "close")}
              className="p-2 bg-red-600/20 hover:bg-red-600/30 text-red-400 rounded-lg transition-colors"
              title="Close Campaign"
            >
              <Square className="w-5 h-5" />
            </button>
          )}
          {canArchive && (
            <button
              onClick={() => onControl(campaign.id, "archive")}
              className="p-2 bg-gray-600/20 hover:bg-gray-600/30 text-gray-400 rounded-lg transition-colors"
              title="Archive Campaign"
            >
              <Archive className="w-5 h-5" />
            </button>
          )}
          <button
            onClick={() => onSelect(campaign.id)}
            className="px-4 py-2 bg-red-600 hover:bg-red-700 text-white text-sm font-medium rounded-lg transition-colors"
          >
            Manage
          </button>
        </div>
      </div>
    </div>
  );
}