import { useEffect } from "react";
import { Routes, Route, Navigate } from "react-router-dom";
import { useConnectionStore } from "./store/connectionStore";
import { useEventStore } from "./store/eventStore";

// Layouts
import { MainLayout } from "./components/layout/MainLayout";

// Pages
import { Dashboard } from "./pages/Dashboard";
import { CampaignManager } from "./pages/CampaignManager";
import { SessionViewer } from "./pages/SessionViewer";
import { TaskBuilder } from "./pages/TaskBuilder";
import { Settings } from "./pages/Settings";
import { Login } from "./pages/Login";

function App() {
  const { isConnected, checkConnection } = useConnectionStore();
  const { startEventStream } = useEventStore();

  useEffect(() => {
    checkConnection();
  }, []);

  useEffect(() => {
    if (isConnected) {
      startEventStream();
    }
  }, [isConnected]);

  if (!isConnected) {
    return <Login />;
  }

  return (
    <MainLayout>
      <Routes>
        <Route path="/" element={<Navigate to="/dashboard" />} />
        <Route path="/dashboard" element={<Dashboard />} />
        <Route path="/campaigns" element={<CampaignManager />} />
        <Route path="/campaigns/:id" element={<SessionViewer />} />
        <Route path="/tasks" element={<TaskBuilder />} />
        <Route path="/settings" element={<Settings />} />
      </Routes>
    </MainLayout>
  );
}

export default App;