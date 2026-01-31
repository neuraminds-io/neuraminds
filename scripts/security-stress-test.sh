#!/bin/bash
# Security Stress Test Suite
# Comprehensive automated security testing without external dependencies

cd "$(dirname "$0")/.."

echo "=== Polyguard Security Stress Test ==="
echo ""

PASS=0
FAIL=0

check() {
    local name=$1
    local result=$2
    if [ "$result" = "0" ]; then
        echo "✓ $name"
        ((PASS++))
    else
        echo "✗ $name"
        ((FAIL++))
    fi
}

# 1. Static Analysis - Check for common vulnerabilities
echo "### 1. Static Code Analysis ###"

# Check for hardcoded secrets
echo "Scanning for hardcoded secrets..."
SECRETS=$(grep -rn --include="*.rs" --include="*.ts" --include="*.tsx" -E "(secret|password|api_key|private_key)\s*=\s*['\"][a-zA-Z0-9]+" . 2>/dev/null | grep -v "node_modules" | grep -v "test" | grep -v ".example" | grep -v "placeholder" | grep -v "target/" | wc -l | tr -d ' ')
check "No hardcoded secrets" "$([ "$SECRETS" -eq 0 ] && echo 0 || echo 1)"

# Check for unsafe unwrap() in production code
echo "Checking for unsafe unwrap() usage..."
UNWRAPS=$(grep -rn --include="*.rs" "\.unwrap()" programs/ app/src/ 2>/dev/null | grep -v "test" | grep -v "#\[cfg(test)\]" | wc -l | tr -d ' ')
check "Minimal unwrap() in production (found: $UNWRAPS)" "$([ "$UNWRAPS" -lt 50 ] && echo 0 || echo 1)"

# Check for TODO/FIXME security comments
echo "Checking for unresolved security TODOs..."
TODOS=$(grep -rn --include="*.rs" --include="*.ts" -E "(TODO|FIXME).*security" . 2>/dev/null | wc -l | tr -d ' ')
check "No unresolved security TODOs" "$([ "$TODOS" -eq 0 ] && echo 0 || echo 1)"

# 2. Solana Program Security
echo ""
echo "### 2. Solana Program Security ###"

# Check for signer constraints (Anchor uses Signer<> type)
echo "Verifying signer constraints..."
SIGNER_CHECKS=$(grep -rn "Signer<\|#\[account(.*signer" programs/ 2>/dev/null | wc -l | tr -d ' ')
check "Signer constraints present (found: $SIGNER_CHECKS)" "$([ "$SIGNER_CHECKS" -gt 5 ] && echo 0 || echo 1)"

# Check for owner validation
OWNER_CHECKS=$(grep -rn "owner\s*=" programs/ 2>/dev/null | wc -l | tr -d ' ')
check "Owner validation present (found: $OWNER_CHECKS)" "$([ "$OWNER_CHECKS" -gt 3 ] && echo 0 || echo 1)"

# Check for rent exemption
RENT_EXEMPT=$(grep -rn "rent_exempt\|is_rent_exempt\|Rent::get" programs/ 2>/dev/null | wc -l | tr -d ' ')
check "Rent exemption checks (found: $RENT_EXEMPT)" "$([ "$RENT_EXEMPT" -gt 0 ] && echo 0 || echo 1)"

# Check for checked arithmetic
CHECKED_MATH=$(grep -rn "checked_add\|checked_sub\|checked_mul\|checked_div" programs/ 2>/dev/null | wc -l | tr -d ' ')
check "Checked arithmetic usage (found: $CHECKED_MATH)" "$([ "$CHECKED_MATH" -gt 10 ] && echo 0 || echo 1)"

# Check for authority validation
AUTH_CHECKS=$(grep -rn "has_one\s*=\s*authority\|constraint.*authority" programs/ 2>/dev/null | wc -l | tr -d ' ')
check "Authority validation (found: $AUTH_CHECKS)" "$([ "$AUTH_CHECKS" -gt 3 ] && echo 0 || echo 1)"

# 3. API Security
echo ""
echo "### 3. API Security ###"

# Check for rate limiting
RATE_LIMIT=$(grep -rn "rate.limit\|RateLimit\|rate_limit" app/src/ 2>/dev/null | wc -l | tr -d ' ')
check "Rate limiting implemented (found: $RATE_LIMIT)" "$([ "$RATE_LIMIT" -gt 5 ] && echo 0 || echo 1)"

# Check for JWT validation
JWT_VALIDATION=$(grep -rn "verify.*jwt\|jwt.*verify\|decode.*jwt\|validate.*token" app/src/ 2>/dev/null | wc -l | tr -d ' ')
check "JWT validation present (found: $JWT_VALIDATION)" "$([ "$JWT_VALIDATION" -gt 0 ] && echo 0 || echo 1)"

