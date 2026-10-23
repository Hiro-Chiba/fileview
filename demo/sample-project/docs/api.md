# HTTP API

## `POST /jobs`

Submit a job. Returns `202 Accepted`.

```json
{ "name": "build", "priority": 9, "depends_on": [41, 42] }
```

## `GET /healthz`

Returns the gateway status and the number of requests served.

| Field | Type | Description |
|-------|------|-------------|
| `ok` | bool | Always `true` when the gateway is up |
| `requests` | int | Requests served since start |
