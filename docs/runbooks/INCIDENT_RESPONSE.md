# Polyguard Incident Response Plan

## Document Control

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-01-22 | Automated | Initial version |

## Overview

This document defines the incident response process for Polyguard, including classification, escalation, communication, and resolution procedures.

---

## Incident Classification

### Severity Levels

| Level | Name | Definition | Response Time | Examples |
|-------|------|------------|---------------|----------|
| P1 | Critical | Complete service outage or data breach | 15 min | API down, database corrupted, security breach |
| P2 | High | Major feature unavailable, significant degradation | 30 min | Order placement failing, settlement stuck |
| P3 | Medium | Minor feature issue, limited user impact | 4 hours | Slow responses, intermittent errors |
| P4 | Low | Cosmetic issues, single user reports | 24 hours | UI bugs, documentation errors |

### Impact Assessment

| Factor | Questions |
|--------|-----------|
| **Users Affected** | How many users are impacted? All, subset, or single? |
| **Revenue Impact** | Is trading blocked? Are settlements failing? |
| **Data Integrity** | Is data at risk? Are positions accurate? |
| **Security** | Is there unauthorized access? Are funds at risk? |

---

## Response Process

### Phase 1: Detection & Triage (0-15 min)

#### 1.1 Alert Received
- Acknowledge alert in PagerDuty within 5 minutes
- Join incident Slack channel: `#incident-YYYYMMDD`
- Initial assessment of severity

#### 1.2 Incident Commander Assignment
For P1/P2 incidents, assign an Incident Commander (IC):
- IC owns the incident until resolution
- IC coordinates response, not technical fixes
- IC manages communication

#### 1.3 Initial Triage
```
Triage Checklist:
[ ] Service status check: kubectl -n polyguard get pods
[ ] Health endpoint: curl https://api.polyguard.cc/health/deep
[ ] Recent deployments: kubectl -n polyguard rollout history
[ ] Error logs: kubectl -n polyguard logs -l app=polyguard-api --since=10m
[ ] External dependencies: Solana RPC, database, Redis
```

### Phase 2: Investigation (15-60 min)

#### 2.1 Gather Context
```
Investigation Questions:
- When did the issue start?
- What changed recently? (deploys, config, traffic)
- Is it affecting all users or a subset?
- Is the issue intermittent or constant?
- Are there correlated alerts?
```

#### 2.2 Assemble Team
| Role | Responsibility |
|------|----------------|
| Incident Commander | Coordination, communication |
| Technical Lead | Root cause investigation |
| Communications Lead | External updates |
| Subject Matter Expert | Domain-specific knowledge |

#### 2.3 Document Timeline
Keep a running log in Slack or incident doc:
```
[14:32 UTC] Alert received - API errors > 5%
[14:35 UTC] IC assigned: @alice
[14:38 UTC] Initial triage: 503 errors on order endpoint
[14:42 UTC] Database connection pool exhausted
[14:45 UTC] Increased pool size, deploying fix
```

### Phase 3: Mitigation (30-120 min)

#### 3.1 Immediate Actions
| Issue Type | Mitigation |
|------------|------------|
| Bad deploy | Rollback: `kubectl rollout undo deployment/polyguard-api-blue` |
| Resource exhaustion | Scale up: `kubectl scale deployment --replicas=10` |
| External dependency | Enable fallback, circuit breaker |
| Data corruption | Switch to read-only mode |

#### 3.2 Communication
- Update status page
- Notify affected users
- Internal stakeholder updates every 30 min for P1

### Phase 4: Resolution

#### 4.1 Verify Fix
```
Verification Checklist:
[ ] Error rates returned to baseline
[ ] Health checks passing
[ ] No new alerts
[ ] Smoke tests passing
[ ] User-reported issues resolved
```

#### 4.2 Close Incident
- Update status page to "Resolved"
- Send final internal communication
- Schedule post-mortem

---

## Communication Templates

### External (Status Page)

#### Investigating
```
We are investigating reports of [ISSUE DESCRIPTION].

Impact: [WHO IS AFFECTED]
Status: Investigating
Updated: [TIME UTC]
```

#### Identified
```
We have identified the cause of [ISSUE].

Root Cause: [BRIEF DESCRIPTION]
Impact: [WHO IS AFFECTED]
Status: Fix in progress
ETA: [ESTIMATE]
Updated: [TIME UTC]
```

