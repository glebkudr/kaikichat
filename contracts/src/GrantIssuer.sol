// SPDX-License-Identifier: MIT
pragma solidity 0.8.36;

/// @notice Network rules for coins minted by the identity server: its hot keys and a
/// daily emission cap that the owner changes at most once per UTC day by at most 10x.
/// @dev The owner is a cold key. Issuer keys can only mint off-chain grants; they hold
/// no rights here. Anyone can end, from the next day, an issuer that signed two
/// grants for one slot; the owner can end an issuer from today.
contract GrantIssuer {
    error NotOwner();
    error NotPendingOwner();
    error InvalidConfiguration();
    error CapAlreadyChangedToday();
    error CapStepTooLarge();
    error InvalidIssuer();
    error IssuerAlreadyKnown();
    error IssuerNotActive();
    error StartTooEarly();
    error NotEquivocation();
    error InvalidSignature();

    /// A grant of this contract's domain as the issuer signed it.
    struct SignedGrant {
        address book;
        uint64 day;
        uint32 serial;
        uint32 count;
        uint64 expiry;
        bytes signature;
    }

    struct CapChange {
        uint64 effectiveDay;
        uint64 capCoins;
    }

    /// Active on days in [fromDay, untilDay).
    struct Window {
        uint64 fromDay;
        uint64 untilDay;
        bool known;
    }

    event DailyCapScheduled(uint64 indexed effectiveDay, uint64 capCoins);
    event IssuerAdded(address indexed issuer, uint64 fromDay);
    event IssuerRevoked(address indexed issuer, uint64 untilDay);
    event OwnershipTransferStarted(address indexed owner, address indexed pendingOwner);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    event EquivocationReported(address indexed issuer, uint64 day, uint32 serial, address reporter);

    uint64 private constant SECONDS_PER_DAY = 86_400;
    uint256 private constant MAX_STEP = 10;
    uint64 private constant OPEN = type(uint64).max;
    bytes32 private constant GRANT_TAG = keccak256("AIN_GRANT_V1");
    uint256 private constant HALF_ORDER = 0x7fffffffffffffffffffffffffffffff5d576e7357a4501ddfe92f46681b20a0;

    bytes32 public immutable domain;
    uint32 public immutable bookSize;
    uint64 public immutable maxValidityDays;
    address public owner;
    address public pendingOwner;
    uint64 public lastCapChangeDay;

    CapChange[] private caps;
    mapping(address => Window) private issuers;

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    constructor(
        address owner_,
        bytes32 domain_,
        uint32 bookSize_,
        uint64 maxValidityDays_,
        uint64 initialCapCoins,
        address firstIssuer
    ) {
        if (
            owner_ == address(0) || domain_ == bytes32(0) || bookSize_ == 0 || maxValidityDays_ == 0
                || initialCapCoins == 0 || firstIssuer == address(0)
        ) revert InvalidConfiguration();
        domain = domain_;
        bookSize = bookSize_;
        maxValidityDays = maxValidityDays_;
        owner = owner_;
        uint64 day = today();
        caps.push(CapChange(day, initialCapCoins));
        lastCapChangeDay = day;
        issuers[firstIssuer] = Window(day, OPEN, true);
        emit OwnershipTransferred(address(0), owner_);
        emit DailyCapScheduled(day, initialCapCoins);
        emit IssuerAdded(firstIssuer, day);
    }

    function today() public view returns (uint64) {
        return uint64(block.timestamp / SECONDS_PER_DAY);
    }

    /// The emission cap in coins for `day`; zero before deployment.
    function capForDay(uint64 day) external view returns (uint64) {
        for (uint256 i = caps.length; i > 0; --i) {
            if (caps[i - 1].effectiveDay <= day) return caps[i - 1].capCoins;
        }
        return 0;
    }

    /// Schedules a new cap from tomorrow; at most once per UTC day, within 1/10..10x.
    function setDailyCap(uint64 capCoins) external onlyOwner {
        uint64 day = today();
        if (day <= lastCapChangeDay) revert CapAlreadyChangedToday();
        uint256 current = caps[caps.length - 1].capCoins;
        if (capCoins == 0 || capCoins > current * MAX_STEP || uint256(capCoins) * MAX_STEP < current) {
            revert CapStepTooLarge();
        }
        caps.push(CapChange(day + 1, capCoins));
        lastCapChangeDay = day;
        emit DailyCapScheduled(day + 1, capCoins);
    }

    /// Registers a new issuer key, active from `fromDay` (tomorrow at the earliest).
    function addIssuer(address issuer, uint64 fromDay) external onlyOwner {
        if (issuer == address(0)) revert InvalidIssuer();
        if (issuers[issuer].known) revert IssuerAlreadyKnown();
        if (fromDay <= today()) revert StartTooEarly();
        issuers[issuer] = Window(fromDay, OPEN, true);
        emit IssuerAdded(issuer, fromDay);
    }

    /// Ends an issuer's window from today, permanently; also shortens a window
    /// that a report ended from tomorrow.
    function revokeIssuer(address issuer) external onlyOwner {
        endWindow(issuer, today());
    }

    function issuerActiveOn(address issuer, uint64 day) external view returns (bool) {
        Window memory window = issuers[issuer];
        return window.known && window.fromDay <= day && day < window.untilDay;
    }

    function transferOwnership(address newOwner) external onlyOwner {
        pendingOwner = newOwner;
        emit OwnershipTransferStarted(owner, newOwner);
    }

    function acceptOwnership() external {
        if (msg.sender != pendingOwner || msg.sender == address(0)) revert NotPendingOwner();
        emit OwnershipTransferred(owner, msg.sender);
        owner = msg.sender;
        pendingOwner = address(0);
    }

    /// Anyone may end an issuer from tomorrow by showing two different grants it
    /// signed for one day and serial of this domain. Grants already issued today
    /// stay valid, so an honest issuer's mistake costs at most the rest of a day.
    function reportEquivocation(address issuer, SignedGrant calldata first, SignedGrant calldata second) external {
        if (first.day != second.day || first.serial != second.serial) revert NotEquivocation();
        bytes32 a = grantDigest(domain, issuer, first.book, first.day, first.serial, first.count, first.expiry);
        bytes32 b = grantDigest(domain, issuer, second.book, second.day, second.serial, second.count, second.expiry);
        if (a == b) revert NotEquivocation();
        if (signer(a, first.signature) != issuer || signer(b, second.signature) != issuer) {
            revert InvalidSignature();
        }
        endWindow(issuer, today() + 1);
        emit EquivocationReported(issuer, first.day, first.serial, msg.sender);
    }

    function grantDigest(
        bytes32 domain_,
        address server,
        address book,
        uint64 day,
        uint32 serial,
        uint32 count,
        uint64 expiry
    ) public pure returns (bytes32) {
        return keccak256(abi.encodePacked(GRANT_TAG, domain_, server, book, day, serial, count, expiry));
    }

    /// Ends an issuer's window at `untilDay`; never reopens or extends it.
    function endWindow(address issuer, uint64 untilDay) private {
        Window storage window = issuers[issuer];
        if (!window.known || window.untilDay <= untilDay) revert IssuerNotActive();
        window.untilDay = untilDay;
        emit IssuerRevoked(issuer, untilDay);
    }

    /// The low-s signer of `digest`, or zero for malformed signatures.
    function signer(bytes32 digest, bytes calldata signature) private pure returns (address) {
        if (signature.length != 65) return address(0);
        bytes32 r = bytes32(signature[0:32]);
        bytes32 s = bytes32(signature[32:64]);
        uint8 v = uint8(signature[64]);
        if (uint256(s) > HALF_ORDER || (v != 27 && v != 28)) return address(0);
        return ecrecover(digest, v, r, s);
    }
}
