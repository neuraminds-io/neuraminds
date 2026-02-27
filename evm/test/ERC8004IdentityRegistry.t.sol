// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import {Test} from "forge-std/Test.sol";
import {ERC8004IdentityRegistry} from "../src/ERC8004IdentityRegistry.sol";

contract ERC8004IdentityRegistryTest is Test {
    address internal admin = makeAddr("admin");
    address internal issuer = makeAddr("issuer");
    address internal alice = makeAddr("alice");

    ERC8004IdentityRegistry internal registry;

    function setUp() external {
        registry = new ERC8004IdentityRegistry(admin);

        vm.startPrank(admin);
        registry.grantRole(registry.ISSUER_ROLE(), issuer);
        vm.stopPrank();
    }

    function test_registerAndReadProfile() external {
        vm.prank(issuer);
        uint256 identityId = registry.register(alice, 42);

        (uint256 storedId, uint8 tier, bool active, uint64 updatedAt) = registry.profile(alice);
        assertEq(identityId, 1);
        assertEq(storedId, identityId);
        assertEq(tier, 42);
        assertTrue(active);
        assertGt(updatedAt, 0);
    }

    function test_setTierAndDeactivate() external {
        vm.prank(issuer);
        registry.register(alice, 15);

        vm.prank(issuer);
        registry.setTier(alice, 77);
        vm.prank(issuer);
        registry.setActive(alice, false);

        (, uint8 tier, bool active,) = registry.profile(alice);
        assertEq(tier, 77);
        assertFalse(active);
    }
}