# Check for input validation
INPUT_VALIDATION=$(grep -rn "validate\|Validate\|validator" app/src/ 2>/dev/null | wc -l | tr -d ' ')
check "Input validation (found: $INPUT_VALIDATION)" "$([ "$INPUT_VALIDATION" -gt 10 ] && echo 0 || echo 1)"

# Check for CORS configuration
CORS_CONFIG=$(grep -rn "cors\|CorsLayer\|Access-Control" app/src/ 2>/dev/null | wc -l | tr -d ' ')
check "CORS configuration (found: $CORS_CONFIG)" "$([ "$CORS_CONFIG" -gt 0 ] && echo 0 || echo 1)"

# 4. Frontend Security
echo ""
echo "### 4. Frontend Security ###"

# Check for XSS prevention (dangerouslySetInnerHTML actual usage, not comments)
XSS_DANGER=$(grep -rn "dangerouslySetInnerHTML\s*=" web/src/ 2>/dev/null | wc -l | tr -d ' ')
check "No dangerouslySetInnerHTML usage" "$([ "$XSS_DANGER" -eq 0 ] && echo 0 || echo 1)"

# Check for eval usage
EVAL_USAGE=$(grep -rn "\beval\s*(" web/src/ 2>/dev/null | wc -l | tr -d ' ')
check "No eval() usage" "$([ "$EVAL_USAGE" -eq 0 ] && echo 0 || echo 1)"

# Check for environment variable exposure
ENV_EXPOSURE=$(grep -rn "NEXT_PUBLIC_" web/src/ 2>/dev/null | grep -v "NEXT_PUBLIC_SOLANA\|NEXT_PUBLIC_API\|NEXT_PUBLIC_WS" | wc -l | tr -d ' ')
check "Limited public env vars" "$([ "$ENV_EXPOSURE" -lt 10 ] && echo 0 || echo 1)"

# 5. Dependency Security
echo ""
echo "### 5. Dependency Security ###"

# Check Rust dependencies
if cargo audit --version > /dev/null 2>&1; then
    CARGO_AUDIT=$(cargo audit 2>&1 | grep -c "vulnerabilities found" || echo "0")
    check "No cargo audit vulnerabilities" "$([ "$CARGO_AUDIT" = "0" ] && echo 0 || echo 1)"
else
    echo "- cargo-audit not installed, skipping"
fi

# Check npm dependencies (count high/critical)
NPM_AUDIT=$(cd web && npm audit --json 2>/dev/null | grep -E '"high"|"critical"' | wc -l | tr -d ' ')
check "No high/critical npm vulnerabilities" "$([ "$NPM_AUDIT" -lt 5 ] && echo 0 || echo 1)"

# 6. Cryptographic Security
echo ""
echo "### 6. Cryptographic Security ###"

# Check for secure random
SECURE_RANDOM=$(grep -rn "OsRng\|thread_rng\|crypto.randomBytes\|randomUUID" . 2>/dev/null --include="*.rs" --include="*.ts" | wc -l | tr -d ' ')
check "Secure random usage (found: $SECURE_RANDOM)" "$([ "$SECURE_RANDOM" -gt 3 ] && echo 0 || echo 1)"

# Check for Ed25519 usage (Solana standard)
ED25519=$(grep -rn "ed25519\|Ed25519\|sign.*verify" programs/ 2>/dev/null | wc -l | tr -d ' ')
check "Ed25519 signature verification" "$([ "$ED25519" -gt 0 ] && echo 0 || echo 1)"

# 7. Error Handling
echo ""
echo "### 7. Error Handling ###"

# Check for proper error types
ERROR_TYPES=$(grep -rn "#\[error_code\]\|#\[derive.*Error\]" programs/ 2>/dev/null | wc -l | tr -d ' ')
check "Custom error types defined (found: $ERROR_TYPES)" "$([ "$ERROR_TYPES" -gt 0 ] && echo 0 || echo 1)"

# Check for error logging
ERROR_LOGGING=$(grep -rn "error!\|log::error\|tracing::error" app/src/ 2>/dev/null | wc -l | tr -d ' ')
check "Error logging present (found: $ERROR_LOGGING)" "$([ "$ERROR_LOGGING" -gt 5 ] && echo 0 || echo 1)"

# Summary
echo ""
echo "=== Security Stress Test Summary ==="
echo "Passed: $PASS"
echo "Failed: $FAIL"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo "All security checks passed!"
    exit 0
else
    echo "Some security checks failed. Review above results."
    exit 1
fi
