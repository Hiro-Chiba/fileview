import polars as pl

from ml.analyze import summarise


def test_summarise_reports_one_row_per_job():
    df = pl.DataFrame({"job": ["build", "build", "test"], "duration_ms": [100, 300, 50]})
    reports = summarise(df)
    assert sorted(r.job for r in reports) == ["build", "test"]
