// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import {AccessControl} from "openzeppelin-contracts/contracts/access/AccessControl.sol";
import {Pausable} from "openzeppelin-contracts/contracts/utils/Pausable.sol";

contract MarketCore is AccessControl, Pausable {
    bytes32 public constant MARKET_CREATOR_ROLE = keccak256("MARKET_CREATOR_ROLE");
    bytes32 public constant RESOLVER_ROLE = keccak256("RESOLVER_ROLE");
    bytes32 public constant PAUSER_ROLE = keccak256("PAUSER_ROLE");

    struct Market {
        bytes32 questionHash;
        uint64 closeTime;
        uint64 resolveTime;
        address resolver;
        bool resolved;
        bool outcome;
    }

    uint256 public marketCount;
    mapping(uint256 => Market) public markets;

    error ZeroAddress();
    error InvalidCloseTime();
    error MarketNotFound();
    error MarketNotClosed();
    error MarketAlreadyResolved();
    error NotDesignatedResolver();

    event MarketCreated(uint256 indexed marketId, bytes32 indexed questionHash, uint64 closeTime, address resolver);
    event MarketResolved(uint256 indexed marketId, bool outcome, uint64 resolveTime, address resolver);

    constructor(address admin) {
        if (admin == address(0)) revert ZeroAddress();

        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(MARKET_CREATOR_ROLE, admin);
        _grantRole(RESOLVER_ROLE, admin);
        _grantRole(PAUSER_ROLE, admin);
    }

    function createMarket(bytes32 questionHash, uint64 closeTime, address resolver)
        external
        onlyRole(MARKET_CREATOR_ROLE)
        whenNotPaused
        returns (uint256 marketId)
    {
        if (resolver == address(0)) revert ZeroAddress();
        if (closeTime <= block.timestamp) revert InvalidCloseTime();

        marketId = ++marketCount;
        markets[marketId] = Market({
            questionHash: questionHash,
            closeTime: closeTime,
            resolveTime: 0,
            resolver: resolver,
            resolved: false,
            outcome: false
        });

        emit MarketCreated(marketId, questionHash, closeTime, resolver);
    }

    function resolveMarket(uint256 marketId, bool outcome) external onlyRole(RESOLVER_ROLE) whenNotPaused {
        Market storage market = markets[marketId];
        if (market.resolver == address(0)) revert MarketNotFound();
        if (block.timestamp < market.closeTime) revert MarketNotClosed();
        if (market.resolved) revert MarketAlreadyResolved();
        if (msg.sender != market.resolver && !hasRole(DEFAULT_ADMIN_ROLE, msg.sender)) {
            revert NotDesignatedResolver();
        }

        market.resolved = true;
        market.outcome = outcome;
        market.resolveTime = uint64(block.timestamp);

        emit MarketResolved(marketId, outcome, market.resolveTime, msg.sender);
    }

    function pause() external onlyRole(PAUSER_ROLE) {
        _pause();
    }

    function unpause() external onlyRole(PAUSER_ROLE) {
        _unpause();
    }
}
