// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import {Test} from "forge-std/Test.sol";
import {OrderBook} from "../src/OrderBook.sol";

contract OrderBookTest is Test {
    address internal admin = makeAddr("admin");
    address internal matcher = makeAddr("matcher");
    address internal maker = makeAddr("maker");
    address internal outsider = makeAddr("outsider");

    OrderBook internal orderBook;

    function setUp() external {
        orderBook = new OrderBook(admin);

        bytes32 matcherRole = orderBook.MATCHER_ROLE();
        vm.prank(admin);
        orderBook.grantRole(matcherRole, matcher);
    }

    function test_placeOrder() external {
        vm.prank(maker);
        uint256 orderId = orderBook.placeOrder(1, true, 5_000, 100e6, uint64(block.timestamp + 1 days));

        (
            address orderMaker,
            uint256 marketId,
            bool isYes,
            uint128 priceBps,
            uint128 size,
            uint128 remaining,,
            bool canceled
        ) = orderBook.orders(orderId);

        assertEq(orderId, 1);
        assertEq(orderMaker, maker);
        assertEq(marketId, 1);
        assertEq(isYes, true);
        assertEq(priceBps, 5_000);
        assertEq(size, 100e6);
        assertEq(remaining, 100e6);
        assertEq(canceled, false);
    }

    function test_cancelOrder() external {
        vm.prank(maker);
        uint256 orderId = orderBook.placeOrder(7, false, 4_900, 50e6, uint64(block.timestamp + 1 days));

        vm.prank(maker);
        orderBook.cancelOrder(orderId);

        (,,,,,,, bool canceled) = orderBook.orders(orderId);
        assertTrue(canceled);
    }

    function test_fillOrder() external {
        vm.prank(maker);
        uint256 orderId = orderBook.placeOrder(3, true, 4_200, 75e6, uint64(block.timestamp + 1 days));

        vm.prank(matcher);
        orderBook.fillOrder(orderId, 25e6);

        (,,,,, uint128 remaining,,) = orderBook.orders(orderId);
        assertEq(remaining, 50e6);
    }

    function test_fillFailsForUnauthorizedMatcher() external {
        vm.prank(maker);
        uint256 orderId = orderBook.placeOrder(3, true, 4_200, 75e6, uint64(block.timestamp + 1 days));

        vm.prank(outsider);
        vm.expectRevert();
        orderBook.fillOrder(orderId, 10e6);
    }

    function test_fillFailsWhenAmountExceedsRemaining() external {
        vm.prank(maker);
        uint256 orderId = orderBook.placeOrder(3, true, 4_200, 75e6, uint64(block.timestamp + 1 days));

        vm.prank(matcher);
        vm.expectRevert(OrderBook.FillExceedsRemaining.selector);
        orderBook.fillOrder(orderId, 100e6);
    }

    function test_orderExpires() external {
        vm.prank(maker);
        uint256 orderId = orderBook.placeOrder(5, false, 5_500, 10e6, uint64(block.timestamp + 1 hours));

        vm.warp(block.timestamp + 1 hours + 1);

        vm.prank(matcher);
        vm.expectRevert(OrderBook.OrderExpired.selector);
        orderBook.fillOrder(orderId, 1e6);
    }
}
