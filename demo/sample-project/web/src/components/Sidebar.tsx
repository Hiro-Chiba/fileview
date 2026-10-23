const FILTERS = ["all", "running", "failed"] as const;
type Filter = (typeof FILTERS)[number];

export function Sidebar({ filter, onChange }: { filter: Filter; onChange: (f: Filter) => void }) {
  return (
    <nav className="sidebar">
      {FILTERS.map((f) => (
        <button key={f} aria-pressed={f === filter} onClick={() => onChange(f)}>
          {f}
        </button>
      ))}
    </nav>
  );
}
