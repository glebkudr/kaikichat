// SPDX-License-Identifier: MIT
pragma solidity 0.8.36;

import {BookShop} from "../src/BookShop.sol";
import {GrantIssuer} from "../src/GrantIssuer.sol";
import {RoyaltySplitter} from "../src/RoyaltySplitter.sol";
import {MockEthUsdFeed, MockUsdc} from "./PaymentMocks.sol";
import {Vm} from "./Vm.sol";

/// The shop prices a book in USD (six decimals, as USDC) and takes USDC at that
/// price or ETH at the ETH/USD feed's rate.
contract BookShopTest {
    Vm internal constant vm = Vm(address(uint160(uint256(keccak256("hevm cheat code")))));

    event BookBought(bytes32 indexed book, address indexed key, address indexed payer, uint32 count, uint64 validUntil);

    bytes32 internal constant DOMAIN = 0xa1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1;
    address internal constant KEY = 0x2222222222222222222222222222222222222222;
    bytes32 internal constant SALT = 0x3333333333333333333333333333333333333333333333333333333333333333;
    /// keccak256(keccak256("AIN_BOOK_V1") ‖ DOMAIN ‖ KEY ‖ SALT), computed with `cast keccak`;
    /// the node's `book_id` test pins the same value.
    bytes32 internal constant BOOK = 0x31165017cd3777e6f74336534ea20d41b9b3df2cdc92685eb57cca728191fa21;
    /// $1.00 for a book of 1000 stamps: a tenth of a cent a message.
    uint256 internal constant PRICE_USDC = 1_000_000;
    uint32 internal constant SIZE = 1000;
    uint64 internal constant VALIDITY = 30 days;
    uint32 internal constant MAX_AGE = 1 hours;
    uint64 internal constant NOW = 1_800_000_000;
    /// $2500.00 an ETH: a book is 0.0004 ETH.
    int256 internal constant RATE = 2_500e8;
    uint256 internal constant QUOTE = 400_000_000_000_000;

    address payable internal treasury = payable(address(0x7EA5));
    address payable internal pool = payable(address(0x9001));
    address internal payer = address(0xB0B);
    RoyaltySplitter internal splitter;
    MockUsdc internal usdc;
    MockEthUsdFeed internal feed;
    BookShop internal shop;

    function setUp() public {
        vm.warp(NOW);
        vm.chainId(84_532);
        address payable[] memory recipients = new address payable[](2);
        recipients[0] = treasury;
        recipients[1] = pool;
        uint16[] memory shares = new uint16[](2);
        shares[0] = 1_000;
        shares[1] = 9_000;
        splitter = new RoyaltySplitter(recipients, shares);
        usdc = new MockUsdc();
        feed = new MockEthUsdFeed(RATE, NOW - 10 minutes);
        shop = newShop(PRICE_USDC, SIZE, VALIDITY, address(splitter), address(usdc), address(feed), MAX_AGE);
        vm.deal(payer, 10 ether);
        usdc.mint(payer, 10_000_000);
    }

    function newShop(
        uint256 price,
        uint32 size,
        uint64 validity,
        address splitter_,
        address usdc_,
        address feed_,
        uint32 maxAge
    ) internal returns (BookShop) {
        return new BookShop(DOMAIN, price, size, validity, RoyaltySplitter(payable(splitter_)), usdc_, feed_, maxAge);
    }

    function buyInEth(address key, bytes32 salt, uint256 value) internal returns (bytes32) {
        vm.prank(payer);
        return shop.buy{value: value}(key, salt);
    }

    function buyInUsdc(address key, bytes32 salt) internal returns (bytes32) {
        vm.prank(payer);
        usdc.approve(address(shop), PRICE_USDC);
        vm.prank(payer);
        return shop.buyWithUsdc(key, salt);
    }

    function recorded(bytes32 book) internal view returns (bool) {
        (address key, uint32 count, uint64 validUntil) = shop.books(book);
        return key == KEY && count == SIZE && validUntil == NOW + VALIDITY;
    }

    function testAQuoteIsTheUsdPriceAtTheFeedsRateRoundedUp() public {
        require(shop.quote() == QUOTE, "quote at $2500");
        // $3000: 0.000333… ETH, rounded up to the next wei.
        feed.set(3_000e8, NOW);
        require(shop.quote() == 333_333_333_333_334, "quote at $3000");
    }

    function testAnEthPurchasePaysTheQuoteAndGetsTheRestBack() public {
        vm.recordLogs();
        uint256 before = payer.balance;
        bytes32 book = buyInEth(KEY, SALT, QUOTE + QUOTE / 100);
        require(book == BOOK && shop.bookId(KEY, SALT) == BOOK && recorded(BOOK), "recorded terms");
        require(payer.balance == before - QUOTE, "paid more than the quote");
        require(address(shop).balance == 0, "shop kept a balance");
        require(splitter.totalReceivedWei() == QUOTE, "payment not forwarded");
        require(splitter.credits(treasury) == QUOTE / 10, "treasury share");
        require(splitter.credits(pool) == QUOTE - QUOTE / 10, "operator pool share");

        Vm.Log[] memory logs = vm.getRecordedLogs();
        bool seen;
        for (uint256 i; i < logs.length; ++i) {
            if (logs[i].emitter == address(shop)) {
                require(!seen, "one purchase event");
                seen = true;
                require(logs[i].topics[0] == BookBought.selector, "event");
                require(logs[i].topics[1] == BOOK, "event book");
                require(logs[i].topics[2] == bytes32(uint256(uint160(KEY))), "event key");
                require(logs[i].topics[3] == bytes32(uint256(uint160(payer))), "event payer");
                require(keccak256(logs[i].data) == keccak256(abi.encode(SIZE, NOW + VALIDITY)), "event terms");
            }
        }
        require(seen, "purchase event missing");
    }

    function testLessEthThanTheQuoteIsRefused() public {
        vm.expectRevert(BookShop.WrongPayment.selector);
        buyInEth(KEY, SALT, QUOTE - 1);
        (address key,,) = shop.books(BOOK);
        require(key == address(0) && splitter.totalReceivedWei() == 0, "refused payment kept");
    }

    function testAUsdcPurchaseTakesExactlyThePriceAndSplitsIt() public {
        bytes32 book = buyInUsdc(KEY, SALT);
        require(book == BOOK && recorded(BOOK), "recorded terms");
        require(usdc.balanceOf(payer) == 10_000_000 - PRICE_USDC, "paid other than the price");
        require(usdc.balanceOf(address(shop)) == 0, "shop kept tokens");
        require(usdc.balanceOf(address(splitter)) == PRICE_USDC, "payment not forwarded");
        require(splitter.tokenCredits(address(usdc), treasury) == PRICE_USDC / 10, "treasury share");
        require(splitter.tokenCredits(address(usdc), pool) == PRICE_USDC - PRICE_USDC / 10, "operator pool share");
        require(splitter.totalReceivedWei() == 0, "no ETH moved");
    }

    function testUsdcWithoutAnAllowanceBuysNothing() public {
        vm.expectRevert(BookShop.PaymentFailed.selector);
        vm.prank(payer);
        shop.buyWithUsdc(KEY, SALT);
        (address key,,) = shop.books(BOOK);
        require(key == address(0) && usdc.balanceOf(payer) == 10_000_000, "refused payment kept");
    }

    /// A stale or broken rate stops ETH purchases; USDC still buys.
    function testWithoutAFreshRateOnlyUsdcBuys() public {
        feed.set(RATE, NOW - MAX_AGE - 1);
        vm.expectRevert(BookShop.StalePrice.selector);
        shop.quote();
        vm.expectRevert(BookShop.StalePrice.selector);
        buyInEth(KEY, SALT, 1 ether);
        feed.set(0, NOW);
        vm.expectRevert(BookShop.BadPrice.selector);
        buyInEth(KEY, SALT, 1 ether);
        feed.set(-1, NOW);
        vm.expectRevert(BookShop.BadPrice.selector);
        shop.quote();
        require(buyInUsdc(KEY, SALT) == BOOK && recorded(BOOK), "USDC purchase");
    }

    /// The operator pool settles each UTC day's sales once their tickets expire.
    function testBooksSoldAreCountedByUtcDay() public {
        uint64 day = NOW / 1 days;
        buyInUsdc(KEY, SALT);
        buyInEth(KEY, bytes32(uint256(SALT) + 1), QUOTE);
        vm.warp((day + 1) * 1 days);
        feed.set(RATE, (day + 1) * 1 days);
        buyInEth(KEY, bytes32(uint256(SALT) + 2), QUOTE);
        require(shop.soldOn(day) == 2 && shop.soldOn(day + 1) == 1 && shop.soldOn(day - 1) == 0, "sold by day");
    }

    function testAnUnboughtBookReadsAsNoKey() public view {
        (address key, uint32 count, uint64 validUntil) = shop.books(BOOK);
        require(key == address(0) && count == 0 && validUntil == 0, "unbought book");
    }

    function testABookNeedsAKey() public {
        vm.expectRevert(BookShop.InvalidKey.selector);
        buyInEth(address(0), SALT, QUOTE);
        vm.prank(payer);
        usdc.approve(address(shop), PRICE_USDC);
        vm.expectRevert(BookShop.InvalidKey.selector);
        vm.prank(payer);
        shop.buyWithUsdc(address(0), SALT);
    }

    function testABookIsBoughtOnceWhoeverPaysAndHowever() public {
        buyInEth(KEY, SALT, QUOTE);
        vm.warp(NOW + 1 days);
        feed.set(RATE, NOW + 1 days);
        vm.expectRevert(BookShop.AlreadyBought.selector);
        buyInEth(KEY, SALT, QUOTE);
        // Someone else, in USDC: the same book is not sold again.
        address other = address(0xCA701);
        usdc.mint(other, PRICE_USDC);
        vm.prank(other);
        usdc.approve(address(shop), PRICE_USDC);
        vm.expectRevert(BookShop.AlreadyBought.selector);
        vm.prank(other);
        shop.buyWithUsdc(KEY, SALT);
        (,, uint64 validUntil) = shop.books(BOOK);
        require(validUntil == NOW + VALIDITY, "second purchase extended the book");
        require(splitter.totalReceivedWei() == QUOTE && usdc.balanceOf(address(splitter)) == 0, "second payment taken");
    }

    function testAnotherSaltIsAnotherBookOfTheSameKey() public {
        bytes32 first = buyInEth(KEY, SALT, QUOTE);
        bytes32 second = buyInUsdc(KEY, bytes32(uint256(SALT) + 1));
        require(first != second, "same book id");
        (address key,,) = shop.books(second);
        require(key == KEY, "second book");
    }

    function testConfigurationMustBeComplete() public {
        address s = address(splitter);
        address u = address(usdc);
        address f = address(feed);
        vm.expectRevert(BookShop.InvalidConfiguration.selector);
        new BookShop(bytes32(0), PRICE_USDC, SIZE, VALIDITY, splitter, u, f, MAX_AGE);
        vm.expectRevert(BookShop.InvalidConfiguration.selector);
        newShop(0, SIZE, VALIDITY, s, u, f, MAX_AGE);
        vm.expectRevert(BookShop.InvalidConfiguration.selector);
        newShop(PRICE_USDC, 0, VALIDITY, s, u, f, MAX_AGE);
        vm.expectRevert(BookShop.InvalidConfiguration.selector);
        newShop(PRICE_USDC, SIZE, 0, s, u, f, MAX_AGE);
        vm.expectRevert(BookShop.InvalidConfiguration.selector);
        newShop(PRICE_USDC, SIZE, VALIDITY, address(0), u, f, MAX_AGE);
        vm.expectRevert(BookShop.InvalidConfiguration.selector);
        newShop(PRICE_USDC, SIZE, VALIDITY, s, address(0), f, MAX_AGE);
        vm.expectRevert(BookShop.InvalidConfiguration.selector);
        newShop(PRICE_USDC, SIZE, VALIDITY, s, u, address(0), MAX_AGE);
        vm.expectRevert(BookShop.InvalidConfiguration.selector);
        newShop(PRICE_USDC, SIZE, VALIDITY, s, u, f, 0);
    }

    /// The node reads these views with hand-encoded calls and hands out this
    /// calldata; their selectors and return words must not drift.
    function testTheNodeReadsTheseSelectorsAndWords() public {
        // Getters of public state are selected through an instance.
        GrantIssuer issuer = GrantIssuer(address(0));
        require(shop.books.selector == bytes4(0x0c0dee70), "books selector");
        require(shop.priceUsdc.selector == bytes4(0x34189d5e), "priceUsdc selector");
        require(shop.usdc.selector == bytes4(0x3e413bee), "usdc selector");
        require(BookShop.quote.selector == bytes4(0x999b93af), "quote selector");
        require(shop.validity.selector == bytes4(0x3e98d1fb), "validity selector");
        require(shop.bookSize.selector == bytes4(0x39f65888), "shop bookSize selector");
        // `coins buy` hands out this calldata for a wallet to send.
        require(BookShop.buy.selector == bytes4(0x9058e228), "buy selector");
        require(BookShop.buyWithUsdc.selector == bytes4(0xa10572fd), "buyWithUsdc selector");
        require(issuer.bookSize.selector == bytes4(0x39f65888), "bookSize selector");
        require(issuer.maxValidityDays.selector == bytes4(0xa7911730), "maxValidityDays selector");
        require(GrantIssuer.issuerActiveOn.selector == bytes4(0x6619052a), "issuerActiveOn selector");
        require(GrantIssuer.capForDay.selector == bytes4(0xff33ddf4), "capForDay selector");
        require(GrantIssuer.today.selector == bytes4(0xb74e452b), "today selector");

        BookShop small = newShop(PRICE_USDC, 100, VALIDITY, address(splitter), address(usdc), address(feed), MAX_AGE);
        vm.prank(payer);
        small.buy{value: QUOTE}(KEY, SALT);
        (bool success, bytes memory words) = address(small).staticcall(abi.encodeWithSelector(bytes4(0x0c0dee70), BOOK));
        // The node's chain reader test (`BOOK_WORDS` in chain_tests.rs) answers with these words.
        require(
            success
                && keccak256(words)
                    == keccak256(
                        hex"0000000000000000000000002222222222222222222222222222222222222222"
                        hex"0000000000000000000000000000000000000000000000000000000000000064"
                        hex"000000000000000000000000000000000000000000000000000000006b715f00"
                    ),
            "books return words"
        );
    }
}
