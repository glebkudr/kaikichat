// SPDX-License-Identifier: MIT
pragma solidity 0.8.36;

/// Forge cheat codes shared by the contract tests.
interface Vm {
    struct Log {
        bytes32[] topics;
        bytes data;
        address emitter;
    }
    function deal(address, uint256) external;
    function prank(address) external;
    function expectRevert(bytes4) external;
    function recordLogs() external;
    function getRecordedLogs() external returns (Log[] memory);
    function warp(uint256) external;
    function chainId(uint256) external;
    function roll(uint256) external;
    function setBlockhash(uint256, bytes32) external;
    function sign(uint256, bytes32) external returns (uint8, bytes32, bytes32);
    function addr(uint256) external returns (address);
    function getNonce(address) external view returns (uint64);
    function computeCreateAddress(address, uint256) external pure returns (address);
}
