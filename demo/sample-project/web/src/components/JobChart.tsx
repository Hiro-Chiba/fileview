import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis } from "recharts";
import type { Job } from "../hooks/useJobStream";

export function JobChart({ jobs }: { jobs: Job[] }) {
  // TODO: bucket by minute once the gateway sends timestamps
  const data = jobs.map((job, i) => ({ i, duration: job.durationMs }));
  return (
    <ResponsiveContainer width="100%" height={320}>
      <AreaChart data={data}>
        <XAxis dataKey="i" hide />
        <Tooltip />
        <Area type="monotone" dataKey="duration" stroke="#8b5cf6" fill="#8b5cf633" />
      </AreaChart>
    </ResponsiveContainer>
  );
}
