// SPDX-License-Identifier: MIT
pragma solidity 0.8.36;

import {BookShop} from "../src/BookShop.sol";
import {NodeRegistry} from "../src/NodeRegistry.sol";
import {OperatorPool} from "../src/OperatorPool.sol";
import {RoyaltySplitter} from "../src/RoyaltySplitter.sol";
import {MockEthUsdFeed, MockUsdc} from "./PaymentMocks.sol";
import {Vm} from "./Vm.sol";

/// The operator pool pays the holders a paid stamp names: a stamp wins with
/// odds that make ten prizes of $0.10 worth the pool's share of the stamp, and
/// each named holder's node or owner withdraws its prize. What nobody
/// withdraws before the tickets expire goes to the treasury.
contract OperatorPoolTest {
    Vm internal constant vm = Vm(address(uint160(uint256(keccak256("hevm cheat code")))));

    bytes32 internal constant DOMAIN = 0xa1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1;
    /// $1.00 for a book of 1000 stamps; the pool's 90% is $0.0009 a stamp.
    uint256 internal constant PRICE_USDC = 1_000_000;
    uint32 internal constant SIZE = 1000;
    uint64 internal constant VALIDITY = 30 days;
    uint32 internal constant MAX_AGE = 1 hours;
    uint256 internal constant SHARE_PER_BOOK = 900_000;
    /// $0.10 to each of the ten holders a winning stamp names.
    uint256 internal constant PRIZE = 100_000;
    uint64 internal constant LIFETIME = 360 days;
    uint16 internal constant SEED_DELAY = 5;
    uint64 internal constant NOW = 1_800_000_000;
    uint64 internal constant DAY = NOW / 1 days;
    /// $2500.00 an ETH: a book is 0.0004 ETH, the pool's share 0.00036 ETH.
    int256 internal constant RATE = 2_500e8;
    uint256 internal constant QUOTE = 400_000_000_000_000;
    uint256 internal constant POOL_WEI_PER_BOOK = QUOTE - QUOTE / 10;
    uint256 internal constant BOOK_KEY = 0xB00C;
    uint256 internal constant OTHER_KEY = 0x0BAD;
    uint64 internal constant PERIOD = 20_833;

    address payable internal treasury = payable(address(0x7EA5));
    address internal payer = address(0xB0B);
    RoyaltySplitter internal splitter;
    MockUsdc internal usdc;
    MockEthUsdFeed internal feed;
    BookShop internal shop;
    NodeRegistry internal registry;
    OperatorPool internal pool;
    uint256 internal salts;

    /// Ten bonded units: unit k has receipt key 0xA000 + k, transport key
    /// 0x7000 + k and owner 0xC000 + k.
    bytes32[10] internal units;

    function setUp() public {
        vm.warp(NOW);
        vm.roll(1_000);
        vm.chainId(84_532);
        usdc = new MockUsdc();
        feed = new MockEthUsdFeed(RATE, NOW);
        registry = new NodeRegistry(keccak256("genesis"), 1e14, 3600, 7200, 7 days, 5);
        for (uint256 k; k < 10; ++k) {
            units[k] = commitment(transport(k), vm.addr(receiptKey(k)));
            vm.deal(owner(k), 1 ether);
            vm.prank(owner(k));
            require(registry.bond{value: 1e14}(units[k]) == k, "unit index");
        }
        (splitter, shop, pool) = deploy(treasury, PRIZE, LIFETIME, SEED_DELAY, DAY);
        vm.deal(payer, 10 ether);
        usdc.mint(payer, 1_000_000_000);
        vm.prank(payer);
        usdc.approve(address(shop), type(uint256).max);
    }

    // ---------------------------------------------------------------- helpers

    function receiptKey(uint256 k) internal pure returns (uint256) {
        return 0xA000 + k;
    }

    function transport(uint256 k) internal pure returns (bytes32) {
        return bytes32(0x7000 + k);
    }

    function owner(uint256 k) internal pure returns (address) {
        return address(uint160(0xC000 + k));
    }

    function commitment(bytes32 transportKey, address receipt) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(keccak256("AIN_UNIT_V1"), DOMAIN, transportKey, receipt));
    }

    /// A splitter paying 9/10 to the pool deployed two creations later, the
    /// shop between them, then the pool.
    function deploy(address treasury_, uint256 prize, uint64 lifetime, uint16 delay, uint64 firstDay)
        internal
        returns (RoyaltySplitter splitter_, BookShop shop_, OperatorPool pool_)
    {
        address predicted = vm.computeCreateAddress(address(this), vm.getNonce(address(this)) + 2);
        splitter_ = newSplitter(treasury_, predicted);
        shop_ = new BookShop(DOMAIN, PRICE_USDC, SIZE, VALIDITY, splitter_, address(usdc), address(feed), MAX_AGE);
        pool_ = new OperatorPool(shop_, registry, treasury_, prize, lifetime, delay, firstDay);
        require(address(pool_) == predicted, "predicted pool address");
    }

    function newSplitter(address treasury_, address pool_) internal returns (RoyaltySplitter) {
        address payable[] memory recipients = new address payable[](2);
        recipients[0] = payable(treasury_);
        recipients[1] = payable(pool_);
        uint16[] memory shares = new uint16[](2);
        shares[0] = 1_000;
        shares[1] = 9_000;
        return new RoyaltySplitter(recipients, shares);
    }

    function buyUsdc() internal returns (bytes32) {
        vm.prank(payer);
        return shop.buyWithUsdc(vm.addr(BOOK_KEY), bytes32(++salts));
    }

    function buyEth() internal returns (bytes32) {
        feed.set(RATE, block.timestamp);
        vm.prank(payer);
        return shop.buy{value: QUOTE}(vm.addr(BOOK_KEY), bytes32(++salts));
    }

    /// Fixes the seed of `day` from `hash`: after the day ends, arm it, then
    /// capture it from the target block.
    function settle(uint64 day, bytes32 hash) internal {
        if (block.timestamp < (day + 1) * 1 days) vm.warp((day + 1) * 1 days);
        uint64 target = pool.armSeed(day);
        require(target == block.number + SEED_DELAY, "seed target");
        vm.roll(target + 1);
        vm.setBlockhash(target, hash);
        require(pool.captureSeed(day) == hash, "the seed is the target block's hash");
    }

    /// Every winning slot of `books`, in book order.
    function winners(bytes32[] memory books) internal view returns (bytes32[] memory book, uint32[] memory index) {
        uint256 found;
        // About 0.9 winning slots a book: eight is far above what any test sees.
        book = new bytes32[](books.length * 8);
        index = new uint32[](books.length * 8);
        for (uint256 b; b < books.length; ++b) {
            for (uint32 i; i < SIZE; ++i) {
                (bool known, bool won) = pool.wins(books[b], i);
                require(known, "seed of the purchase day");
                if (won) {
                    require(found < book.length, "more winning slots than expected");
                    book[found] = books[b];
                    index[found++] = i;
                }
            }
        }
        assembly {
            mstore(book, found)
            mstore(index, found)
        }
    }

    /// The first losing slot of `book`.
    function loser(bytes32 book) internal view returns (uint32 i) {
        for (;; ++i) {
            (bool known, bool won) = pool.wins(book, i);
            require(known, "seed of the purchase day");
            if (!won) return i;
        }
    }

    /// USDC the pool holds or may pull from the splitter.
    function poolUsdc() internal view returns (uint256) {
        return usdc.balanceOf(address(pool)) + splitter.tokenCredits(address(usdc), address(pool));
    }

    function allUnits() internal view returns (bytes32[10] memory holders) {
        holders = units;
    }

    /// The stamp `key` signs for a message to `mailbox` naming `holders`,
    /// claimed by the holder at `position`.
    function ticket(
        uint256 key,
        bytes32 book,
        uint32 index,
        bytes32 mailbox,
        bytes32[10] memory holders,
        uint8 position
    ) internal returns (OperatorPool.Ticket memory t) {
        t.book = book;
        t.index = index;
        t.mailbox = mailbox;
        t.period = PERIOD;
        t.envelope = keccak256(abi.encode("envelope", mailbox));
        t.holders = holders;
        t.position = position;
        bytes32 swarm = keccak256(abi.encodePacked(keccak256("AIN_SWARM_V1"), holders));
        bytes32 operation =
            keccak256(abi.encodePacked(keccak256("AIN_OPERATION_V2"), mailbox, PERIOD, swarm, t.envelope));
        bytes32 digest = keccak256(abi.encodePacked(keccak256("AIN_STAMP_V1"), DOMAIN, book, index, operation));
        (t.v, t.r, t.s) = vm.sign(key, digest);
    }

    /// The paid stamp of slot (book, index), to the ten units, claimed by unit `k`.
    function ticketFor(uint256 k, bytes32 book, uint32 index) internal returns (OperatorPool.Ticket memory) {
        return ticket(BOOK_KEY, book, index, keccak256("mailbox"), allUnits(), uint8(k));
    }

    function one(OperatorPool.Ticket memory t) internal pure returns (OperatorPool.Ticket[] memory list) {
        list = new OperatorPool.Ticket[](1);
        list[0] = t;
    }

    /// Unit `k`'s node claims `list` with its receipt key.
    function claimAsNode(uint256 k, OperatorPool.Ticket[] memory list) internal returns (uint256) {
        vm.prank(vm.addr(receiptKey(k)));
        return pool.claim(uint32(k), transport(k), list);
    }

    /// Books bought today, and the first winning slot among them once the day's
    /// seed is fixed from `hash`.
    function winningSlot(uint256 count, bytes32 hash) internal returns (bytes32 book, uint32 index) {
        bytes32[] memory books = new bytes32[](count);
        for (uint256 b; b < count; ++b) {
            books[b] = buyUsdc();
        }
        settle(uint64(block.timestamp / 1 days), hash);
        (bytes32[] memory wonBook, uint32[] memory wonIndex) = winners(books);
        require(wonBook.length > 0, "a winning slot among the books");
        return (wonBook[0], wonIndex[0]);
    }

    // ------------------------------------------------------------ the odds

    /// Ten prizes of $0.10 at these odds are worth $0.0009, the pool's share
    /// of a $1 book of 1000 stamps.
    function testTheOddsMakeTenPrizesWorthThePoolsShareOfAStamp() public view {
        // ⌊(2^256 − 1) / (prize × 10 × book size × 10000)⌋ × (price × 9000), computed apart.
        require(pool.winThreshold() == 0x3afb7e90ff972474538ef34d6a161e4f765fd8adab9f559b3d07c6a2df8000, "threshold");
        // threshold / 2^256 × 10 prizes, in USDC units: $0.0009 less rounding.
        uint256 perStamp = ((pool.winThreshold() >> 128) * (PRIZE * 10)) >> 128;
        require(perStamp == 899 || perStamp == 900, "expected payout per paid stamp");
        require(pool.prizeUsdc() == PRIZE && pool.ticketLifetime() == LIFETIME, "terms");
    }

    // --------------------------------------------------------- withdrawals

    function testAWinningStampPaysEachNamedHolderTenCentsInUsdc() public {
        (bytes32 book, uint32 index) = winningSlot(5, keccak256("pay"));
        require(claimAsNode(3, one(ticketFor(3, book, index))) == PRIZE, "credited to unit 3");
        require(usdc.balanceOf(owner(3)) == PRIZE, "paid to unit 3's owner");
        require(claimAsNode(7, one(ticketFor(7, book, index))) == PRIZE, "credited to unit 7");
        require(usdc.balanceOf(owner(7)) == PRIZE, "paid to unit 7's owner");
        require(pool.owed(3) == 0 && pool.owed(7) == 0 && pool.totalOwed() == 0, "nothing left owed");
    }

    function testManyTicketsArePaidInOneWithdrawal() public {
        bytes32[] memory books = new bytes32[](10);
        for (uint256 b; b < 10; ++b) {
            books[b] = buyUsdc();
        }
        settle(DAY, keccak256("many"));
        (bytes32[] memory book, uint32[] memory index) = winners(books);
        require(book.length >= 3, "several winning slots");
        OperatorPool.Ticket[] memory list = new OperatorPool.Ticket[](book.length);
        for (uint256 i; i < book.length; ++i) {
            list[i] = ticketFor(0, book[i], index[i]);
        }
        require(claimAsNode(0, list) == PRIZE * book.length, "credited");
        require(usdc.balanceOf(owner(0)) == PRIZE * book.length, "paid");
    }

    function testTheUnitOwnerMayWithdrawForItsNode() public {
        (bytes32 book, uint32 index) = winningSlot(5, keccak256("pay"));
        vm.prank(owner(2));
        require(pool.claim(2, bytes32(0), one(ticketFor(2, book, index))) == PRIZE, "credited");
        require(usdc.balanceOf(owner(2)) == PRIZE, "paid to the owner");
    }

    function testNobodyElseStartsAWithdrawal() public {
        (bytes32 book, uint32 index) = winningSlot(5, keccak256("pay"));
        OperatorPool.Ticket[] memory list = one(ticketFor(2, book, index));
        vm.expectRevert(OperatorPool.Unauthorized.selector);
        vm.prank(address(0xDEAD));
        pool.claim(2, transport(2), list);
        // Unit 5's node signs for unit 2.
        vm.expectRevert(OperatorPool.Unauthorized.selector);
        vm.prank(vm.addr(receiptKey(5)));
        pool.claim(2, transport(5), list);
        // Unit 2's receipt account with another transport key is another unit.
        vm.expectRevert(OperatorPool.Unauthorized.selector);
        vm.prank(vm.addr(receiptKey(2)));
        pool.claim(2, transport(5), list);
        // An index nobody bonded.
        vm.expectRevert(OperatorPool.Unauthorized.selector);
        vm.prank(vm.addr(receiptKey(2)));
        pool.claim(99, transport(2), list);
        require(usdc.balanceOf(owner(2)) == 0, "paid");
    }

    function testAnOperatorLeavingTheNetworkStillCollectsWhatItEarned() public {
        (bytes32 book, uint32 index) = winningSlot(5, keccak256("pay"));
        vm.prank(owner(4));
        registry.requestExit(4);
        require(claimAsNode(4, one(ticketFor(4, book, index))) == PRIZE, "credited");
        require(usdc.balanceOf(owner(4)) == PRIZE, "paid");
    }

    function testAHolderTheStampDoesNotNameIsRefused() public {
        (bytes32 book, uint32 index) = winningSlot(5, keccak256("pay"));
        // Units 0..8 and a stranger: unit 9 is not named.
        bytes32[10] memory holders = allUnits();
        holders[9] = keccak256("stranger");
        OperatorPool.Ticket memory t = ticket(BOOK_KEY, book, index, keccak256("mailbox"), holders, 9);
        vm.expectRevert(OperatorPool.NotHolder.selector);
        claimAsNode(9, one(t));
        // Unit 9 pointing at unit 0's place.
        t = ticket(BOOK_KEY, book, index, keccak256("mailbox"), holders, 0);
        vm.expectRevert(OperatorPool.NotHolder.selector);
        claimAsNode(9, one(t));
        // A place past the list.
        t = ticket(BOOK_KEY, book, index, keccak256("mailbox"), allUnits(), 10);
        vm.expectRevert(OperatorPool.NotHolder.selector);
        claimAsNode(9, one(t));
    }

    function testAStampNamingAUnitTwicePaysItAtNeitherPlace() public {
        (bytes32 book, uint32 index) = winningSlot(5, keccak256("pay"));
        bytes32[10] memory holders = allUnits();
        holders[5] = units[0];
        OperatorPool.Ticket memory t = ticket(BOOK_KEY, book, index, keccak256("mailbox"), holders, 0);
        vm.expectRevert(OperatorPool.NotHolder.selector);
        claimAsNode(0, one(t));
        t.position = 5;
        vm.expectRevert(OperatorPool.NotHolder.selector);
        claimAsNode(0, one(t));
    }

    function testOnlyAStampOfTheBooksKeyInsideTheBookPays() public {
        (bytes32 book, uint32 index) = winningSlot(5, keccak256("pay"));
        OperatorPool.Ticket memory t = ticket(OTHER_KEY, book, index, keccak256("mailbox"), allUnits(), 0);
        vm.expectRevert(OperatorPool.BadStamp.selector);
        claimAsNode(0, one(t));
        // A valid signature over other fields.
        t = ticketFor(0, book, index);
        t.mailbox = keccak256("elsewhere");
        vm.expectRevert(OperatorPool.BadStamp.selector);
        claimAsNode(0, one(t));
        t = ticketFor(0, book, SIZE);
        vm.expectRevert(OperatorPool.BadStamp.selector);
        claimAsNode(0, one(t));
    }

    function testAStampOfABookThisShopNeverSoldPaysNothing() public {
        vm.expectRevert(OperatorPool.UnknownBook.selector);
        claimAsNode(0, one(ticketFor(0, keccak256("unsold"), 0)));
    }

    function testALosingStampPaysNothing() public {
        (bytes32 book,) = winningSlot(5, keccak256("pay"));
        OperatorPool.Ticket memory t = ticketFor(0, book, loser(book));
        vm.expectRevert(OperatorPool.NotWinning.selector);
        claimAsNode(0, one(t));
    }

    /// A book wins by the seed of its own purchase day, fixed after that day:
    /// a later seed neither reveals nor changes it.
    function testEachBookWinsByTheSeedOfItsOwnPurchaseDay() public {
        (bytes32 first, uint32 index) = winningSlot(5, keccak256("pay"));
        uint32 lost = loser(first);
        bytes32 second = buyUsdc();
        (bool known,) = pool.wins(second, 0);
        require(!known, "a book of today known");
        OperatorPool.Ticket memory early = ticketFor(0, second, 0);
        vm.expectRevert(OperatorPool.SeedUnknown.selector);
        claimAsNode(0, one(early));
        settle(DAY + 1, keccak256("next"));
        (bool stillKnown, bool stillWon) = pool.wins(first, index);
        (, bool nowWon) = pool.wins(first, lost);
        require(stillKnown && stillWon && !nowWon, "a later seed changed an earlier book");
        require(pool.score(pool.seedOf(DAY), pool.ticketId(first, index)) < pool.winThreshold(), "winner's score");
        require(pool.score(pool.seedOf(DAY), pool.ticketId(first, lost)) >= pool.winThreshold(), "loser's score");
        (known,) = pool.wins(second, 0);
        require(known, "the next day's seed");
    }

    /// In a network of four units a stamp names four; the rest of its list is empty.
    function testAStampToFewerThanTenHoldersPaysTheNamedOnes() public {
        (bytes32 book, uint32 index) = winningSlot(5, keccak256("pay"));
        bytes32[10] memory holders;
        for (uint256 k; k < 4; ++k) {
            holders[k] = units[k];
        }
        OperatorPool.Ticket memory t = ticket(BOOK_KEY, book, index, keccak256("mailbox"), holders, 2);
        require(claimAsNode(2, one(t)) == PRIZE, "unit 2 credited");
        t = ticket(BOOK_KEY, book, index, keccak256("mailbox"), holders, 3);
        require(claimAsNode(3, one(t)) == PRIZE, "unit 3 credited");
        require(usdc.balanceOf(owner(2)) == PRIZE && usdc.balanceOf(owner(3)) == PRIZE, "paid");
    }

    /// About one stamp in 1111 wins, and the prize of $0.10 was chosen so that a
    /// withdrawal costs about 1%: each ticket a first holder withdraws adds at most
    /// 50 thousand gas, and a withdrawal of one costs at most 140 thousand.
    function testAboutOneStampIn1111WinsAndATicketCostsAtMostFiftyThousandGas() public {
        bytes32[] memory books = new bytes32[](20);
        for (uint256 b; b < 20; ++b) {
            books[b] = buyUsdc();
        }
        settle(DAY, keccak256("odds"));
        (bytes32[] memory book, uint32[] memory index) = winners(books);
        // 20000 slots at 1 in 1111: 18 expected.
        require(book.length >= 10 && book.length <= 30, "winning slots");
        // Unit 1's withdrawal takes the pool's share from the splitter.
        claimAsNode(1, one(ticketFor(1, book[0], index[0])));
        uint256 single = claimGas(0, book, index, 1, 1);
        uint256 nine = claimGas(2, book, index, 2, 9);
        require(single <= 140_000, "a withdrawal of one ticket");
        require((nine - single) / 8 <= 50_000, "gas a ticket");
        require(usdc.balanceOf(owner(0)) == PRIZE && usdc.balanceOf(owner(2)) == 9 * PRIZE, "paid");
    }

    /// Gas of unit `k`'s node claiming `count` winning slots from `from` on.
    function claimGas(uint256 k, bytes32[] memory book, uint32[] memory index, uint256 from, uint256 count)
        internal
        returns (uint256 used)
    {
        OperatorPool.Ticket[] memory list = new OperatorPool.Ticket[](count);
        for (uint256 i; i < count; ++i) {
            list[i] = ticketFor(k, book[from + i], index[from + i]);
        }
        bytes memory data = abi.encodeCall(OperatorPool.claim, (uint32(k), transport(k), list));
        vm.prank(vm.addr(receiptKey(k)));
        uint256 before = gasleft();
        (bool called,) = address(pool).call(data);
        used = before - gasleft();
        require(called, "claimed");
    }

    function testATicketIsPaidOncePerHolder() public {
        (bytes32 book, uint32 index) = winningSlot(5, keccak256("pay"));
        claimAsNode(1, one(ticketFor(1, book, index)));
        vm.expectRevert(OperatorPool.AlreadyPaid.selector);
        claimAsNode(1, one(ticketFor(1, book, index)));
        // The same ticket twice in one withdrawal.
        OperatorPool.Ticket[] memory twice = new OperatorPool.Ticket[](2);
        twice[0] = ticketFor(2, book, index);
        twice[1] = twice[0];
        vm.expectRevert(OperatorPool.AlreadyPaid.selector);
        claimAsNode(2, twice);
        require(usdc.balanceOf(owner(1)) == PRIZE && usdc.balanceOf(owner(2)) == 0, "paid");
    }

    /// A book key that signs one slot for two messages does not get two sets
    /// of prizes: the first withdrawal fixes the slot's message.
    function testASlotSpentTwicePaysOnlyTheHoldersOfTheFirstMessageWithdrawn() public {
        (bytes32 book, uint32 index) = winningSlot(5, keccak256("pay"));
        claimAsNode(0, one(ticketFor(0, book, index)));
        OperatorPool.Ticket memory other = ticket(BOOK_KEY, book, index, keccak256("second"), allUnits(), 1);
        vm.expectRevert(OperatorPool.OtherOperation.selector);
        claimAsNode(1, one(other));
        require(claimAsNode(1, one(ticketFor(1, book, index))) == PRIZE, "the first message's holder");
    }

    function testATicketExpires360DaysAfterItsBookWasBought() public {
        uint64 bought = uint64(block.timestamp);
        (bytes32 book, uint32 index) = winningSlot(5, keccak256("pay"));
        vm.warp(bought + LIFETIME - 1);
        claimAsNode(0, one(ticketFor(0, book, index)));
        vm.warp(bought + LIFETIME);
        vm.expectRevert(OperatorPool.TicketExpired.selector);
        claimAsNode(1, one(ticketFor(1, book, index)));
    }

    // ---------------------------------------------------------------- seeds

    function testADaysSeedIsFixedOnlyAfterTheDayEnds() public {
        vm.expectRevert(OperatorPool.DayNotOver.selector);
        pool.armSeed(DAY);
        vm.warp((DAY + 1) * 1 days - 1);
        vm.expectRevert(OperatorPool.DayNotOver.selector);
        pool.armSeed(DAY);
        vm.warp((DAY + 1) * 1 days);
        require(pool.armSeed(DAY) == block.number + SEED_DELAY, "armed");
    }

    function testTheSeedComesFromABlockMinedAfterArming() public {
        vm.warp((DAY + 1) * 1 days);
        vm.expectRevert(OperatorPool.SeedNotArmed.selector);
        pool.captureSeed(DAY);
        uint64 target = pool.armSeed(DAY);
        vm.expectRevert(OperatorPool.SeedPending.selector);
        pool.armSeed(DAY);
        vm.roll(target);
        vm.expectRevert(OperatorPool.SeedNotReady.selector);
        pool.captureSeed(DAY);
        vm.roll(target + 1);
        vm.setBlockhash(target, keccak256("a"));
        bytes32 seed = pool.captureSeed(DAY);
        require(seed != 0 && pool.seedOf(DAY) == seed, "captured");
        vm.expectRevert(OperatorPool.SeedKnown.selector);
        pool.captureSeed(DAY);
        vm.expectRevert(OperatorPool.SeedKnown.selector);
        pool.armSeed(DAY);
        require(pool.seedOf(DAY) == seed, "a known seed changed");
    }

    function testAMissedCaptureWindowLetsTheDayBeArmedAgain() public {
        vm.warp((DAY + 1) * 1 days);
        uint64 target = pool.armSeed(DAY);
        vm.roll(target + 257);
        vm.expectRevert(OperatorPool.SeedLapsed.selector);
        pool.captureSeed(DAY);
        uint64 again = pool.armSeed(DAY);
        require(again == block.number + SEED_DELAY, "armed again");
        vm.roll(again + 1);
        vm.setBlockhash(again, keccak256("again"));
        require(pool.captureSeed(DAY) != 0, "captured");
    }

    // -------------------------------------------------------------- currency

    function testWhenUsdcRunsOutEthPaysTheRestAtTheFeedsRate() public {
        bytes32[] memory books = new bytes32[](5);
        for (uint256 b; b < 5; ++b) {
            books[b] = buyEth();
        }
        settle(DAY, keccak256("eth"));
        (bytes32[] memory book, uint32[] memory index) = winners(books);
        require(book.length > 0, "a winning slot");
        usdc.mint(address(pool), 40_000);
        // $2000 an ETH now: $0.06 is 0.00003 ETH.
        feed.set(2_000e8, block.timestamp);
        uint256 before = owner(6).balance;
        claimAsNode(6, one(ticketFor(6, book[0], index[0])));
        require(usdc.balanceOf(owner(6)) == 40_000, "USDC first");
        require(owner(6).balance - before == 30_000_000_000_000, "the rest in ETH");
        require(pool.owed(6) == 0, "owed");
    }

    function testWithoutAFreshRateTheEthPartStaysOwedUntilPaidOut() public {
        bytes32[] memory books = new bytes32[](5);
        for (uint256 b; b < 5; ++b) {
            books[b] = buyEth();
        }
        settle(DAY, keccak256("eth"));
        (bytes32[] memory book, uint32[] memory index) = winners(books);
        require(book.length > 0, "a winning slot");
        feed.set(RATE, block.timestamp - MAX_AGE - 1);
        uint256 before = owner(6).balance;
        require(claimAsNode(6, one(ticketFor(6, book[0], index[0]))) == PRIZE, "credited");
        require(owner(6).balance == before && pool.owed(6) == PRIZE && pool.totalOwed() == PRIZE, "owed");
        // Nobody but the node or its owner pays it out.
        vm.expectRevert(OperatorPool.Unauthorized.selector);
        pool.payOut(6, transport(6));
        feed.set(RATE, block.timestamp);
        vm.prank(vm.addr(receiptKey(6)));
        require(pool.payOut(6, transport(6)) == PRIZE, "paid out");
        require(owner(6).balance - before == 40_000_000_000_000, "in ETH at $2500");
        require(pool.owed(6) == 0 && pool.totalOwed() == 0, "still owed");
        vm.prank(owner(6));
        require(pool.payOut(6, bytes32(0)) == 0, "paid twice");
    }

    // -------------------------------------------------------------- treasury

    function testTheTreasuryTakesWhatHoldersDidNotWithdrawOnceTheTicketsExpire() public {
        (bytes32 book, uint32 index) = winningSlot(3, keccak256("sweep"));
        claimAsNode(0, one(ticketFor(0, book, index)));
        // A sale of the next day, whose tickets live a day longer.
        buyUsdc();
        vm.expectRevert(OperatorPool.NotTreasury.selector);
        pool.sweep(1000);
        uint256 expired = (DAY + 1) * 1 days + LIFETIME;
        vm.warp(expired - 1);
        vm.prank(treasury);
        require(pool.sweep(1000) == 0 && usdc.balanceOf(treasury) == 0, "swept before expiry");
        vm.warp(expired);
        vm.prank(treasury);
        require(pool.sweep(1000) == 3 * SHARE_PER_BOOK - PRIZE, "swept");
        require(usdc.balanceOf(treasury) == 3 * SHARE_PER_BOOK - PRIZE, "treasury paid");
        require(poolUsdc() == SHARE_PER_BOOK, "the next day's share kept");
        vm.expectRevert(OperatorPool.TicketExpired.selector);
        claimAsNode(1, one(ticketFor(1, book, index)));
        vm.prank(treasury);
        require(pool.sweep(1000) == 0, "swept twice");
        vm.warp(expired + 1 days);
        vm.prank(treasury);
        require(pool.sweep(1000) == SHARE_PER_BOOK, "the next day");
    }

    /// A day whose winners took more than its share is netted against the
    /// next: the treasury does not keep the good luck and leave the bad.
    function testLuckyDaysOffsetUnluckyOnes() public {
        (bytes32 book, uint32 index) = winningSlot(1, keccak256("lucky"));
        for (uint256 k; k < 10; ++k) {
            claimAsNode(k, one(ticketFor(k, book, index)));
        }
        // Ten prizes of one winning slot are more than its book's share: the
        // tenth holder's stays owed.
        require(pool.paidOn(DAY) == 10 * PRIZE, "credited on the lucky day");
        require(pool.owed(9) == PRIZE && usdc.balanceOf(owner(9)) == 0, "the tenth owed");
        // The next day nobody wins; the day after is not expired at the sweep.
        buyUsdc();
        vm.warp((DAY + 2) * 1 days);
        buyUsdc();
        vm.warp((DAY + 2) * 1 days + LIFETIME);
        // One day a call: the lucky day's overdraft carries into the next.
        vm.prank(treasury);
        require(pool.sweep(1) == 0, "the lucky day");
        vm.prank(treasury);
        require(pool.sweep(1) == 2 * SHARE_PER_BOOK - 10 * PRIZE, "netted");
        vm.prank(owner(9));
        require(pool.payOut(9, bytes32(0)) == PRIZE, "the tenth paid");
        require(poolUsdc() == SHARE_PER_BOOK, "the last day's share kept");
    }

    function testTheTreasuryNeverTakesWhatIsOwedToAUnit() public {
        bytes32[] memory books = new bytes32[](3);
        for (uint256 b; b < 3; ++b) {
            books[b] = buyEth();
        }
        settle(DAY, keccak256("owed"));
        (bytes32[] memory book, uint32[] memory index) = winners(books);
        require(book.length > 0, "a winning slot");
        feed.set(RATE, block.timestamp - MAX_AGE - 1);
        claimAsNode(8, one(ticketFor(8, book[0], index[0])));
        require(pool.owed(8) == PRIZE, "owed");
        // ETH fell to $2000: the pool's 0.00108 ETH is $2.16, of which $0.10 is owed.
        vm.warp((DAY + 1) * 1 days + LIFETIME);
        feed.set(2_000e8, block.timestamp);
        vm.prank(treasury);
        require(pool.sweep(1000) == 2_060_000, "swept what is not owed");
        require(treasury.balance == 1_030_000_000_000_000, "treasury paid in ETH");
        uint256 before = owner(8).balance;
        vm.prank(owner(8));
        require(pool.payOut(8, bytes32(0)) == PRIZE, "owed paid out");
        require(owner(8).balance - before == 50_000_000_000_000, "in ETH at $2000");
        // What the fall of ETH left unpaid is not taken from a later day's sale.
        buyEth();
        vm.prank(treasury);
        require(pool.sweep(1000) == 0, "the shortfall taken later");
        require(address(pool).balance + splitter.credits(address(pool)) == POOL_WEI_PER_BOOK, "the later day's share");
    }

    function testASweepWalksAtMostTheDaysAskedFor() public {
        buyUsdc();
        vm.warp((DAY + 1) * 1 days);
        buyUsdc();
        vm.warp((DAY + 2) * 1 days + LIFETIME);
        vm.prank(treasury);
        require(pool.sweep(1) == SHARE_PER_BOOK, "first day");
        vm.prank(treasury);
        require(pool.sweep(1) == SHARE_PER_BOOK, "second day");
        vm.prank(treasury);
        require(pool.sweep(1) == 0, "nothing left");
    }

    // ---------------------------------------------------------- deployment

    function testAPoolTheSplitterDoesNotPayIsRefused() public {
        RoyaltySplitter elsewhere = newSplitter(treasury, address(0x9001));
        BookShop other =
            new BookShop(DOMAIN, PRICE_USDC, SIZE, VALIDITY, elsewhere, address(usdc), address(feed), MAX_AGE);
        vm.expectRevert(OperatorPool.InvalidConfiguration.selector);
        new OperatorPool(other, registry, treasury, PRIZE, LIFETIME, SEED_DELAY, DAY);
    }

    function testTermsMustBePayableAndComplete() public {
        // Ten prizes of $0.00009 are the whole share: every stamp would win.
        expectRefusedPool(treasury, 90, LIFETIME, SEED_DELAY, DAY);
        expectRefusedPool(treasury, PRIZE, 0, SEED_DELAY, DAY);
        expectRefusedPool(treasury, PRIZE, LIFETIME, 0, DAY);
        expectRefusedPool(treasury, PRIZE, LIFETIME, SEED_DELAY, DAY + 1);
        expectRefusedPool(address(0), PRIZE, LIFETIME, SEED_DELAY, DAY);
    }

    function expectRefusedPool(address treasury_, uint256 prize, uint64 lifetime, uint16 delay, uint64 firstDay)
        internal
    {
        address predicted = vm.computeCreateAddress(address(this), vm.getNonce(address(this)) + 2);
        RoyaltySplitter s = newSplitter(treasury_ == address(0) ? treasury : treasury_, predicted);
        BookShop b = new BookShop(DOMAIN, PRICE_USDC, SIZE, VALIDITY, s, address(usdc), address(feed), MAX_AGE);
        vm.expectRevert(OperatorPool.InvalidConfiguration.selector);
        new OperatorPool(b, registry, treasury_, prize, lifetime, delay, firstDay);
    }

    /// The node builds operations, slots and claims with these digests; the
    /// values were computed with `cast keccak`, apart from Solidity.
    function testTheDigestsMatchThePinnedVectors() public view {
        bytes32[10] memory holders;
        for (uint256 i; i < 10; ++i) {
            holders[i] = bytes32(i + 1);
        }
        require(
            pool.swarmDigest(holders) == 0xc4c4ec6d96bf24101619bb4ed2d644915cadd263f9b8e274df175130ac9f7ee9, "swarm"
        );
        require(
            pool.operation(
                bytes32(0x1111111111111111111111111111111111111111111111111111111111111111),
                PERIOD,
                holders,
                keccak256("envelope")
            ) == 0x38e95e7e9e9d39732b91626f16f474f110ff67c9c3102f4e72d269bdcde7e085,
            "operation"
        );
        require(
            pool.ticketId(0x31165017cd3777e6f74336534ea20d41b9b3df2cdc92685eb57cca728191fa21, 7)
                == 0xd45234d39174f6f5ea7e37e04b6774afcf35a17f1134a38111268845294e3d46,
            "ticket"
        );
        require(
            pool.score(
                0x66a80b61b29ec044d14c4c8c613e762ba1fb8eeb0c454d1ee00ed6dedaa5b5c5,
                0xd45234d39174f6f5ea7e37e04b6774afcf35a17f1134a38111268845294e3d46
            ) == 0x35807ea272f633c31d9709e4ab801142fe20e5df6d90ba15f940734b729b8f77,
            "win score"
        );
    }

    /// The node calls these selectors (`crates/node/src/chain.rs`; its claim
    /// encoding is pinned there against `cast calldata`).
    function testTheNodeCallsTheseSelectors() public view {
        require(OperatorPool.claim.selector == bytes4(0xe40593b5), "claim");
        require(OperatorPool.payOut.selector == bytes4(0xf6a08139), "payOut");
        require(OperatorPool.armSeed.selector == bytes4(0xc9bc7a8c), "armSeed");
        require(OperatorPool.captureSeed.selector == bytes4(0x0628829b), "captureSeed");
        require(pool.seeds.selector == bytes4(0xc2f1c307), "seeds");
        require(pool.winThreshold.selector == bytes4(0xed96cc51), "winThreshold");
        require(pool.prizeUsdc.selector == bytes4(0xf6a18664), "prizeUsdc");
        require(pool.ticketLifetime.selector == bytes4(0x5bbb8b82), "ticketLifetime");
        require(pool.owed.selector == bytes4(0x239118ce), "owed");
        require(NodeRegistry.unit.selector == bytes4(0xa43ec96e), "unit");
        require(registry.nextIndex.selector == bytes4(0xfc7e9c6f), "nextIndex");
    }

    receive() external payable {}
}
