type Props = { connected: boolean; total: number };

export function Header({ connected, total }: Props) {
  return (
    <header className="header">
      <h1>Nebula</h1>
      <span className={connected ? "dot online" : "dot offline"} />
      <span>{total.toLocaleString()} jobs</span>
    </header>
  );
}
