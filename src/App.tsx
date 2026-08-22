import { HashRouter, Route, Routes } from "react-router-dom";
import AppLayout from "@/components/AppLayout";
import Dashboard from "@/pages/Dashboard";
import Placeholder from "@/pages/Placeholder";
import Settings from "@/pages/Settings";
import { SECTIONS } from "@/lib/sections";

export default function App() {
  return (
    <HashRouter>
      <Routes>
        <Route element={<AppLayout />}>
          <Route path="/" element={<Dashboard />} />
          {SECTIONS.filter((s) => s.phase).map(({ path, label, phase, icon }) => (
            <Route
              key={path}
              path={path}
              element={<Placeholder title={label} phase={phase!} icon={icon} />}
            />
          ))}
          <Route path="/settings" element={<Settings />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}
