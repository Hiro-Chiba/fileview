"""Summarise job durations and flag slow job types."""

from dataclasses import dataclass
from pathlib import Path

import polars as pl


@dataclass
class Report:
    job: str
    p50_ms: float
    p95_ms: float


def load(path: Path) -> pl.DataFrame:
    return pl.read_csv(path).filter(pl.col("status") == "done")


def summarise(df: pl.DataFrame) -> list[Report]:
    grouped = df.group_by("job").agg(
        pl.col("duration_ms").quantile(0.5).alias("p50"),
        pl.col("duration_ms").quantile(0.95).alias("p95"),
    )
    # TODO: compare against last week's baseline instead of a fixed threshold
    return [Report(r["job"], r["p50"], r["p95"]) for r in grouped.iter_rows(named=True)]


if __name__ == "__main__":
    for report in summarise(load(Path("data/jobs.csv"))):
        flag = " <- slow" if report.p95_ms > 5_000 else ""
        print(f"{report.job:<12} p50={report.p50_ms:>7.0f}ms p95={report.p95_ms:>7.0f}ms{flag}")
