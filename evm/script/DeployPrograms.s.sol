// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import {Script, console2} from "forge-std/Script.sol";
import {MarketCore} from "../src/MarketCore.sol";
import {OrderBook} from "../src/OrderBook.sol";
import {CollateralVault} from "../src/CollateralVault.sol";
import {AgentRuntime} from "../src/AgentRuntime.sol";

interface IAccessControlLike {
    function grantRole(bytes32 role, address account) external;
    function revokeRole(bytes32 role, address account) external;
    function hasRole(bytes32 role, address account) external view returns (bool);
}

contract DeployProgramsScript is Script {
    error MissingAdmin();
    error MissingCollateralToken();
    error ZeroAddress(string field);

    function run() external {
        address admin = _envAddressOr("BASE_ADMIN", address(0));
        if (admin == address(0)) revert MissingAdmin();

        address bootstrapAdmin = _envAddressOr("BOOTSTRAP_ADMIN", admin);
        if (bootstrapAdmin == address(0)) revert ZeroAddress("BOOTSTRAP_ADMIN");

        address pauser = _envAddressOr("BASE_PAUSER", admin);
        address resolver = _envAddressOr("BASE_RESOLVER", admin);
        address marketCreator = _envAddressOr("BASE_MARKET_CREATOR", admin);
        address operator = _envAddressOr("BASE_OPERATOR", admin);
        address runtimeOperator = _envAddressOr("BASE_AGENT_RUNTIME_OPERATOR", address(0));

        address collateralToken = _resolveCollateralToken();
        if (collateralToken == address(0)) revert MissingCollateralToken();

        vm.startBroadcast();

        MarketCore marketCore = new MarketCore(bootstrapAdmin);
        CollateralVault collateralVault = new CollateralVault(bootstrapAdmin, collateralToken);
        OrderBook orderBook = new OrderBook(bootstrapAdmin, address(marketCore), address(collateralVault));
        AgentRuntime agentRuntime = new AgentRuntime(bootstrapAdmin, address(orderBook));

        _configureMarketCore(marketCore, bootstrapAdmin, admin, marketCreator, resolver, pauser);
        _configureCollateralVault(collateralVault, bootstrapAdmin, admin, operator, pauser, address(orderBook));
        _configureOrderBook(orderBook, bootstrapAdmin, admin, pauser, address(agentRuntime), runtimeOperator);
        _configureAgentRuntime(agentRuntime, bootstrapAdmin, admin, pauser);

        vm.stopBroadcast();

        console2.log("chainId:", block.chainid);
        console2.log("admin:", admin);
        console2.log("bootstrapAdmin:", bootstrapAdmin);
        console2.log("MarketCore:", address(marketCore));
        console2.log("CollateralVault:", address(collateralVault));
        console2.log("OrderBook:", address(orderBook));
        console2.log("AgentRuntime:", address(agentRuntime));
        console2.log("collateralToken:", collateralToken);
        console2.log("marketCreator:", marketCreator);
        console2.log("resolver:", resolver);
        console2.log("operator:", operator);
        console2.log("pauser:", pauser);
        console2.log("runtimeOperator:", runtimeOperator);
    }

    function _configureMarketCore(
        MarketCore marketCore,
        address bootstrapAdmin,
        address admin,
        address marketCreator,
        address resolver,
        address pauser
    ) internal {
        bytes32 defaultAdminRole = marketCore.DEFAULT_ADMIN_ROLE();

        _grantRoleIfMissing(IAccessControlLike(address(marketCore)), marketCore.MARKET_CREATOR_ROLE(), marketCreator);
        _grantRoleIfMissing(IAccessControlLike(address(marketCore)), marketCore.RESOLVER_ROLE(), resolver);
        _grantRoleIfMissing(IAccessControlLike(address(marketCore)), marketCore.PAUSER_ROLE(), pauser);

        if (bootstrapAdmin != admin) {
            _grantRoleIfMissing(IAccessControlLike(address(marketCore)), defaultAdminRole, admin);
            _revokeRoleIfPresent(IAccessControlLike(address(marketCore)), marketCore.MARKET_CREATOR_ROLE(), bootstrapAdmin);
            _revokeRoleIfPresent(IAccessControlLike(address(marketCore)), marketCore.RESOLVER_ROLE(), bootstrapAdmin);
            _revokeRoleIfPresent(IAccessControlLike(address(marketCore)), marketCore.PAUSER_ROLE(), bootstrapAdmin);
            _revokeRoleIfPresent(IAccessControlLike(address(marketCore)), defaultAdminRole, bootstrapAdmin);
        }
    }

    function _configureOrderBook(
        OrderBook orderBook,
        address bootstrapAdmin,
        address admin,
        address pauser,
        address agentRuntime,
        address runtimeOperator
    ) internal {
        bytes32 defaultAdminRole = orderBook.DEFAULT_ADMIN_ROLE();

        _grantRoleIfMissing(IAccessControlLike(address(orderBook)), orderBook.PAUSER_ROLE(), pauser);
        _grantRoleIfMissing(IAccessControlLike(address(orderBook)), orderBook.AGENT_RUNTIME_ROLE(), agentRuntime);
        if (runtimeOperator != address(0)) {
            _grantRoleIfMissing(IAccessControlLike(address(orderBook)), orderBook.AGENT_RUNTIME_ROLE(), runtimeOperator);
        }

        if (bootstrapAdmin != admin) {
            _grantRoleIfMissing(IAccessControlLike(address(orderBook)), defaultAdminRole, admin);
            _revokeRoleIfPresent(IAccessControlLike(address(orderBook)), orderBook.PAUSER_ROLE(), bootstrapAdmin);
            _revokeRoleIfPresent(IAccessControlLike(address(orderBook)), orderBook.AGENT_RUNTIME_ROLE(), bootstrapAdmin);
            _revokeRoleIfPresent(IAccessControlLike(address(orderBook)), defaultAdminRole, bootstrapAdmin);
        }
    }

    function _configureCollateralVault(
        CollateralVault collateralVault,
        address bootstrapAdmin,
        address admin,
        address operator,
        address pauser,
        address orderBook
    ) internal {
        bytes32 defaultAdminRole = collateralVault.DEFAULT_ADMIN_ROLE();

        _grantRoleIfMissing(IAccessControlLike(address(collateralVault)), collateralVault.OPERATOR_ROLE(), operator);
        _grantRoleIfMissing(IAccessControlLike(address(collateralVault)), collateralVault.OPERATOR_ROLE(), orderBook);
        _grantRoleIfMissing(IAccessControlLike(address(collateralVault)), collateralVault.PAUSER_ROLE(), pauser);

        if (bootstrapAdmin != admin) {
            _grantRoleIfMissing(IAccessControlLike(address(collateralVault)), defaultAdminRole, admin);
            _revokeRoleIfPresent(IAccessControlLike(address(collateralVault)), collateralVault.OPERATOR_ROLE(), bootstrapAdmin);
            _revokeRoleIfPresent(IAccessControlLike(address(collateralVault)), collateralVault.PAUSER_ROLE(), bootstrapAdmin);
            _revokeRoleIfPresent(IAccessControlLike(address(collateralVault)), defaultAdminRole, bootstrapAdmin);
        }
    }

    function _configureAgentRuntime(
        AgentRuntime agentRuntime,
        address bootstrapAdmin,
        address admin,
        address pauser
    ) internal {
        bytes32 defaultAdminRole = agentRuntime.DEFAULT_ADMIN_ROLE();

        _grantRoleIfMissing(IAccessControlLike(address(agentRuntime)), agentRuntime.PAUSER_ROLE(), pauser);

        if (bootstrapAdmin != admin) {
            _grantRoleIfMissing(IAccessControlLike(address(agentRuntime)), defaultAdminRole, admin);
            _revokeRoleIfPresent(IAccessControlLike(address(agentRuntime)), agentRuntime.PAUSER_ROLE(), bootstrapAdmin);
            _revokeRoleIfPresent(IAccessControlLike(address(agentRuntime)), defaultAdminRole, bootstrapAdmin);
        }
    }

    function _grantRoleIfMissing(IAccessControlLike target, bytes32 role, address account) internal {
        if (account == address(0)) revert ZeroAddress("role-account");
        if (!target.hasRole(role, account)) {
            target.grantRole(role, account);
        }
    }

    function _revokeRoleIfPresent(IAccessControlLike target, bytes32 role, address account) internal {
        if (target.hasRole(role, account)) {
            target.revokeRole(role, account);
        }
    }

    function _envAddressOr(string memory key, address fallbackValue) internal view returns (address) {
        try vm.envAddress(key) returns (address value) {
            return value;
        } catch {
            return fallbackValue;
        }
    }

    function _resolveCollateralToken() internal view returns (address) {
        address collateralToken = _envAddressOr("COLLATERAL_TOKEN_ADDRESS", address(0));
        if (collateralToken != address(0)) return collateralToken;

        if (block.chainid == 8453) {
            return _envAddressOr("COLLATERAL_TOKEN_BASE_MAINNET", address(0));
        }
        if (block.chainid == 84532) {
            return _envAddressOr("COLLATERAL_TOKEN_BASE_SEPOLIA", address(0));
        }

        return address(0);
    }
}