#### Resolved
```
The issue affecting [SERVICE] has been resolved.

Root Cause: [BRIEF DESCRIPTION]
Duration: [X hours/minutes]
Status: Resolved
Updated: [TIME UTC]

We apologize for any inconvenience. A detailed post-mortem will follow.
```

### Internal (Slack)

#### Incident Start
```
:alert: *INCIDENT DECLARED* :alert:

*Severity:* P[X]
*Issue:* [DESCRIPTION]
*IC:* @[NAME]
*Channel:* #incident-YYYYMMDD

Initial Assessment:
- [OBSERVATION 1]
- [OBSERVATION 2]

Next Steps:
- [ ] [ACTION]
```

#### Status Update
```
:information_source: *INCIDENT UPDATE* - [TIME UTC]

*Status:* [Investigating/Mitigating/Monitoring]
*Progress:*
- [WHAT WE'VE DONE]
- [WHAT WE'VE LEARNED]

*Next Steps:*
- [ ] [ACTION]

*ETA:* [ESTIMATE]
```

#### Incident Closed
```
:white_check_mark: *INCIDENT RESOLVED* - [TIME UTC]

*Duration:* [X] minutes
*Root Cause:* [BRIEF DESCRIPTION]
*Resolution:* [WHAT FIXED IT]

*Action Items:*
- [ ] Post-mortem scheduled for [DATE]
- [ ] [OTHER ITEMS]

Thank you to everyone involved.
```

---

## Post-Mortem Process

### Timeline
- **Within 24 hours**: Create post-mortem document
- **Within 48 hours**: Hold post-mortem meeting
- **Within 1 week**: Action items assigned and tracked

### Post-Mortem Template

```markdown
# Post-Mortem: [INCIDENT TITLE]

## Summary
[One paragraph description of what happened]

## Impact
- Duration: [X minutes/hours]
- Users affected: [NUMBER]
- Orders affected: [NUMBER]
- Revenue impact: [ESTIMATE]

## Timeline
| Time (UTC) | Event |
|------------|-------|
| HH:MM | [EVENT] |

## Root Cause
[Detailed explanation of what caused the incident]

## Resolution
[What was done to fix the issue]

## What Went Well
- [ITEM]

## What Could Be Improved
- [ITEM]

## Action Items
| Item | Owner | Due Date | Status |
|------|-------|----------|--------|
| [ITEM] | @[NAME] | [DATE] | Open |

## Lessons Learned
[Key takeaways]
```

### Blameless Culture
- Focus on systems, not individuals
- Ask "how did this happen?" not "who caused this?"
- Assume everyone had good intentions
- Share learnings broadly

---

## Escalation Matrix

### On-Call Rotation
| Time | Primary | Secondary |
|------|---------|-----------|
| Weekdays 9-18 UTC | Engineering Lead | Senior Engineer |
| After Hours | On-Call Rotation | Engineering Lead |
| Weekends | On-Call Rotation | Senior Engineer |

### Escalation Path
```
L1: On-Call Engineer (0-15 min)
  ↓
L2: Engineering Lead (15-30 min for P1)
  ↓
L3: CTO / Security Lead (30+ min for P1, or any security issue)
```

### When to Escalate
- Unable to diagnose after 15 minutes
- Issue requires elevated access
- Security implications
- PR/Legal implications
- Extended customer impact

---

## Tools & Access

### Monitoring
- **Grafana**: metrics.polyguard.cc
- **PagerDuty**: pagerduty.com/polyguard
- **Status Page**: status.polyguard.cc

### Runbooks
- [Disaster Recovery](./DISASTER_RECOVERY.md)
- [Database Runbook](./DATABASE.md)
- [Deployment Runbook](./DEPLOYMENT.md)

### Access Requirements
All on-call engineers must have:
- [ ] kubectl access to production cluster
- [ ] Database read access
- [ ] PagerDuty account
- [ ] Grafana access
- [ ] AWS console access (for S3 backups)

---

## Regular Review

This plan should be reviewed:
- After every P1/P2 incident
- Quarterly (minimum)
- When major infrastructure changes occur

### Review Checklist
- [ ] Contact list current
- [ ] Escalation paths accurate
- [ ] Runbooks tested and working
- [ ] Tools access verified
- [ ] Communication templates updated
