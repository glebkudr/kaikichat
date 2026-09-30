// SPDX-License-Identifier: MIT
pragma solidity 0.8.36;

/// A six-decimal token that behaves like USDC: a transfer beyond the balance
/// or the allowance reverts.
contract MockUsdc {
    uint8 public constant decimals = 6;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    function mint(address to, uint256 amount) external {
        balanceOf[to] += amount;
    }

    function approve(address spender, uint256 amount) external returns (bool) {
        allowance[msg.sender][spender] = amount;
        return true;
    }

    function transfer(address to, uint256 amount) external returns (bool) {
        require(balanceOf[msg.sender] >= amount, "balance");
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
        return true;
    }

    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        require(allowance[from][msg.sender] >= amount, "allowance");
        require(balanceOf[from] >= amount, "balance");
        allowance[from][msg.sender] -= amount;
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        return true;
    }
}

/// An ETH/USD price feed with Chainlink's interface: eight decimals.
contract MockEthUsdFeed {
    uint8 public constant decimals = 8;
    int256 internal answer;
    uint256 internal updatedAt;

    constructor(int256 answer_, uint256 updatedAt_) {
        set(answer_, updatedAt_);
    }

    function set(int256 answer_, uint256 updatedAt_) public {
        answer = answer_;
        updatedAt = updatedAt_;
    }

    function latestRoundData() external view returns (uint80, int256, uint256, uint256, uint80) {
        return (1, answer, updatedAt, updatedAt, 1);
    }
}
