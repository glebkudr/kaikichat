// SPDX-License-Identifier: MIT
pragma solidity 0.8.36;

/// @notice Immutable pull-payment royalty splitter with no governance surface, for ETH
/// and for tokens (USDC).
/// @dev Recipients only receive credits and cannot mint, pause, upgrade or block each other.
contract RoyaltySplitter {
    error InvalidConfiguration();
    error InvalidAmount();
    error NoCredit();
    error InvalidRecipient();
    error TransferFailed();
    error WrongChain();

    event RoyaltyDeposited(address indexed payer, uint256 amount);
    event RoyaltyWithdrawn(address indexed account, address indexed recipient, uint256 amount);
    event TokenRoyaltyDeposited(address indexed token, uint256 amount);
    event TokenRoyaltyWithdrawn(
        address indexed token, address indexed account, address indexed recipient, uint256 amount
    );

    uint16 private constant BPS = 10_000;
    bytes32 private constant DOMAIN_TAG = keccak256("AgenticInternet/RoyaltySplitter/v1");

    uint256 public immutable deploymentChainId;
    bytes32 public immutable configHash;
    bytes32 public immutable domain;

    uint256 public totalReceivedWei;
    uint256 public totalCreditedWei;
    mapping(address account => uint256 amount) public credits;
    mapping(address token => mapping(address account => uint256 amount)) public tokenCredits;
    /// Tokens credited and not withdrawn: what arrives beyond it is new.
    mapping(address token => uint256 amount) public tokenOwed;
    address payable[] private recipients;
    uint16[] private sharesBps;

    constructor(address payable[] memory accounts, uint16[] memory shares) {
        uint256 count = accounts.length;
        if (block.chainid == 0 || count == 0 || count > 16 || count != shares.length) {
            revert InvalidConfiguration();
        }
        uint256 total;
        for (uint256 i; i < count; ++i) {
            if (accounts[i] == address(0) || shares[i] == 0) revert InvalidConfiguration();
            for (uint256 j; j < i; ++j) {
                if (accounts[i] == accounts[j]) revert InvalidConfiguration();
            }
            total += shares[i];
            recipients.push(accounts[i]);
            sharesBps.push(shares[i]);
        }
        if (total != BPS) revert InvalidConfiguration();
        deploymentChainId = block.chainid;
        configHash = keccak256(abi.encode(accounts, shares));
        domain = keccak256(abi.encode(DOMAIN_TAG, block.chainid, address(this), configHash));
    }

    modifier sameChain() {
        if (block.chainid != deploymentChainId) revert WrongChain();
        _;
    }

    receive() external payable {
        deposit();
    }

    function recipientCount() external view returns (uint256) {
        return recipients.length;
    }

    function recipient(uint256 index) external view returns (address account, uint16 shareBps) {
        return (recipients[index], sharesBps[index]);
    }

    function deposit() public payable sameChain {
        if (msg.value == 0) revert InvalidAmount();
        uint256[] memory parts = split(msg.value);
        for (uint256 i; i < parts.length; ++i) {
            credits[recipients[i]] += parts[i];
        }
        totalReceivedWei += msg.value;
        totalCreditedWei += msg.value;
        emit RoyaltyDeposited(msg.sender, msg.value);
    }

    function withdrawRoyalty(address payable recipient_) external sameChain {
        if (recipient_ == address(0)) revert InvalidRecipient();
        uint256 amount = credits[msg.sender];
        if (amount == 0) revert NoCredit();
        credits[msg.sender] = 0;
        (bool paid,) = recipient_.call{value: amount}("");
        if (!paid) revert TransferFailed();
        emit RoyaltyWithdrawn(msg.sender, recipient_, amount);
    }

    /// Credits by the shares the tokens that arrived since the last deposit. Anyone may
    /// call it; only tokens actually held are credited.
    function depositToken(address token) external sameChain returns (uint256 amount) {
        (bool called, bytes memory result) =
            token.staticcall(abi.encodeWithSignature("balanceOf(address)", address(this)));
        if (!called || result.length != 32) revert InvalidAmount();
        uint256 held = abi.decode(result, (uint256));
        if (held <= tokenOwed[token]) revert InvalidAmount();
        amount = held - tokenOwed[token];
        uint256[] memory parts = split(amount);
        for (uint256 i; i < parts.length; ++i) {
            tokenCredits[token][recipients[i]] += parts[i];
        }
        tokenOwed[token] = held;
        emit TokenRoyaltyDeposited(token, amount);
    }

    function withdrawTokenRoyalty(address token, address recipient_) external sameChain {
        if (recipient_ == address(0)) revert InvalidRecipient();
        uint256 amount = tokenCredits[token][msg.sender];
        if (amount == 0) revert NoCredit();
        tokenCredits[token][msg.sender] = 0;
        tokenOwed[token] -= amount;
        (bool called, bytes memory result) =
            token.call(abi.encodeWithSignature("transfer(address,uint256)", recipient_, amount));
        if (!called || (result.length != 0 && (result.length != 32 || !abi.decode(result, (bool))))) {
            revert TransferFailed();
        }
        emit TokenRoyaltyWithdrawn(token, msg.sender, recipient_, amount);
    }

    /// `amount` by the shares; the last recipient takes the rounding.
    function split(uint256 amount) private view returns (uint256[] memory parts) {
        uint256 count = recipients.length;
        parts = new uint256[](count);
        uint256 remaining = amount;
        for (uint256 i; i < count; ++i) {
            parts[i] = i + 1 == count ? remaining : amount * sharesBps[i] / BPS;
            remaining -= parts[i];
        }
    }
}
