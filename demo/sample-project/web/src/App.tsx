import { useState } from "react";
import { Header } from "./components/Header";
import { Sidebar } from "./components/Sidebar";
import { JobChart } from "./components/JobChart";
import { useJobStream } from "./hooks/useJobStream";

export function App() {
  const [filter, setFilter] = useState<"all" | "running" | "failed">("all");
  const { jobs, connected } = useJobStream("ws://localhost:8080/jobs");

  const visible = jobs.filter((job) => filter === "all" || job.status === filter);

  return (
    <div className="layout">
      <Header connected={connected} total={jobs.length} />
      <Sidebar filter={filter} onChange={setFilter} />
      <main>
        <JobChart jobs={visible} />
      </main>
    </div>
  );
}
