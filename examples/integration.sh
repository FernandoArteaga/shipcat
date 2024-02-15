#!/bin/bash
set -ex

# Start vault
docker run --cap-add=IPC_LOCK -e 'VAULT_DEV_ROOT_TOKEN_ID=myroot' -e 'VAULT_DEV_LISTEN_ADDRESS=0.0.0.0:8200' -p 8200:8200 -d --rm --name vault vault:0.11.3
export VAULT_ADDR=http://127.0.0.1:8200
export VAULT_TOKEN=myroot

# Use kv1
sleep 5 # wait for vault
vault secrets disable secret
vault secrets enable -version=1 -path=secret kv

# Start a database for the webapp service
helm repo add bitnami https://charts.bitnami.com/bitnami
helm repo update
helm install postgresql bitnami/postgresql -n apps --create-namespace --set image.tag=10.14.0 --set auth.password=pw --set auth.database=webapp
# Write its database password in vault
vault write secret/webapp/DATABASE_URL value=postgres://postgres:pw@postgresql.apps.svc.cluster.local:5432/webapp
