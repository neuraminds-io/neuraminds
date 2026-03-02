# Alert Routing Matrix

## Severity Targets

| Severity | Definition | Ack SLO | Escalate After | Resolve Target |
| --- | --- | --- | --- | --- |
| P1 | Full outage, security event, or settlement integrity risk | 5 minutes | 10 minutes | 60 minutes |
| P2 | Major degradation impacting trading or auth flows | 15 minutes | 30 minutes | 4 hours |
| P3 | Partial degradation with workaround | 60 minutes | 120 minutes | 24 hours |
| P4 | Low-impact issue, no immediate user-facing degradation | Business hours | Next business day | 3 business days |

## Routing Matrix

| Signal | Severity | Primary Owner | Secondary Owner | Pager Policy | Channel |
| --- | --- | --- | --- | --- | --- |
| Synthetic probe failure (`api_health` or `web_home`) | P1 | Platform on-call | Backend on-call | `platform-primary` | `#alerts-prod` |
| `/health/detailed` component unhealthy | P1 | Backend on-call | Platform on-call | `backend-primary` | `#alerts-prod` |
| 5xx > 2% for 5 minutes | P1 | Backend on-call | Platform on-call | `backend-primary` | `#alerts-prod` |
| p95 latency breach (>= 500ms for 10 minutes) | P2 | Backend on-call | Platform on-call | `backend-primary` | `#alerts-prod` |
| Auth refresh failure spike | P2 | Backend on-call | Frontend on-call | `backend-primary` | `#alerts-prod` |
| RPC dependency degraded with fallback active | P3 | Platform on-call | Backend on-call | `platform-primary` | `#alerts-infra` |
| Non-critical batch/reconciliation lag | P3 | Backend on-call | Platform on-call | `backend-primary` | `#alerts-ops` |

## Escalation Windows (UTC)

| Window | Primary | Secondary | Executive Escalation |
| --- | --- | --- | --- |
| 00:00-08:00 | Platform on-call | Backend on-call | Operations lead |
| 08:00-16:00 | Backend on-call | Platform on-call | Engineering lead |
| 16:00-24:00 | Platform on-call | Frontend on-call | Operations lead |

## Escalation Policy

1. If the primary owner does not acknowledge within the Ack SLO, page the secondary owner.
2. If no mitigation begins within escalation window, page executive escalation contact.
3. For P1 incidents, open incident channel immediately and assign Incident Commander.
