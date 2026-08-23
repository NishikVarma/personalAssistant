import { HashRouter, Route, Routes } from "react-router-dom";
import AppLayout from "@/components/AppLayout";
import CareerProfile from "@/pages/CareerProfile";
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
          <Route path="/career-profile" element={<CareerProfile />} />
          {SECTIONS.filter((s) => s.phase && s.path !== "/career-profile").map(
            ({ path, label, phase, icon }) => (
              <Route
                key={path}
                path={path}
                element={<Placeholder title={label} phase={phase!} icon={icon} />}
              />
            ),
          )}
          <Route path="/settings" element={<Settings />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}
