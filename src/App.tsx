import { HashRouter, Route, Routes } from "react-router-dom";
import AppLayout from "@/components/AppLayout";
import Applications from "@/pages/Applications";
import CareerProfile from "@/pages/CareerProfile";
import Contacts from "@/pages/Contacts";
import Dashboard from "@/pages/Dashboard";
import Placeholder from "@/pages/Placeholder";
import Settings from "@/pages/Settings";
import { SECTIONS } from "@/lib/sections";

const BUILT_SECTIONS = new Set(["/career-profile", "/contacts", "/applications"]);

export default function App() {
  return (
    <HashRouter>
      <Routes>
        <Route element={<AppLayout />}>
          <Route path="/" element={<Dashboard />} />
          <Route path="/career-profile" element={<CareerProfile />} />
          <Route path="/contacts" element={<Contacts />} />
          <Route path="/applications" element={<Applications />} />
          {SECTIONS.filter((s) => s.phase && !BUILT_SECTIONS.has(s.path)).map(
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
