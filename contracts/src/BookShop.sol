// SPDX-License-Identifier: MIT
pragma solidity 0.8.36;

import {RoyaltySplitter} from "./RoyaltySplitter.sol";

/// Chainlink's price feed interface, as much of it as the shop reads.
interface PriceFeed {
    function decimals() external view returns (uint8);
    function latestRoundData()
        external
        view
        returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound);
}

/// @notice Sells stamp books: a book pays for `bookSize` messages signed by its key until
/// `validUntil`. Holders read `books` at a confirmed block and check stamps against it.
/// A book is priced in USD (`priceUsdc`, in USDC units) and paid either in USDC or in ETH
/// at the ETH/USD feed's rate.
/// @dev No owner and no balance: every payment goes to an immutable royalty splitter
/// (treasury share and operator pool). Changing price or terms means a new deployment.
/// The operator pool settles each UTC day's sales by `soldOn` once their tickets expire.
contract BookShop {
    error InvalidConfiguration();
    error WrongPayment();
    error InvalidKey();
    error AlreadyBought();
    error StalePrice();
    error BadPrice();
    error PaymentFailed();
    error RefundFailed();

    struct Book {
        address key;
        uint32 count;
        uint64 validUntil;
    }

    event BookBought(bytes32 indexed book, address indexed key, address indexed payer, uint32 count, uint64 validUntil);

    bytes32 private constant BOOK_TAG = keccak256("AIN_BOOK_V1");

    bytes32 public immutable domain;
    /// A book's price in USDC units (six decimals: 1_000_000 is $1.00).
    uint256 public immutable priceUsdc;
    uint32 public immutable bookSize;
    uint64 public immutable validity;
    RoyaltySplitter public immutable splitter;
    address public immutable usdc;
    PriceFeed public immutable ethUsd;
    /// The oldest feed answer an ETH purchase is priced at, in seconds.
    uint32 public immutable maxPriceAge;
    /// Wei per USDC unit at a rate of one: 10^(18 - token decimals + feed decimals).
    uint256 private immutable weiScale;

    mapping(bytes32 book => Book) public books;
    /// Books sold on each UTC day (`timestamp / 1 days`).
    mapping(uint64 day => uint64 count) public soldOn;

    constructor(
        bytes32 domain_,
        uint256 priceUsdc_,
        uint32 bookSize_,
        uint64 validity_,
        RoyaltySplitter splitter_,
        address usdc_,
        address ethUsd_,
        uint32 maxPriceAge_
    ) {
        if (
            domain_ == bytes32(0) || priceUsdc_ == 0 || bookSize_ == 0 || validity_ == 0
                || address(splitter_) == address(0) || usdc_.code.length == 0 || ethUsd_.code.length == 0
                || maxPriceAge_ == 0
        ) {
            revert InvalidConfiguration();
        }
        uint8 tokenDecimals = PriceFeed(usdc_).decimals();
        uint8 feedDecimals = PriceFeed(ethUsd_).decimals();
        if (tokenDecimals > 18 || feedDecimals > 18) revert InvalidConfiguration();
        domain = domain_;
        priceUsdc = priceUsdc_;
        bookSize = bookSize_;
        validity = validity_;
        splitter = splitter_;
        usdc = usdc_;
        ethUsd = PriceFeed(ethUsd_);
        maxPriceAge = maxPriceAge_;
        weiScale = 10 ** (18 - tokenDecimals + feedDecimals);
    }

    /// The node's book id: keccak256(keccak256("AIN_BOOK_V1") ‖ domain ‖ key ‖ salt).
    function bookId(address key, bytes32 salt) public view returns (bytes32) {
        return keccak256(abi.encodePacked(BOOK_TAG, domain, key, salt));
    }

    /// A book's price in wei at the feed's latest rate, rounded up. Reverts without a
    /// fresh, positive rate: then only USDC buys.
    function quote() public view returns (uint256) {
        (, int256 answer,, uint256 updatedAt,) = ethUsd.latestRoundData();
        if (answer <= 0) revert BadPrice();
        if (updatedAt == 0 || updatedAt > block.timestamp || block.timestamp - updatedAt > maxPriceAge) {
            revert StalePrice();
        }
        uint256 rate = uint256(answer);
        return (priceUsdc * weiScale + rate - 1) / rate;
    }

    /// Buys the book of `key` and `salt` in ETH: the quote goes to the splitter, the rest
    /// back to the sender. Anyone may pay; the stamps can only be signed by `key`.
    function buy(address key, bytes32 salt) external payable returns (bytes32 book) {
        uint256 cost = quote();
        if (msg.value < cost) revert WrongPayment();
        book = record(key, salt);
        splitter.deposit{value: cost}();
        if (msg.value > cost) {
            (bool refunded,) = msg.sender.call{value: msg.value - cost}("");
            if (!refunded) revert RefundFailed();
        }
    }

    /// Buys the book of `key` and `salt` for exactly `priceUsdc`, which the sender allowed
    /// this shop to take; the tokens go straight to the splitter.
    function buyWithUsdc(address key, bytes32 salt) external returns (bytes32 book) {
        book = record(key, salt);
        (bool called, bytes memory result) = usdc.call(
            abi.encodeWithSignature("transferFrom(address,address,uint256)", msg.sender, address(splitter), priceUsdc)
        );
        if (!called || (result.length != 0 && (result.length != 32 || !abi.decode(result, (bool))))) {
            revert PaymentFailed();
        }
        splitter.depositToken(usdc);
    }

    function record(address key, bytes32 salt) private returns (bytes32 book) {
        if (key == address(0)) revert InvalidKey();
        book = bookId(key, salt);
        if (books[book].key != address(0)) revert AlreadyBought();
        uint64 validUntil = uint64(block.timestamp) + validity;
        books[book] = Book(key, bookSize, validUntil);
        ++soldOn[uint64(block.timestamp / 1 days)];
        emit BookBought(book, key, msg.sender, bookSize, validUntil);
    }
}
