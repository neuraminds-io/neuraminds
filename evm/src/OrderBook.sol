// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import {AccessControl} from "openzeppelin-contracts/contracts/access/AccessControl.sol";
import {Pausable} from "openzeppelin-contracts/contracts/utils/Pausable.sol";

contract OrderBook is AccessControl, Pausable {
    bytes32 public constant MATCHER_ROLE = keccak256("MATCHER_ROLE");
    bytes32 public constant PAUSER_ROLE = keccak256("PAUSER_ROLE");

    uint256 public constant MIN_PRICE_BPS = 1;
    uint256 public constant MAX_PRICE_BPS = 9_999;

    struct Order {
        address maker;
        uint256 marketId;
        bool isYes;
        uint128 priceBps;
        uint128 size;
        uint128 remaining;
        uint64 expiry;
        bool canceled;
    }

    uint256 public orderCount;
    mapping(uint256 => Order) public orders;

    error ZeroAddress();
    error InvalidPrice();
    error InvalidSize();
    error InvalidExpiry();
    error OrderNotFound();
    error OrderExpired();
    error OrderAlreadyCanceled();
    error OrderFullyFilled();
    error NotOrderOwner();
    error FillExceedsRemaining();

    event OrderPlaced(
        uint256 indexed orderId,
        address indexed maker,
        uint256 indexed marketId,
        bool isYes,
        uint128 priceBps,
        uint128 size,
        uint64 expiry
    );
    event OrderCanceled(uint256 indexed orderId, address indexed actor);
    event OrderFilled(uint256 indexed orderId, uint128 fillSize, uint128 remaining, address indexed matcher);

    constructor(address admin) {
        if (admin == address(0)) revert ZeroAddress();

        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(MATCHER_ROLE, admin);
        _grantRole(PAUSER_ROLE, admin);
    }

    function placeOrder(uint256 marketId, bool isYes, uint128 priceBps, uint128 size, uint64 expiry)
        external
        whenNotPaused
        returns (uint256 orderId)
    {
        if (priceBps < MIN_PRICE_BPS || priceBps > MAX_PRICE_BPS) revert InvalidPrice();
        if (size == 0) revert InvalidSize();
        if (expiry <= block.timestamp) revert InvalidExpiry();

        orderId = ++orderCount;
        orders[orderId] = Order({
            maker: msg.sender,
            marketId: marketId,
            isYes: isYes,
            priceBps: priceBps,
            size: size,
            remaining: size,
            expiry: expiry,
            canceled: false
        });

        emit OrderPlaced(orderId, msg.sender, marketId, isYes, priceBps, size, expiry);
    }

    function cancelOrder(uint256 orderId) external whenNotPaused {
        Order storage order = orders[orderId];
        if (order.maker == address(0)) revert OrderNotFound();
        if (order.canceled) revert OrderAlreadyCanceled();
        if (order.remaining == 0) revert OrderFullyFilled();
        if (msg.sender != order.maker && !hasRole(DEFAULT_ADMIN_ROLE, msg.sender)) {
            revert NotOrderOwner();
        }

        order.canceled = true;
        emit OrderCanceled(orderId, msg.sender);
    }

    function fillOrder(uint256 orderId, uint128 fillSize) external onlyRole(MATCHER_ROLE) whenNotPaused {
        Order storage order = orders[orderId];
        if (order.maker == address(0)) revert OrderNotFound();
        if (order.canceled) revert OrderAlreadyCanceled();
        if (order.remaining == 0) revert OrderFullyFilled();
        if (order.expiry < block.timestamp) revert OrderExpired();
        if (fillSize == 0 || fillSize > order.remaining) revert FillExceedsRemaining();

        order.remaining -= fillSize;
        emit OrderFilled(orderId, fillSize, order.remaining, msg.sender);
    }

    function pause() external onlyRole(PAUSER_ROLE) {
        _pause();
    }

    function unpause() external onlyRole(PAUSER_ROLE) {
        _unpause();
    }
}
