#!/usr/bin/env bash
set -euo pipefail

ENV="${1:-staging}"
TAG="$(git rev-parse --short HEAD)"

echo "Deploying nebula:${TAG} to ${ENV}"
docker build -t "nebula:${TAG}" .
kubectl --context "${ENV}" set image deployment/nebula scheduler="nebula:${TAG}"
kubectl --context "${ENV}" rollout status deployment/nebula --timeout=120s
