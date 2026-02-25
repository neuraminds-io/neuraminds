// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import {Script, console2} from "forge-std/Script.sol";
import {NeuraToken} from "../src/NeuraToken.sol";
import {MarketCore} from "../src/MarketCore.sol";
import {OrderBook} from "../src/OrderBook.sol";
import {CollateralVault} from "../src/CollateralVault.sol";

contract DeployCoreScript is Script {
    function run() external {
        address admin = vm.envAddress("BASE_ADMIN");
        address treasury = vm.envAddress("BASE_TREASURY");
        uint256 cap = vm.envUint("NEURA_CAP_WEI");
        uint256 initialSupply = vm.envUint("NEURA_INITIAL_SUPPLY_WEI");

        vm.startBroadcast();

        NeuraToken token = new NeuraToken("Neura", "NEURA", cap, admin, treasury, initialSupply);
        MarketCore marketCore = new MarketCore(admin);
        OrderBook orderBook = new OrderBook(admin);

        address collateralToken = vm.envOr("COLLATERAL_TOKEN_ADDRESS", address(0));
        if (collateralToken == address(0)) {
            collateralToken = address(token);
        }
        CollateralVault collateralVault = new CollateralVault(admin, collateralToken);

        vm.stopBroadcast();

        console2.log("NeuraToken:", address(token));
        console2.log("MarketCore:", address(marketCore));
        console2.log("OrderBook:", address(orderBook));
        console2.log("CollateralVault:", address(collateralVault));
    }
}
