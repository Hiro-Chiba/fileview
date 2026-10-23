from sklearn.ensemble import GradientBoostingRegressor
from sklearn.model_selection import train_test_split

import polars as pl

df = pl.read_csv("data/jobs.csv")
X = df.select(["priority", "inputs_mb", "workers"]).to_numpy()
y = df["duration_ms"].to_numpy()

X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.2, random_state=7)
model = GradientBoostingRegressor(n_estimators=200, max_depth=3).fit(X_train, y_train)
print(f"R^2 on held-out jobs: {model.score(X_test, y_test):.3f}")
