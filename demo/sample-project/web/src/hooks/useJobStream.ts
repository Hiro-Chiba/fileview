import { useEffect, useState } from "react";

export type Job = {
  id: number;
  name: string;
  status: "queued" | "running" | "failed" | "done";
  durationMs: number;
};

export function useJobStream(url: string) {
  const [jobs, setJobs] = useState<Job[]>([]);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    const ws = new WebSocket(url);
    ws.onopen = () => setConnected(true);
    ws.onclose = () => setConnected(false);
    ws.onmessage = (event) => {
      const job: Job = JSON.parse(event.data);
      setJobs((prev) => [...prev.filter((j) => j.id !== job.id), job]);
    };
    return () => ws.close();
  }, [url]);

  return { jobs, connected };
}
