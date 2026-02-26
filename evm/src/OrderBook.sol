// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import {AccessControl} from "openzeppelin-contracts/contracts/access/AccessControl.sol";
import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "openzeppelin-contracts/contracts/token/ERC20/utils/SafeERC20.sol";
import {Pausable} from "openzeppelin-contracts/contracts/utils/Pausable.sol";
import {ReentrancyGuard} from "openzeppelin-contracts/contracts/utils/ReentrancyGuard.sol";

interface IMarketCoreRead {
    function markets(uint256 marketId)
        external
        view
        returns (bytes32 questionHash, uint64 closeTime, uint64 resolveTime, address resolver, bool resolved, bool outcome);
}

contract OrderBook is AccessControl, Pausable, ReentrancyGuard {
    using SafeERC20 for IERC20;

    bytes32 public constant MATCHER_ROLE = keccak256("MATCHER_ROLE");
    bytes32 public constant PAUSER_ROLE = keccak256("PAUSER_ROLE");

    uint256 public constant MIN_PRICE_BPS = 1;
    uint256 public constant MAX_PRICE_BPS = 9_999;
    uint256 public constant PAR_PRICE_BPS = 10_000;

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

    struct Position {
        uint128 yesShares;
        uint128 noShares;
        bool claimed;
    }

    struct MarketPool {
        uint256 escrow;
        uint256 paidOut;
        uint256 matchedShares;
    }

    uint256 public orderCount;
    mapping(uint256 => Order) public orders;
    mapping(uint256 => mapping(address => Position)) public positions;
    mapping(uint256 => MarketPool) public marketPools;

    IERC20 public immutable collateral;
    IMarketCoreRead public immutable marketCore;

    error ZeroAddress();
    error InvalidPrice();
    error PriceCrossFailed();
    error InvalidSize();
    error InvalidExpiry();
    error OrderNotFound();
    error OrderExpired();
    error OrderAlreadyCanceled();
    error OrderFullyFilled();
    error NotOrderOwner();
    error FillExceedsRemaining();
    error InvalidMatchPair();
    error MarketNotResolved();
    error AlreadyClaimed();
    error NoPosition();
    error NoWinningShares();
    error InsufficientEscrow();

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
    event OrdersMatched(
        uint256 indexed marketId,
        uint256 indexed yesOrderId,
        uint256 indexed noOrderId,
        uint128 fillSize,
        uint128 yesPriceBps,
        uint128 noPriceBps
    );
    event Claimed(uint256 indexed marketId, address indexed user, bool outcome, uint256 payout, uint256 shares);

    constructor(address admin, address marketCoreAddress, address collateralToken) {
        if (admin == address(0) || marketCoreAddress == address(0) || collateralToken == address(0)) {
            revert ZeroAddress();
        }

        marketCore = IMarketCoreRead(marketCoreAddress);
        collateral = IERC20(collateralToken);
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

    function matchOrders(uint256 firstOrderId, uint256 secondOrderId, uint128 fillSize)
        external
        onlyRole(MATCHER_ROLE)
        whenNotPaused
        nonReentrant
    {
        if (firstOrderId == secondOrderId) revert InvalidMatchPair();

        Order storage first = orders[firstOrderId];
        Order storage second = orders[secondOrderId];
        if (first.maker == address(0) || second.maker == address(0)) revert OrderNotFound();
        if (first.marketId != second.marketId || first.isYes == second.isYes) revert InvalidMatchPair();

        _assertOrderFillable(first, fillSize);
        _assertOrderFillable(second, fillSize);

        Order storage yesOrder = first.isYes ? first : second;
        Order storage noOrder = first.isYes ? second : first;
        if (uint256(yesOrder.priceBps) + uint256(noOrder.priceBps) < PAR_PRICE_BPS) {
            revert PriceCrossFailed();
        }

        yesOrder.remaining -= fillSize;
        noOrder.remaining -= fillSize;

        collateral.safeTransferFrom(yesOrder.maker, address(this), fillSize);
        collateral.safeTransferFrom(noOrder.maker, address(this), fillSize);

        positions[yesOrder.marketId][yesOrder.maker].yesShares += fillSize;
        positions[yesOrder.marketId][noOrder.maker].noShares += fillSize;

        MarketPool storage pool = marketPools[yesOrder.marketId];
        pool.escrow += uint256(fillSize) * 2;
        pool.matchedShares += fillSize;

        emit OrderFilled(firstOrderId, fillSize, first.remaining, msg.sender);
        emit OrderFilled(secondOrderId, fillSize, second.remaining, msg.sender);
        emit OrdersMatched(
            yesOrder.marketId, first.isYes ? firstOrderId : secondOrderId, first.isYes ? secondOrderId : firstOrderId, fillSize, yesOrder.priceBps, noOrder.priceBps
        );
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
        _assertOrderFillable(order, fillSize);

        order.remaining -= fillSize;
        emit OrderFilled(orderId, fillSize, order.remaining, msg.sender);
    }

    function claim(uint256 marketId) external whenNotPaused nonReentrant returns (uint256 payout) {
        Position storage position = positions[marketId][msg.sender];
        if (position.claimed) revert AlreadyClaimed();
        if (position.yesShares == 0 && position.noShares == 0) revert NoPosition();

        (, , , , bool resolved, bool outcome) = marketCore.markets(marketId);
        if (!resolved) revert MarketNotResolved();

        uint256 winningShares = outcome ? position.yesShares : position.noShares;
        if (winningShares == 0) revert NoWinningShares();

        payout = winningShares * 2;

        MarketPool storage pool = marketPools[marketId];
        uint256 remainingEscrow = pool.escrow - pool.paidOut;
        if (remainingEscrow < payout) revert InsufficientEscrow();
        pool.paidOut += payout;

        position.yesShares = 0;
        position.noShares = 0;
        position.claimed = true;

        collateral.safeTransfer(msg.sender, payout);
        emit Claimed(marketId, msg.sender, outcome, payout, winningShares);
    }

    function claimable(uint256 marketId, address user) external view returns (uint256) {
        if (user == address(0)) revert ZeroAddress();

        Position memory position = positions[marketId][user];
        if (position.claimed) return 0;
        if (position.yesShares == 0 && position.noShares == 0) return 0;

        (, , , , bool resolved, bool outcome) = marketCore.markets(marketId);
        if (!resolved) return 0;

        uint256 winningShares = outcome ? position.yesShares : position.noShares;
        return winningShares * 2;
    }

    function _assertOrderFillable(Order storage order, uint128 fillSize) internal view {
        if (order.maker == address(0)) revert OrderNotFound();
        if (order.canceled) revert OrderAlreadyCanceled();
        if (order.remaining == 0) revert OrderFullyFilled();
        if (order.expiry < block.timestamp) revert OrderExpired();
        if (fillSize == 0 || fillSize > order.remaining) revert FillExceedsRemaining();
    }

    function pause() external onlyRole(PAUSER_ROLE) {
        _pause();
    }

    function unpause() external onlyRole(PAUSER_ROLE) {
        _unpause();
    }
}
