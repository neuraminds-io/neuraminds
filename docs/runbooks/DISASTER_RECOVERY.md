# Polyguard Disaster Recovery Runbook

## Document Control

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-01-22 | Automated | Initial version |

## Overview

This document provides procedures for recovering Polyguard services from various disaster scenarios.

### Recovery Time Objectives (RTO)

| System | RTO | RPO |
|--------|-----|-----|
| API Backend | 15 min | 5 min |
| Database | 30 min | 1 min |
| Redis Cache | 5 min | N/A |
| Solana On-chain | N/A | N/A |
| Frontend | 10 min | 0 |

### Critical Dependencies

1. **PostgreSQL Database** - User accounts, orders, positions, market metadata
2. **Redis** - Session cache, rate limiting, real-time data
3. **Solana RPC** - Blockchain interactions
4. **Keeper Wallet** - Settlement transactions

---

## Scenario 1: Database Failure

### Symptoms
- API returns 500 errors on data operations
- Health check `/health/deep` shows database degraded/unhealthy
- Logs show "Database connection failed" errors

### Recovery Steps

#### 1.1 Assess the Situation
```bash
# Check database status
kubectl -n polyguard exec -it $(kubectl get pod -l app=postgres -o jsonpath='{.items[0].metadata.name}') -- pg_isready

# Check for replication lag (if using replicas)
kubectl -n polyguard exec -it postgres-0 -- psql -c "SELECT * FROM pg_stat_replication;"

# Review recent logs
kubectl -n polyguard logs -l app=postgres --tail=100
```

#### 1.2 Attempt Connection Recovery
```bash
# Restart database pod
kubectl -n polyguard rollout restart statefulset/postgres

# Wait for ready
kubectl -n polyguard wait --for=condition=Ready pod -l app=postgres --timeout=300s
```

#### 1.3 Failover to Replica (if primary is dead)
```bash
# Promote replica to primary
kubectl -n polyguard exec -it postgres-replica-0 -- pg_ctl promote

# Update service to point to new primary
kubectl -n polyguard patch service postgres -p '{"spec":{"selector":{"role":"primary"}}}'
```

#### 1.4 Restore from Backup
```bash
# List available backups
aws s3 ls s3://polyguard-backups/postgres/

# Download latest backup
aws s3 cp s3://polyguard-backups/postgres/LATEST.sql.gz ./

# Restore
gunzip LATEST.sql.gz
kubectl -n polyguard exec -i postgres-0 -- psql < LATEST.sql
```

### Post-Recovery Verification
```bash
# Verify data integrity
kubectl -n polyguard exec -it postgres-0 -- psql -c "SELECT COUNT(*) FROM orders;"

# Check API health
curl https://api.polyguard.cc/health/deep
```

---

## Scenario 2: Redis Failure

### Symptoms
- Rate limiting not working
- Session data lost
- WebSocket connections dropping

### Recovery Steps

#### 2.1 Restart Redis
```bash
kubectl -n polyguard rollout restart deployment/redis
kubectl -n polyguard wait --for=condition=Ready pod -l app=redis --timeout=60s
```

#### 2.2 Clear and Rebuild (if corrupted)
```bash
# Delete corrupted data
kubectl -n polyguard exec -it redis-0 -- redis-cli FLUSHALL

# Restart API pods to reestablish connections
kubectl -n polyguard rollout restart deployment/polyguard-api-blue
```

### Note
Redis data is ephemeral. Users may need to re-authenticate. No data loss for orders/positions (stored in PostgreSQL).

---

## Scenario 3: API Service Failure

### Symptoms
- 502/503 errors from load balancer
- All pods showing CrashLoopBackOff
- Health checks failing

### Recovery Steps

#### 3.1 Check Pod Status
```bash
kubectl -n polyguard get pods -l app=polyguard-api
kubectl -n polyguard describe pod <pod-name>
kubectl -n polyguard logs <pod-name> --previous
```

#### 3.2 Rollback Deployment
```bash
# List rollout history
kubectl -n polyguard rollout history deployment/polyguard-api-blue

# Rollback to previous version
kubectl -n polyguard rollout undo deployment/polyguard-api-blue

# Or rollback to specific revision
kubectl -n polyguard rollout undo deployment/polyguard-api-blue --to-revision=3
```

#### 3.3 Emergency Static Response (if all else fails)
```bash
# Deploy emergency maintenance page
kubectl -n polyguard apply -f infra/k8s/maintenance-mode.yaml
```

---

## Scenario 4: Solana RPC Issues

### Symptoms
- Order settlement failing
- Position claims stuck
- Logs show "Solana RPC error"

### Recovery Steps

#### 4.1 Check RPC Status
```bash
# Test RPC connectivity
curl https://api.mainnet-beta.solana.com -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

#### 4.2 Switch RPC Provider
```bash
# Update ConfigMap
kubectl -n polyguard patch configmap polyguard-config -p '{"data":{"solana-rpc-url":"https://backup-rpc.provider.com"}}'

