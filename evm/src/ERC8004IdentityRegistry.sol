// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import {AccessControl} from "openzeppelin-contracts/contracts/access/AccessControl.sol";
import {ERC721} from "openzeppelin-contracts/contracts/token/ERC721/ERC721.sol";
import {ERC721URIStorage} from "openzeppelin-contracts/contracts/token/ERC721/extensions/ERC721URIStorage.sol";
import {Pausable} from "openzeppelin-contracts/contracts/utils/Pausable.sol";

contract ERC8004IdentityRegistry is ERC721URIStorage, AccessControl, Pausable {
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
    error IdentityNonTransferable();
    error NotAuthorized();

    event IdentityRegistered(address indexed wallet, uint256 indexed identityId, uint8 tier);
    event TierUpdated(address indexed wallet, uint8 tier);
    event IdentityActivated(address indexed wallet, bool active);
    event IdentityURIUpdated(address indexed wallet, uint256 indexed identityId, string uri);
    event IdentityRevoked(address indexed wallet, uint256 indexed identityId);

    constructor(address admin) ERC721("NeuraMinds Agent Identity", "NMAI") {
        if (admin == address(0)) revert ZeroAddress();
        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(PAUSER_ROLE, admin);
        _grantRole(ISSUER_ROLE, admin);
    }

    function register(address wallet, uint8 tier) external onlyRole(ISSUER_ROLE) whenNotPaused returns (uint256) {
        return _register(wallet, tier, "");
    }

    function registerIdentity(string calldata identityURI) external whenNotPaused returns (uint256) {
        return _register(msg.sender, 0, identityURI);
    }

    function registerIdentityFor(address wallet, string calldata identityURI, uint8 tier, bool active)
        external
        onlyRole(ISSUER_ROLE)
        whenNotPaused
        returns (uint256)
    {
        uint256 identityId = _register(wallet, tier, identityURI);
        if (!active) {
            IdentityProfile storage entry = _profiles[wallet];
            entry.active = false;
            entry.updatedAt = uint64(block.timestamp);
            emit IdentityActivated(wallet, false);
        }
        return identityId;
    }

    function updateIdentityURI(uint256 identityId, string calldata identityURI) external whenNotPaused {
        address wallet = _ownersByIdentity[identityId];
        if (wallet == address(0)) revert NotRegistered();
        if (msg.sender != wallet && !hasRole(ISSUER_ROLE, msg.sender)) revert NotAuthorized();
        _setTokenURI(identityId, identityURI);
        _profiles[wallet].updatedAt = uint64(block.timestamp);
        emit IdentityURIUpdated(wallet, identityId, identityURI);
    }

    function revokeIdentity(uint256 identityId) external onlyRole(ISSUER_ROLE) whenNotPaused {
        address wallet = _ownersByIdentity[identityId];
        if (wallet == address(0)) revert NotRegistered();
        IdentityProfile storage entry = _profiles[wallet];
        entry.active = false;
        entry.updatedAt = uint64(block.timestamp);
        emit IdentityActivated(wallet, false);
        emit IdentityRevoked(wallet, identityId);
    }

    function isRegistered(address wallet) external view returns (bool) {
        return _profiles[wallet].identityId != 0;
    }

    function getAgentId(address wallet) external view returns (uint256) {
        return _profiles[wallet].identityId;
    }

    function registerable(address wallet) external view returns (bool) {
        return wallet != address(0) && _profiles[wallet].identityId == 0;
    }

    function _register(address wallet, uint8 tier, string memory identityURI) internal returns (uint256 identityId) {
        if (wallet == address(0)) revert ZeroAddress();
        if (tier > MAX_TIER) revert InvalidTier();
        if (_profiles[wallet].identityId != 0) revert AlreadyRegistered();

        identityId = ++identityCount;
        uint64 nowTs = uint64(block.timestamp);
        _profiles[wallet] =
            IdentityProfile({identityId: identityId, tier: tier, active: true, createdAt: nowTs, updatedAt: nowTs});
        _ownersByIdentity[identityId] = wallet;

        _mint(wallet, identityId);
        if (bytes(identityURI).length > 0) {
            _setTokenURI(identityId, identityURI);
            emit IdentityURIUpdated(wallet, identityId, identityURI);
        }

        emit IdentityRegistered(wallet, identityId, tier);
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

    function _update(address to, uint256 tokenId, address auth)
        internal
        override(ERC721)
        returns (address)
    {
        address from = _ownerOf(tokenId);
        if (from != address(0) && to != address(0)) revert IdentityNonTransferable();
        return super._update(to, tokenId, auth);
    }

    function supportsInterface(bytes4 interfaceId)
        public
        view
        override(ERC721URIStorage, AccessControl)
        returns (bool)
    {
        return super.supportsInterface(interfaceId);
    }

    function pause() external onlyRole(PAUSER_ROLE) {
        _pause();
    }

    function unpause() external onlyRole(PAUSER_ROLE) {
        _unpause();
    }
}
