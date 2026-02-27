// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import {AccessControl} from "openzeppelin-contracts/contracts/access/AccessControl.sol";
import {Pausable} from "openzeppelin-contracts/contracts/utils/Pausable.sol";

interface IERC8004IdentityRead {
    function identityOf(address wallet) external view returns (uint256);
}

contract ERC8004ReputationRegistry is AccessControl, Pausable {
    bytes32 public constant PAUSER_ROLE = keccak256("PAUSER_ROLE");
    bytes32 public constant ATTESTER_ROLE = keccak256("ATTESTER_ROLE");

    struct Reputation {
        uint64 eventCount;
        uint64 successCount;
        uint128 notionalMicrousdc;
        uint32 scoreBps;
        uint32 confidenceBps;
        uint64 updatedAt;
    }

    IERC8004IdentityRead public immutable identityRegistry;
    mapping(address => Reputation) private _reputation;

    error ZeroAddress();
    error IdentityMissing();
    error InvalidConfidenceWeight();

    event OutcomeSubmitted(address indexed wallet, bool success, uint128 notionalMicrousdc, uint16 confidenceWeightBps);
    event ReputationUpdated(
        address indexed wallet, uint32 scoreBps, uint32 confidenceBps, uint64 eventCount, uint128 notionalMicrousdc
    );

    constructor(address admin, address identityRegistryAddress) {
        if (admin == address(0) || identityRegistryAddress == address(0)) revert ZeroAddress();
        identityRegistry = IERC8004IdentityRead(identityRegistryAddress);

        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(PAUSER_ROLE, admin);
        _grantRole(ATTESTER_ROLE, admin);
    }

    function submitOutcome(address wallet, bool success, uint128 notionalMicrousdc, uint16 confidenceWeightBps)
        external
        onlyRole(ATTESTER_ROLE)
        whenNotPaused
    {
        if (wallet == address(0)) revert ZeroAddress();
        if (identityRegistry.identityOf(wallet) == 0) revert IdentityMissing();
        if (confidenceWeightBps > 10_000) revert InvalidConfidenceWeight();

        Reputation storage row = _reputation[wallet];
        row.eventCount += 1;
        if (success) {
            row.successCount += 1;
        }
        row.notionalMicrousdc += notionalMicrousdc;
        row.scoreBps = uint32((uint256(row.successCount) * 10_000) / uint256(row.eventCount));

        uint256 confidence = uint256(row.eventCount) * 250 + uint256(confidenceWeightBps);
        if (confidence > 10_000) {
            confidence = 10_000;
        }
        row.confidenceBps = uint32(confidence);
        row.updatedAt = uint64(block.timestamp);

        emit OutcomeSubmitted(wallet, success, notionalMicrousdc, confidenceWeightBps);
        emit ReputationUpdated(wallet, row.scoreBps, row.confidenceBps, row.eventCount, row.notionalMicrousdc);
    }

    function reputationOf(address wallet)
        external
        view
        returns (uint32 scoreBps, uint32 confidenceBps, uint64 eventCount, uint128 notionalMicrousdc)
    {
        Reputation storage row = _reputation[wallet];
        return (row.scoreBps, row.confidenceBps, row.eventCount, row.notionalMicrousdc);
    }

    function pause() external onlyRole(PAUSER_ROLE) {
        _pause();
    }

    function unpause() external onlyRole(PAUSER_ROLE) {
        _unpause();
    }
}