# Restart API pods
kubectl -n polyguard rollout restart deployment/polyguard-api-blue
```

#### 4.3 RPC Fallback Order
1. Primary: Helius
2. Secondary: QuickNode
3. Tertiary: Public RPC (rate limited)

---

## Scenario 5: Keeper Wallet Compromise

### Symptoms
- Unauthorized transactions from keeper wallet
- Unusual settlement patterns

### Immediate Actions

#### 5.1 Disable Keeper Operations
```bash
# Set API to read-only mode
kubectl -n polyguard set env deployment/polyguard-api-blue READONLY_MODE=true

# Rotate keeper key immediately (requires on-chain program update)
```

#### 5.2 Investigate
```bash
# Check recent keeper transactions
solana transaction-history <KEEPER_ADDRESS>

# Review settlement logs
kubectl -n polyguard logs -l app=polyguard-api --since=24h | grep -i "settle"
```

#### 5.3 Recovery
1. Generate new keeper keypair
2. Update on-chain program authority
3. Update Kubernetes secrets
4. Deploy with new key
5. Post-mortem analysis

---

## Scenario 6: Complete Cluster Loss

### Recovery Procedure

#### 6.1 Provision New Cluster
```bash
# Using Terraform
cd infra/terraform
terraform apply -var-file=production.tfvars
```

#### 6.2 Restore Core Infrastructure
```bash
# Install ingress controller
kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/aws/deploy.yaml

# Install cert-manager
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml

# Install external-secrets-operator
helm install external-secrets external-secrets/external-secrets -n external-secrets --create-namespace
```

#### 6.3 Restore Database
```bash
# Deploy PostgreSQL
kubectl apply -f infra/k8s/postgres/

# Restore from backup
aws s3 cp s3://polyguard-backups/postgres/LATEST.sql.gz ./
kubectl -n polyguard exec -i postgres-0 -- psql < LATEST.sql
```

#### 6.4 Deploy Application
```bash
kubectl apply -f infra/k8s/deployment.yaml
kubectl apply -f infra/k8s/secrets.yaml
```

#### 6.5 Verify Recovery
```bash
# Check all services
kubectl -n polyguard get pods,svc,ingress

# Test API
curl https://api.polyguard.cc/health/deep

# Run smoke tests
k6 run --env SCENARIO=smoke tests/load/k6-config.js
```

---

## Backup Procedures

### Database Backup (Automated)

Backups run automatically via CronJob:

```yaml
# infra/k8s/backup-cronjob.yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: postgres-backup
  namespace: polyguard
spec:
  schedule: "0 */6 * * *"  # Every 6 hours
  jobTemplate:
    spec:
      template:
        spec:
          containers:
            - name: backup
              image: postgres:15
              command:
                - /bin/sh
                - -c
                - |
                  pg_dump $DATABASE_URL | gzip | aws s3 cp - s3://polyguard-backups/postgres/$(date +%Y%m%d-%H%M%S).sql.gz
              env:
                - name: DATABASE_URL
                  valueFrom:
                    secretKeyRef:
                      name: polyguard-secrets
                      key: database-url
          restartPolicy: OnFailure
```

### Manual Backup
```bash
# Create immediate backup
kubectl -n polyguard create job --from=cronjob/postgres-backup manual-backup-$(date +%s)
```

---

## Communication Templates

### Status Page Update
```
[INVESTIGATING] We are currently investigating issues with [SERVICE].
Updated: [TIME UTC]

[IDENTIFIED] The issue has been identified. We are working on a fix.
Impact: [DESCRIPTION]
Updated: [TIME UTC]

[MONITORING] A fix has been implemented. We are monitoring for stability.
Updated: [TIME UTC]

[RESOLVED] The issue has been resolved. All systems operational.
Duration: [X] minutes
Updated: [TIME UTC]
```

### Internal Escalation
```
Subject: [P1] Polyguard Service Degradation

Impact: [DESCRIPTION]
Start Time: [TIME UTC]
Current Status: [STATUS]
ETA: [ESTIMATE]

Actions Taken:
1. [ACTION]
2. [ACTION]

Next Steps:
1. [STEP]
```

---

## Contact List

| Role | Name | Contact |
|------|------|---------|
| On-Call Primary | Rotation | PagerDuty |
| On-Call Secondary | Rotation | PagerDuty |
| Infrastructure Lead | TBD | Slack @infra |
| Security Lead | TBD | Slack @security |
| Database Admin | TBD | Slack @dba |

---

## Post-Incident

After any incident:

1. **Stabilize** - Ensure services are stable
2. **Document** - Record timeline, actions, and outcomes
3. **Communicate** - Update stakeholders and status page
4. **Review** - Schedule post-mortem within 48 hours
5. **Improve** - Create action items to prevent recurrence
