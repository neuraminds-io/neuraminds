// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC8004IdentityRegistry} from "../src/ERC8004IdentityRegistry.sol";
import {ERC8004ReputationRegistry} from "../src/ERC8004ReputationRegistry.sol";

contract ERC8004ReputationRegistryTest is Test {
    address internal admin = makeAddr("admin");
    address internal issuer = makeAddr("issuer");
    address internal attester = makeAddr("attester");
    address internal alice = makeAddr("alice");

    ERC8004IdentityRegistry internal identityRegistry;
    ERC8004ReputationRegistry internal reputationRegistry;

    function setUp() external {
        identityRegistry = new ERC8004IdentityRegistry(admin);
        reputationRegistry = new ERC8004ReputationRegistry(admin, address(identityRegistry));

        vm.startPrank(admin);
        identityRegistry.grantRole(identityRegistry.ISSUER_ROLE(), issuer);
        reputationRegistry.grantRole(reputationRegistry.ATTESTER_ROLE(), attester);
        vm.stopPrank();

        vm.prank(issuer);
        identityRegistry.register(alice, 25);
    }

    function test_submitOutcomesUpdatesScoreAndConfidence() external {
        vm.prank(attester);
        reputationRegistry.submitOutcome(alice, true, 1_000_000, 7000);
        vm.prank(attester);
        reputationRegistry.submitOutcome(alice, false, 2_500_000, 5000);

        (uint32 scoreBps, uint32 confidenceBps, uint64 events, uint128 notional) =
            reputationRegistry.reputationOf(alice);
        assertEq(events, 2);
        assertEq(notional, 3_500_000);
        assertEq(scoreBps, 5000);
        assertGt(confidenceBps, 0);
    }

    function test_submitOutcomeRequiresIdentity() external {
        address noIdentity = makeAddr("noIdentity");
        vm.prank(attester);
        vm.expectRevert(ERC8004ReputationRegistry.IdentityMissing.selector);
        reputationRegistry.submitOutcome(noIdentity, true, 1_000_000, 2000);
    }
}
