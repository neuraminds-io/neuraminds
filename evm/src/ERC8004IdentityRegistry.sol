// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import {AccessControl} from "openzeppelin-contracts/contracts/access/AccessControl.sol";
import {Pausable} from "openzeppelin-contracts/contracts/utils/Pausable.sol";

contract ERC8004IdentityRegistry is AccessControl, Pausable {
    bytes32 public constant PAUSER_ROLE = keccak256("PAUSER_ROLE");
    bytes32 public constant ISSUER_ROLE = keccak256("ISSUER_ROLE");

    struct IdentityProfile {
        uint256 identityId;
        uint8 tier;
        bool active;
        uint64 createdAt;
        uint64 updatedAt;
    }

    uint8 public constant MAX_TIER = 100;
    uint256 public identityCount;
    mapping(address => IdentityProfile) private _profiles;
    mapping(uint256 => address) private _ownersByIdentity;

    error ZeroAddress();
    error InvalidTier();
    error AlreadyRegistered();
    error NotRegistered();

    event IdentityRegistered(address indexed wallet, uint256 indexed identityId, uint8 tier);
    event TierUpdated(address indexed wallet, uint8 tier);
    event IdentityActivated(address indexed wallet, bool active);

    constructor(address admin) {
        if (admin == address(0)) revert ZeroAddress();
        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(PAUSER_ROLE, admin);
        _grantRole(ISSUER_ROLE, admin);
    }

    function register(address wallet, uint8 tier) external onlyRole(ISSUER_ROLE) whenNotPaused returns (uint256) {
        if (wallet == address(0)) revert ZeroAddress();
        if (tier > MAX_TIER) revert InvalidTier();
        if (_profiles[wallet].identityId != 0) revert AlreadyRegistered();

        uint256 identityId = ++identityCount;
        uint64 nowTs = uint64(block.timestamp);
        _profiles[wallet] =
            IdentityProfile({identityId: identityId, tier: tier, active: true, createdAt: nowTs, updatedAt: nowTs});
        _ownersByIdentity[identityId] = wallet;

        emit IdentityRegistered(wallet, identityId, tier);
        return identityId;
    }

    function setTier(address wallet, uint8 tier) external onlyRole(ISSUER_ROLE) whenNotPaused {
        if (tier > MAX_TIER) revert InvalidTier();
        IdentityProfile storage entry = _profiles[wallet];
        if (entry.identityId == 0) revert NotRegistered();
        entry.tier = tier;
        entry.updatedAt = uint64(block.timestamp);
        emit TierUpdated(wallet, tier);
    }

    function setActive(address wallet, bool active) external onlyRole(ISSUER_ROLE) whenNotPaused {
        IdentityProfile storage entry = _profiles[wallet];
        if (entry.identityId == 0) revert NotRegistered();
        entry.active = active;
        entry.updatedAt = uint64(block.timestamp);
        emit IdentityActivated(wallet, active);
    }

    function profile(address wallet)
        external
        view
        returns (uint256 identityId, uint8 tier, bool active, uint64 updatedAt)
    {
        IdentityProfile storage entry = _profiles[wallet];
        return (entry.identityId, entry.tier, entry.active, entry.updatedAt);
    }

    function identityOf(address wallet) external view returns (uint256) {
        return _profiles[wallet].identityId;
    }

    function ownerOfIdentity(uint256 identityId) external view returns (address) {
        return _ownersByIdentity[identityId];
    }

    function pause() external onlyRole(PAUSER_ROLE) {
        _pause();
    }

    function unpause() external onlyRole(PAUSER_ROLE) {
        _unpause();
    }
}
