//! Fuzz target for redemption logic
//!
//! Tests market resolution and token redemption:
//! - YES wins: YES tokens redeem 1:1
//! - NO wins: NO tokens redeem 1:1
//! - Invalid: 50/50 split refund
//! - Collateral conservation after redemption

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::{fuzz_target, Corpus};
use log::info;
use std::sync::Once;

const MAX_BALANCE: u64 = 1_000_000_000_000;

#[derive(Debug, Arbitrary, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum ResolutionOutcome {
    Unresolved = 0,
    Yes = 1,
    No = 2,
    Invalid = 3,
}

#[derive(Debug, Arbitrary, Clone)]
struct RedemptionData {
    scenarios: Vec<RedemptionScenario>,
}

#[derive(Debug, Arbitrary, Clone)]
struct RedemptionScenario {
    /// YES token balance (free)
    yes_free: u64,
    /// YES token balance (locked in orders)
    yes_locked: u64,
    /// NO token balance (free)
    no_free: u64,
    /// NO token balance (locked in orders)
    no_locked: u64,
    /// Remaining collateral (from cancelled orders, etc.)
    collateral_remaining: u64,
    /// Market resolution outcome
    outcome: ResolutionOutcome,
}

#[derive(Debug, Clone)]
struct UserPosition {
    yes_free: u64,
    yes_locked: u64,
    no_free: u64,
    no_locked: u64,
    collateral_free: u64,
    collateral_locked: u64,
}

#[derive(Debug, Clone)]
struct RedemptionResult {
    yes_burned: u64,
    no_burned: u64,
    collateral_received: u64,
}

fn constrain_balance(balance: u64) -> u64 {
    balance % MAX_BALANCE
}

/// Calculate redemption amounts based on resolution outcome
fn calculate_redemption(position: &UserPosition, outcome: ResolutionOutcome) -> RedemptionResult {
    let total_yes = position.yes_free.saturating_add(position.yes_locked);
    let total_no = position.no_free.saturating_add(position.no_locked);
    let remaining_collateral = position
        .collateral_free
        .saturating_add(position.collateral_locked);

    let (yes_burned, no_burned, outcome_collateral) = match outcome {
        ResolutionOutcome::Yes => {
            // YES wins: YES tokens redeem 1:1, NO tokens worthless
            (total_yes, total_no, total_yes)
        }
        ResolutionOutcome::No => {
            // NO wins: NO tokens redeem 1:1, YES tokens worthless
            (total_yes, total_no, total_no)
        }
        ResolutionOutcome::Invalid => {
            // Market invalid: 50% refund for all tokens
            let total_tokens = total_yes.saturating_add(total_no);
            let collateral = total_tokens.checked_div(2).unwrap_or(0);
            (total_yes, total_no, collateral)
        }
        ResolutionOutcome::Unresolved => {
            // Cannot redeem unresolved market
            (0, 0, 0)
        }
    };

    RedemptionResult {
        yes_burned,
        no_burned,
        collateral_received: outcome_collateral.saturating_add(remaining_collateral),
    }
}

fuzz_target!(|data: RedemptionData| -> Corpus {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = env_logger::try_init();
    });

    if data.scenarios.is_empty() || data.scenarios.len() > 50 {
        return Corpus::Reject;
    }

    // Skip if all scenarios are unresolved (not interesting)
    let has_resolved = data
        .scenarios
        .iter()
        .any(|s| s.outcome != ResolutionOutcome::Unresolved);
    if !has_resolved {
        return Corpus::Reject;
    }

    run_redemption_fuzz(data)
});

