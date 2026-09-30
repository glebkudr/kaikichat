// SPDX-License-Identifier: MIT
pragma solidity 0.8.36;

import {BookShop, PriceFeed} from "./BookShop.sol";
import {NodeRegistry} from "./NodeRegistry.sol";
import {RoyaltySplitter} from "./RoyaltySplitter.sol";

/// @notice Pays node operators for the paid messages they hold. A paid stamp is a
/// ticket: it wins with odds that make `SWARM` prizes worth the pool's share of the
/// stamp, and each holder the stamp names withdraws its prize once, until the ticket
/// expires. What nobody withdrew by then goes to the treasury.
/// @dev No owner and no upgrade. A book wins by the seed of its purchase day: a block
/// hash fixed after that day ends, so nobody knows a book's winning slots when buying
/// it. Only the holder's node or the unit's owner starts a withdrawal; the unit's owner
/// is paid, in USDC first and then in ETH at the shop's feed rate.
contract OperatorPool {
    error InvalidConfiguration();
    error WrongChain();
    error Unauthorized();
    error UnknownBook();
    error NotHolder();
    error BadStamp();
    error TicketExpired();
    error SeedUnknown();
    error NotWinning();
    error OtherOperation();
    error AlreadyPaid();
    error DayNotOver();
    error SeedPending();
    error SeedNotArmed();
    error SeedNotReady();
    error SeedLapsed();
    error SeedKnown();
    error NotTreasury();
    error StalePrice();
    error TransferFailed();

    /// A paid stamp as its holder claims it.
    struct Ticket {
        bytes32 book;
        uint32 index;
        bytes32 mailbox;
        uint64 period;
        /// keccak256 of the envelope bytes.
        bytes32 envelope;
        /// The units the sender addressed, best first; zero past the last.
        bytes32[10] holders;
        /// The claimant's place in `holders`.
        uint8 position;
        uint8 v;
        bytes32 r;
        bytes32 s;
    }

    /// A day's seed: the hash of block `target`, once captured.
    struct Seed {
        uint64 target;
        bytes32 hash;
    }

    event SeedArmed(uint64 indexed day, uint64 target);
    event SeedCaptured(uint64 indexed day, bytes32 seed);
    event Claimed(uint32 indexed unit, uint256 tickets, uint256 credited);
    event PaidOut(uint32 indexed unit, address indexed to, uint256 usdc, uint256 weiAmount);
    event Swept(uint64 nextDay, uint256 usdc, uint256 weiAmount);

    /// Holders of a mailbox, as `agentic_mailbox_swarm::select::SWARM_SIZE`.
    uint256 public constant SWARM = 10;
    uint64 private constant DAY = 1 days;
    uint256 private constant BPS = 10_000;
    /// A blockhash is readable for 256 blocks.
    uint256 private constant HASH_WINDOW = 256;
    /// The low bits of a slot's record: a bit per paid place.
    uint256 private constant PLACES = (1 << SWARM) - 1;
    bytes32 private constant UNIT_TAG = keccak256("AIN_UNIT_V1");
    bytes32 private constant SWARM_TAG = keccak256("AIN_SWARM_V1");
    bytes32 private constant OPERATION_TAG = keccak256("AIN_OPERATION_V2");
    bytes32 private constant STAMP_TAG = keccak256("AIN_STAMP_V1");
    bytes32 private constant TICKET_TAG = keccak256("AIN_TICKET_V1");
    bytes32 private constant WIN_TAG = keccak256("AIN_WIN_V1");

    BookShop public immutable shop;
    NodeRegistry public immutable registry;
    RoyaltySplitter public immutable splitter;
    address public immutable treasury;
    address public immutable usdc;
    PriceFeed public immutable ethUsd;
    bytes32 public immutable domain;
    uint256 public immutable deploymentChainId;
    /// A holder's prize, in USDC units.
    uint256 public immutable prizeUsdc;
    uint64 public immutable ticketLifetime;
    uint16 public immutable seedDelay;
    /// The first purchase day a sweep settles.
    uint64 public immutable firstDay;
    /// A slot wins when its score is below this.
    uint256 public immutable winThreshold;
    /// The pool's share of one book, in USDC units.
    uint256 public immutable bookShareUsdc;
    uint64 private immutable validity;
    uint32 private immutable maxPriceAge;
    /// Wei per USDC unit at a rate of one: 10^(18 - token decimals + feed decimals).
    uint256 private immutable weiScale;

    mapping(uint64 day => Seed) public seeds;
    /// Per slot: the high bits of the operation first paid and a bit per paid place.
    mapping(bytes32 ticket => uint256) public paidPlaces;
    /// Prizes credited, by the purchase day of their books.
    mapping(uint64 day => uint256) public paidOn;
    /// Prizes credited to a unit and not paid yet, in USDC units.
    mapping(uint32 unit => uint256) public owed;
    uint256 public totalOwed;
    /// The next purchase day a sweep settles.
    uint64 public sweptTo;
    /// Settled days' shares less their prizes: an overdraft carries on.
    int256 public sweepBalance;

    constructor(
        BookShop shop_,
        NodeRegistry registry_,
        address treasury_,
        uint256 prizeUsdc_,
        uint64 ticketLifetime_,
        uint16 seedDelay_,
        uint64 firstDay_
    ) {
        if (
            block.chainid == 0 || address(registry_) == address(0) || treasury_ == address(0) || prizeUsdc_ == 0
                || ticketLifetime_ == 0 || seedDelay_ == 0 || seedDelay_ > HASH_WINDOW / 2
                || firstDay_ > block.timestamp / DAY
        ) revert InvalidConfiguration();
        RoyaltySplitter splitter_ = shop_.splitter();
        uint256 share;
        for (uint256 i; i < splitter_.recipientCount(); ++i) {
            (address account, uint16 bps) = splitter_.recipient(i);
            if (account == address(this)) share = bps;
        }
        uint256 price = shop_.priceUsdc();
        // Odds below one: `SWARM` prizes are worth more than a stamp's share.
        uint256 odds = prizeUsdc_ * SWARM * shop_.bookSize() * BPS;
        if (share == 0 || price * share >= odds) revert InvalidConfiguration();
        address usdc_ = shop_.usdc();
        PriceFeed feed = shop_.ethUsd();
        shop = shop_;
        registry = registry_;
        splitter = splitter_;
        treasury = treasury_;
        usdc = usdc_;
        ethUsd = feed;
        domain = shop_.domain();
        deploymentChainId = block.chainid;
        prizeUsdc = prizeUsdc_;
        ticketLifetime = ticketLifetime_;
        seedDelay = seedDelay_;
        firstDay = firstDay_;
        sweptTo = firstDay_;
        winThreshold = (type(uint256).max / odds) * (price * share);
        bookShareUsdc = price * share / BPS;
        validity = shop_.validity();
        maxPriceAge = shop_.maxPriceAge();
        weiScale = 10 ** (18 - PriceFeed(usdc_).decimals() + feed.decimals());
    }

    modifier sameChain() {
        if (block.chainid != deploymentChainId) revert WrongChain();
        _;
    }

    /// The splitter pays the pool's share here.
    receive() external payable {}

    // ------------------------------------------------------------------ digests

    /// What a stamp commits to about the units it addresses.
    function swarmDigest(bytes32[10] memory holders) public pure returns (bytes32) {
        return keccak256(abi.encodePacked(SWARM_TAG, holders));
    }

    /// What a stamp of a mailbox message pays for: these envelope bytes, to this
    /// mailbox, held by these units.
    function operation(bytes32 mailbox, uint64 period, bytes32[10] memory holders, bytes32 envelope)
        public
        pure
        returns (bytes32)
    {
        return operationDigest(mailbox, period, swarmDigest(holders), envelope);
    }

    /// The slot a stamp spends.
    function ticketId(bytes32 book, uint32 index) public view returns (bytes32 digest) {
        bytes32 tag = TICKET_TAG;
        bytes32 domain_ = domain;
        assembly ("memory-safe") {
            let p := mload(0x40)
            mstore(p, tag)
            mstore(add(p, 0x20), domain_)
            mstore(add(p, 0x40), book)
            mstore(add(p, 0x60), shl(224, index))
            digest := keccak256(p, 0x64)
        }
    }

    /// A slot's draw under a seed; it wins below `winThreshold`.
    function score(bytes32 seed, bytes32 ticket) public pure returns (uint256 draw) {
        bytes32 tag = WIN_TAG;
        assembly ("memory-safe") {
            let p := mload(0x40)
            mstore(p, tag)
            mstore(add(p, 0x20), seed)
            mstore(add(p, 0x40), ticket)
            draw := keccak256(p, 0x60)
        }
    }

    // The digests below hash in scratch memory past the free pointer, so a claim
    // of many tickets does not grow its memory ticket by ticket.

    function operationDigest(bytes32 mailbox, uint64 period, bytes32 swarm, bytes32 envelope)
        private
        pure
        returns (bytes32 digest)
    {
        bytes32 tag = OPERATION_TAG;
        assembly ("memory-safe") {
            let p := mload(0x40)
            mstore(p, tag)
            mstore(add(p, 0x20), mailbox)
            mstore(add(p, 0x40), shl(192, period))
            mstore(add(p, 0x48), swarm)
            mstore(add(p, 0x68), envelope)
            digest := keccak256(p, 0x88)
        }
    }

    /// `swarmDigest` of a ticket's holders, read straight from calldata.
    function swarmOf(Ticket calldata t) private pure returns (bytes32 digest) {
        bytes32 tag = SWARM_TAG;
        assembly ("memory-safe") {
            let p := mload(0x40)
            mstore(p, tag)
            // `holders` follows book, index, mailbox, period and envelope.
            calldatacopy(add(p, 0x20), add(t, 0xa0), 0x140)
            digest := keccak256(p, 0x160)
        }
    }

    function stampDigest(bytes32 book, uint32 index, bytes32 op) private view returns (bytes32 digest) {
        bytes32 tag = STAMP_TAG;
        bytes32 domain_ = domain;
        assembly ("memory-safe") {
            let p := mload(0x40)
            mstore(p, tag)
            mstore(add(p, 0x20), domain_)
            mstore(add(p, 0x40), book)
            mstore(add(p, 0x60), shl(224, index))
            mstore(add(p, 0x64), op)
            digest := keccak256(p, 0x84)
        }
    }

    function seedOf(uint64 day) external view returns (bytes32) {
        return seeds[day].hash;
    }

    /// Whether the seed of the book's purchase day is known, and whether the slot won.
    function wins(bytes32 book, uint32 index) external view returns (bool known, bool won) {
        (address key,, uint64 validUntil) = shop.books(book);
        if (key == address(0)) return (false, false);
        bytes32 seed = seeds[uint64((validUntil - validity) / DAY)].hash;
        if (seed == 0) return (false, false);
        return (true, score(seed, ticketId(book, index)) < winThreshold);
    }

    // -------------------------------------------------------------------- seeds

    /// After `day` ends, names the block whose hash becomes its seed; again once
    /// that block's hash is no longer readable.
    function armSeed(uint64 day) external sameChain returns (uint64 target) {
        if (block.timestamp < (uint256(day) + 1) * DAY) revert DayNotOver();
        Seed storage seed = seeds[day];
        if (seed.hash != 0) revert SeedKnown();
        if (seed.target != 0 && block.number <= uint256(seed.target) + HASH_WINDOW) revert SeedPending();
        target = uint64(block.number) + seedDelay;
        seed.target = target;
        emit SeedArmed(day, target);
    }

    /// Records the target block's hash as the day's seed.
    function captureSeed(uint64 day) external sameChain returns (bytes32 hash) {
        Seed storage seed = seeds[day];
        if (seed.hash != 0) revert SeedKnown();
        if (seed.target == 0) revert SeedNotArmed();
        if (block.number <= seed.target) revert SeedNotReady();
        hash = blockhash(seed.target);
        if (block.number - seed.target > HASH_WINDOW || hash == 0) revert SeedLapsed();
        seed.hash = hash;
        emit SeedCaptured(day, hash);
    }

    // -------------------------------------------------------------- withdrawals

    /// Credits unit `unitIndex` with the prizes of `tickets` and pays its owner what
    /// the pool can. The unit's node (its receipt account, with its transport key) or
    /// the unit's owner calls it.
    function claim(uint32 unitIndex, bytes32 transportKey, Ticket[] calldata tickets)
        external
        sameChain
        returns (uint256 credited)
    {
        NodeRegistry.Unit memory unit = authorize(unitIndex, transportKey);
        for (uint256 i; i < tickets.length; ++i) {
            redeem(tickets[i], unit.commitment);
        }
        credited = tickets.length * prizeUsdc;
        emit Claimed(unitIndex, tickets.length, credited);
        pay(unitIndex, unit.owner, credited);
    }

    /// Pays the unit's owner what is owed to the unit, as far as the pool can.
    function payOut(uint32 unitIndex, bytes32 transportKey) external sameChain returns (uint256 paid) {
        NodeRegistry.Unit memory unit = authorize(unitIndex, transportKey);
        return pay(unitIndex, unit.owner, 0);
    }

    function authorize(uint32 unitIndex, bytes32 transportKey) private view returns (NodeRegistry.Unit memory unit) {
        unit = registry.unit(unitIndex);
        if (unit.state == NodeRegistry.UnitState.None) revert Unauthorized();
        if (msg.sender == unit.owner) return unit;
        if (keccak256(abi.encodePacked(UNIT_TAG, domain, transportKey, msg.sender)) != unit.commitment) {
            revert Unauthorized();
        }
    }

    function redeem(Ticket calldata t, bytes32 own) private {
        (address key, uint32 count, uint64 validUntil) = shop.books(t.book);
        if (key == address(0)) revert UnknownBook();
        if (t.position >= SWARM || t.holders[t.position] != own) revert NotHolder();
        for (uint256 j; j < SWARM; ++j) {
            if (j != t.position && t.holders[j] == own) revert NotHolder();
        }
        bytes32 op = operationDigest(t.mailbox, t.period, swarmOf(t), t.envelope);
        if (t.index >= count || ecrecover(stampDigest(t.book, t.index, op), t.v, t.r, t.s) != key) revert BadStamp();
        uint256 bought = validUntil - validity;
        if (block.timestamp >= bought + ticketLifetime) revert TicketExpired();
        markPaid(t.book, t.index, uint64(bought / DAY), op, t.position);
    }

    /// Records the place's prize if the slot won and the place is unpaid; the first
    /// paid operation of a slot is the only one paid.
    function markPaid(bytes32 book, uint32 index, uint64 day, bytes32 op, uint8 position) private {
        bytes32 seed = seeds[day].hash;
        if (seed == 0) revert SeedUnknown();
        bytes32 slot = ticketId(book, index);
        if (score(seed, slot) >= winThreshold) revert NotWinning();
        uint256 places = paidPlaces[slot];
        uint256 pinned = uint256(op) & ~PLACES;
        if (places == 0) places = pinned;
        else if (places & ~PLACES != pinned) revert OtherOperation();
        uint256 place = uint256(1) << position;
        if (places & place != 0) revert AlreadyPaid();
        paidPlaces[slot] = places | place;
        paidOn[day] += prizeUsdc;
    }

    /// Pays the unit's owner what was owed to the unit and `credited`, as far as the
    /// pool can; the rest stays owed. Only an unpaid rest is written.
    function pay(uint32 unitIndex, address to, uint256 credited) private returns (uint256 value) {
        uint256 before = owed[unitIndex];
        uint256 due = before + credited;
        if (due == 0) return 0;
        uint256 inTokens;
        uint256 inWei;
        (value, inTokens, inWei) = plan(due, 0, false);
        if (due - value != before) owed[unitIndex] = due - value;
        if (value != credited) totalOwed = totalOwed + credited - value;
        if (value == 0) return 0;
        send(to, inTokens, inWei);
        emit PaidOut(unitIndex, to, inTokens, inWei);
    }

    // ---------------------------------------------------------------- treasury

    /// Settles up to `maxDays` purchase days whose tickets have all expired and pays
    /// the treasury their shares less their prizes, netted across days, out of what
    /// the pool holds beyond the amounts owed to units. What the pool cannot pay is
    /// not carried to later days; an overdraft is.
    function sweep(uint64 maxDays) external sameChain returns (uint256 value) {
        if (msg.sender != treasury) revert NotTreasury();
        uint64 day = sweptTo;
        int256 balance = sweepBalance;
        for (uint64 n; n < maxDays && (uint256(day) + 1) * DAY + ticketLifetime <= block.timestamp; ++n) {
            balance += int256(uint256(shop.soldOn(day)) * bookShareUsdc) - int256(paidOn[day]);
            ++day;
        }
        sweptTo = day;
        if (balance <= 0) {
            sweepBalance = balance;
            return 0;
        }
        uint256 inTokens;
        uint256 inWei;
        (value, inTokens, inWei) = plan(uint256(balance), totalOwed, true);
        sweepBalance = 0;
        send(treasury, inTokens, inWei);
        emit Swept(day, inTokens, inWei);
    }

    // ----------------------------------------------------------------- payments

    /// Takes the pool's credits from the splitter; a claim does it only when the
    /// pool's own USDC falls short.
    function pull() private {
        if (splitter.credits(address(this)) != 0) splitter.withdrawRoyalty(payable(address(this)));
        if (splitter.tokenCredits(usdc, address(this)) != 0) splitter.withdrawTokenRoyalty(usdc, address(this));
    }

    /// Up to `amount` (USDC units) of the pool's value beyond `keep`: USDC first,
    /// then ETH at the feed's rate. The rate is read only when USDC is short; without
    /// a fresh one the ETH does not count, or, when `strict`, the call is refused.
    function plan(uint256 amount, uint256 keep, bool strict)
        private
        returns (uint256 value, uint256 inTokens, uint256 inWei)
    {
        uint256 tokens = usdcBalance();
        if (tokens < amount + keep) {
            pull();
            tokens = usdcBalance();
        }
        uint256 rate;
        uint256 ethValue;
        if (tokens < amount + keep && address(this).balance != 0) {
            rate = freshRate();
            if (rate == 0 && strict) revert StalePrice();
            if (rate != 0) ethValue = address(this).balance * rate / weiScale;
        }
        uint256 total = tokens + ethValue;
        if (total <= keep) return (0, 0, 0);
        value = total - keep < amount ? total - keep : amount;
        inTokens = value < tokens ? value : tokens;
        if (value > inTokens) inWei = (value - inTokens) * weiScale / rate;
    }

    function send(address to, uint256 inTokens, uint256 inWei) private {
        if (inTokens != 0) {
            (bool called, bytes memory result) =
                usdc.call(abi.encodeWithSignature("transfer(address,uint256)", to, inTokens));
            if (!called || (result.length != 0 && (result.length != 32 || !abi.decode(result, (bool))))) {
                revert TransferFailed();
            }
        }
        if (inWei != 0) {
            (bool paid,) = payable(to).call{value: inWei}("");
            if (!paid) revert TransferFailed();
        }
    }

    function usdcBalance() private view returns (uint256) {
        (bool called, bytes memory result) =
            usdc.staticcall(abi.encodeWithSignature("balanceOf(address)", address(this)));
        if (!called || result.length != 32) revert TransferFailed();
        return abi.decode(result, (uint256));
    }

    /// The feed's ETH/USD rate if fresh and positive, else zero.
    function freshRate() private view returns (uint256) {
        try ethUsd.latestRoundData() returns (uint80, int256 answer, uint256, uint256 updatedAt, uint80) {
            if (answer <= 0 || updatedAt == 0 || updatedAt > block.timestamp) return 0;
            if (block.timestamp - updatedAt > maxPriceAge) return 0;
            return uint256(answer);
        } catch {
            return 0;
        }
    }
}