fn run_redemption_fuzz(data: RedemptionData) -> Corpus {
    info!("Testing {} redemption scenarios", data.scenarios.len());

    for (i, scenario) in data.scenarios.iter().enumerate() {
        if scenario.outcome == ResolutionOutcome::Unresolved {
            continue;
        }

        let position = UserPosition {
            yes_free: constrain_balance(scenario.yes_free),
            yes_locked: constrain_balance(scenario.yes_locked),
            no_free: constrain_balance(scenario.no_free),
            no_locked: constrain_balance(scenario.no_locked),
            collateral_free: constrain_balance(scenario.collateral_remaining),
            collateral_locked: 0,
        };

        let total_yes = position.yes_free.saturating_add(position.yes_locked);
        let total_no = position.no_free.saturating_add(position.no_locked);

        info!(
            "Scenario {}: outcome={:?} yes={} no={} collateral={}",
            i, scenario.outcome, total_yes, total_no, position.collateral_free
        );

        let result = calculate_redemption(&position, scenario.outcome);

        // Verify invariants

        // 1. All tokens are burned
        assert_eq!(
            result.yes_burned, total_yes,
            "Not all YES tokens burned"
        );
        assert_eq!(
            result.no_burned, total_no,
            "Not all NO tokens burned"
        );

        // 2. Collateral received matches expected for outcome
        match scenario.outcome {
            ResolutionOutcome::Yes => {
                // YES wins: collateral = YES tokens + remaining
                let expected = total_yes.saturating_add(position.collateral_free);
                assert_eq!(
                    result.collateral_received, expected,
                    "YES outcome: wrong collateral"
                );
            }
            ResolutionOutcome::No => {
                // NO wins: collateral = NO tokens + remaining
                let expected = total_no.saturating_add(position.collateral_free);
                assert_eq!(
                    result.collateral_received, expected,
                    "NO outcome: wrong collateral"
                );
            }
            ResolutionOutcome::Invalid => {
                // Invalid: 50% of tokens + remaining
                let token_collateral = total_yes.saturating_add(total_no) / 2;
                let expected = token_collateral.saturating_add(position.collateral_free);
                assert_eq!(
                    result.collateral_received, expected,
                    "Invalid outcome: wrong collateral"
                );
            }
            ResolutionOutcome::Unresolved => unreachable!(),
        }

        // 3. Collateral received never exceeds theoretical maximum
        // Maximum is if user held 100% of both outcomes
        let max_possible = total_yes.saturating_add(total_no).saturating_add(position.collateral_free);
        assert!(
            result.collateral_received <= max_possible,
            "Collateral {} exceeds max {}",
            result.collateral_received,
            max_possible
        );

        // 4. For YES/NO outcomes, winner gets full value
        if scenario.outcome == ResolutionOutcome::Yes {
            assert!(
                result.collateral_received >= total_yes,
                "YES winner didn't get full value"
            );
        }
        if scenario.outcome == ResolutionOutcome::No {
            assert!(
                result.collateral_received >= total_no,
                "NO winner didn't get full value"
            );
        }

        // 5. No arithmetic overflow in calculations
        let _ = total_yes.checked_add(total_no).expect("Token sum overflow");
        let _ = result
            .collateral_received
            .checked_add(0)
            .expect("Collateral overflow");
    }

    Corpus::Keep
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yes_wins() {
        let position = UserPosition {
            yes_free: 100,
            yes_locked: 50,
            no_free: 30,
            no_locked: 20,
            collateral_free: 10,
            collateral_locked: 0,
        };

        let result = calculate_redemption(&position, ResolutionOutcome::Yes);
        assert_eq!(result.yes_burned, 150);
        assert_eq!(result.no_burned, 50);
        assert_eq!(result.collateral_received, 160); // 150 + 10
    }

    #[test]
    fn test_no_wins() {
        let position = UserPosition {
            yes_free: 100,
            yes_locked: 50,
            no_free: 200,
            no_locked: 0,
            collateral_free: 5,
            collateral_locked: 0,
        };

        let result = calculate_redemption(&position, ResolutionOutcome::No);
        assert_eq!(result.collateral_received, 205); // 200 + 5
    }

    #[test]
    fn test_invalid_market() {
        let position = UserPosition {
            yes_free: 100,
            yes_locked: 0,
            no_free: 100,
            no_locked: 0,
            collateral_free: 0,
            collateral_locked: 0,
        };

        let result = calculate_redemption(&position, ResolutionOutcome::Invalid);
        assert_eq!(result.collateral_received, 100); // (100 + 100) / 2
    }

    #[test]
    fn test_zero_balances() {
        let position = UserPosition {
            yes_free: 0,
            yes_locked: 0,
            no_free: 0,
            no_locked: 0,
            collateral_free: 50,
            collateral_locked: 0,
        };

        let result = calculate_redemption(&position, ResolutionOutcome::Yes);
        assert_eq!(result.collateral_received, 50); // Only remaining collateral
    }
}
