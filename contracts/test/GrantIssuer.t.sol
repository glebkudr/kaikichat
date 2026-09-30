// SPDX-License-Identifier: MIT
pragma solidity 0.8.36;

import {GrantIssuer} from "../src/GrantIssuer.sol";

interface GrantVm {
    struct Log {
        bytes32[] topics;
        bytes data;
        address emitter;
    }
    function prank(address) external;
    function warp(uint256) external;
    function expectRevert(bytes4) external;
    function recordLogs() external;
    function getRecordedLogs() external returns (Log[] memory);
}

contract GrantIssuerTest {
    GrantVm internal constant vm = GrantVm(address(uint160(uint256(keccak256("hevm cheat code")))));
    uint64 internal constant DAY = 86_400;
    uint64 internal constant START = 20_721;
    // fixtures/grant-book/v1.json, produced independently with Foundry cast.
    bytes32 internal constant DOMAIN = 0xbf5832c4f6f221989b409e5fb15ff71b86d708d6dec9a6580cf8a59fdab5e1a5;
    address internal constant SERVER = 0x974Fc34Db0b00B6Cf694E81cba82Ae65751A74Da;
    address internal constant BOOK = 0x5C7F047384eA1d2242350550704f211371F43731;
    address internal constant OTHER_BOOK = 0xdDb1E141f95b5149216b6fFBE61C0aC6Ff3b2450;
    uint64 internal constant EXPIRY = 1_792_886_400;
    bytes internal constant SIG_GRANT =
        hex"5f28f88b83ccf6e83fa57a0dec526c506154352c2e6e148a5ed76a07b56d8f350e4b487706fc45f88274596575572726d4275c23939b79cea9b0b17478c1ad181b";
    bytes internal constant SIG_GRANT_ALTERNATE =
        hex"4143de357d0fa022abbf84392febeaacc83aeb13e8a4f999f3db1e08baaa04326f6a9e0767ccb62ee014cb33cb960a39c08e6983c14e32f2f26e74835566f2431c";
    bytes internal constant SIG_CONFLICTING =
        hex"f23de57c0e092aed372cbd61dedefeb579a17b220ab13e31c64dce5d5e645fb65024f0a2bfef3d6439d3a3b695e997171ca925f5c4cdb6c4304a5057d33061981b";
    bytes internal constant SIG_SERIAL_8 =
        hex"469f4ef3cb97c4978902b438709ee064e9ce75ca99d5a7435bc51e72bd0873ad166d9b09c82006516209830a0171048ecf92eaac63f4ae0f50a1a7a56babc1ea1c";
    bytes internal constant SIG_NEXT_DAY =
        hex"0f0985cc652f6f961011dff005d4da789a18202f06a07e23c7fb4833925cce585fda4e07282caa461f5926f1e1fbc5689ef4b64d69563466211732d9883fecab1c";
    bytes internal constant SIG_FOREIGN_DOMAIN =
        hex"fb8410451cee4afb5b5dce767d34f10be24188648ae0c5448e0554bfcdf94b382771693139710b35de3a3b78e100a92151bd3d2e3bbaff536db76f203584dfdc1b";
    address internal constant OWNER = address(0xA11CE);
    address internal constant STRANGER = address(0xB0B);
    address internal constant SECOND = address(0x5EC0);

    GrantIssuer internal issuer;

    function setUp() public {
        goTo(START);
        issuer = new GrantIssuer(OWNER, DOMAIN, 50, 30, 1_000, SERVER);
    }

    function goTo(uint64 day) internal {
        vm.warp(uint256(day) * DAY + 3_600);
    }

    function setCap(uint64 cap) internal {
        vm.prank(OWNER);
        issuer.setDailyCap(cap);
    }

    function expectCapRevert(uint64 cap, bytes4 reason) internal {
        vm.prank(OWNER);
        vm.expectRevert(reason);
        issuer.setDailyCap(cap);
    }

    function grant(address book, uint32 serial, bytes memory signature)
        internal
        pure
        returns (GrantIssuer.SignedGrant memory)
    {
        return GrantIssuer.SignedGrant(book, START, serial, 50, EXPIRY, signature);
    }

    function testConstructorRejectsAMissingOwnerSizeOrCap() public {
        vm.expectRevert(GrantIssuer.InvalidConfiguration.selector);
        new GrantIssuer(address(0), DOMAIN, 50, 30, 1_000, SERVER);
        vm.expectRevert(GrantIssuer.InvalidConfiguration.selector);
        new GrantIssuer(OWNER, DOMAIN, 0, 30, 1_000, SERVER);
        // A zero cap could never be raised: 10x of zero is zero.
        vm.expectRevert(GrantIssuer.InvalidConfiguration.selector);
        new GrantIssuer(OWNER, DOMAIN, 50, 30, 0, SERVER);
    }

    function testInitialCapAppliesFromTheDeploymentDayOnwards() public view {
        require(issuer.today() == START, "today is the UTC day number");
        require(issuer.domain() == DOMAIN && issuer.bookSize() == 50 && issuer.maxValidityDays() == 30, "rules");
        require(issuer.capForDay(START - 1) == 0, "no emission before deployment");
        require(issuer.capForDay(START) == 1_000, "deployment day");
        require(issuer.capForDay(START + 400) == 1_000, "later days");
    }

    function testDeploymentCountsAsTheChangeOfItsDay() public {
        expectCapRevert(2_000, GrantIssuer.CapAlreadyChangedToday.selector);
    }

    function testRaiseUpToTenfoldTakesEffectNextDay() public {
        goTo(START + 1);
        vm.recordLogs();
        setCap(10_000);
        GrantVm.Log[] memory logs = vm.getRecordedLogs();
        require(logs.length == 1 && logs[0].emitter == address(issuer), "one event");
        require(logs[0].topics[0] == keccak256("DailyCapScheduled(uint64,uint64)"), "event");
        require(uint256(logs[0].topics[1]) == START + 2, "effective day indexed");
        require(abi.decode(logs[0].data, (uint64)) == 10_000, "cap in event");
        require(issuer.capForDay(START + 1) == 1_000, "today keeps its cap");
        require(issuer.capForDay(START + 2) == 10_000, "raised from tomorrow");
    }

    function testRaiseAboveTenfoldReverts() public {
        goTo(START + 1);
        expectCapRevert(10_001, GrantIssuer.CapStepTooLarge.selector);
        require(issuer.capForDay(START + 2) == 1_000, "unchanged");
    }

    function testLowerDownToOneTenthAndNotBelow() public {
        goTo(START + 1);
        expectCapRevert(99, GrantIssuer.CapStepTooLarge.selector);
        expectCapRevert(0, GrantIssuer.CapStepTooLarge.selector);
        setCap(100);
        require(issuer.capForDay(START + 2) == 100, "lowered from tomorrow");
    }

    function testSecondChangeOnTheSameDayReverts() public {
        goTo(START + 1);
        setCap(5_000);
        expectCapRevert(5_000, GrantIssuer.CapAlreadyChangedToday.selector);
        expectCapRevert(1_000, GrantIssuer.CapAlreadyChangedToday.selector);
    }

    function testStepsCompoundOnlyDayByDayAndHistoryStaysFixed() public {
        goTo(START + 1);
        setCap(10_000); // from START + 2
        expectCapRevert(100_000, GrantIssuer.CapAlreadyChangedToday.selector);
        goTo(START + 2);
        expectCapRevert(100_001, GrantIssuer.CapStepTooLarge.selector);
        setCap(100_000); // from START + 3
        goTo(START + 4);
        setCap(10_000); // from START + 5
        require(issuer.capForDay(START) == 1_000, "deployment day");
        require(issuer.capForDay(START + 1) == 1_000, "first change day");
        require(issuer.capForDay(START + 2) == 10_000, "first raise");
        require(issuer.capForDay(START + 3) == 100_000, "second raise");
        require(issuer.capForDay(START + 4) == 100_000, "no change day");
        require(issuer.capForDay(START + 5) == 10_000, "lowered");
        require(issuer.capForDay(START + 50) == 10_000, "latest");
    }

    function testNeitherTheServerKeyNorStrangersGovern() public {
        goTo(START + 1);
        address[2] memory outsiders = [SERVER, STRANGER];
        for (uint256 i; i < outsiders.length; ++i) {
            vm.prank(outsiders[i]);
            vm.expectRevert(GrantIssuer.NotOwner.selector);
            issuer.setDailyCap(10_000);
            vm.prank(outsiders[i]);
            vm.expectRevert(GrantIssuer.NotOwner.selector);
            issuer.setDailyCap(100);
            vm.prank(outsiders[i]);
            vm.expectRevert(GrantIssuer.NotOwner.selector);
            issuer.addIssuer(SECOND, START + 2);
            vm.prank(outsiders[i]);
            vm.expectRevert(GrantIssuer.NotOwner.selector);
            issuer.revokeIssuer(SERVER);
            vm.prank(outsiders[i]);
            vm.expectRevert(GrantIssuer.NotOwner.selector);
            issuer.transferOwnership(outsiders[i]);
        }
        require(issuer.capForDay(START + 2) == 1_000, "cap unchanged");
        require(issuer.issuerActiveOn(SERVER, START + 2) && !issuer.issuerActiveOn(SECOND, START + 2), "keys unchanged");
        require(issuer.owner() == OWNER && issuer.pendingOwner() == address(0), "owner unchanged");
    }

    function testFirstIssuerIsActiveFromDeployment() public view {
        require(!issuer.issuerActiveOn(SERVER, START - 1), "before deployment");
        require(issuer.issuerActiveOn(SERVER, START), "deployment day");
        require(issuer.issuerActiveOn(SERVER, START + 1000), "open window");
        require(!issuer.issuerActiveOn(STRANGER, START), "unknown account");
    }

    function testNewIssuerStartsNoEarlierThanTomorrow() public {
        vm.prank(OWNER);
        vm.expectRevert(GrantIssuer.StartTooEarly.selector);
        issuer.addIssuer(SECOND, START);
        vm.prank(OWNER);
        issuer.addIssuer(SECOND, START + 1);
        require(!issuer.issuerActiveOn(SECOND, START), "not today");
        require(issuer.issuerActiveOn(SECOND, START + 1), "from tomorrow");
        vm.prank(OWNER);
        vm.expectRevert(GrantIssuer.InvalidIssuer.selector);
        issuer.addIssuer(address(0), START + 3);
        vm.prank(OWNER);
        vm.expectRevert(GrantIssuer.IssuerAlreadyKnown.selector);
        issuer.addIssuer(SECOND, START + 5);
    }

    function testRevocationEndsTheWindowTodayAndIsPermanent() public {
        goTo(START + 2);
        vm.prank(OWNER);
        issuer.revokeIssuer(SERVER);
        require(issuer.issuerActiveOn(SERVER, START + 1), "earlier days keep their grants");
        require(!issuer.issuerActiveOn(SERVER, START + 2), "inactive from the revocation day");
        require(!issuer.issuerActiveOn(SERVER, START + 3), "stays inactive");
        vm.prank(OWNER);
        vm.expectRevert(GrantIssuer.IssuerNotActive.selector);
        issuer.revokeIssuer(SERVER);
        vm.prank(OWNER);
        vm.expectRevert(GrantIssuer.IssuerAlreadyKnown.selector);
        issuer.addIssuer(SERVER, START + 4);
        vm.prank(OWNER);
        vm.expectRevert(GrantIssuer.IssuerNotActive.selector);
        issuer.revokeIssuer(STRANGER);
    }

    function testOwnershipMovesOnlyWhenTheNewOwnerAccepts() public {
        vm.prank(OWNER);
        issuer.transferOwnership(SECOND);
        require(issuer.owner() == OWNER && issuer.pendingOwner() == SECOND, "pending");
        vm.prank(STRANGER);
        vm.expectRevert(GrantIssuer.NotPendingOwner.selector);
        issuer.acceptOwnership();
        vm.prank(SECOND);
        issuer.acceptOwnership();
        require(issuer.owner() == SECOND && issuer.pendingOwner() == address(0), "moved");
        goTo(START + 1);
        expectCapRevert(2_000, GrantIssuer.NotOwner.selector);
        vm.prank(SECOND);
        issuer.setDailyCap(2_000);
    }

    function testAnyoneRevokesAnIssuerThatSignedTwoGrantsForOneSlotFromTomorrow() public {
        require(
            issuer.grantDigest(DOMAIN, SERVER, BOOK, START, 7, 50, EXPIRY)
                == 0x9f9e8f2ccd55c494a66060f34389831314b154f6496fe91b2cca272ffa0e405e,
            "digest differs from cast"
        );
        goTo(START + 2);
        vm.recordLogs();
        vm.prank(STRANGER);
        issuer.reportEquivocation(SERVER, grant(BOOK, 7, SIG_GRANT), grant(OTHER_BOOK, 7, SIG_CONFLICTING));
        GrantVm.Log[] memory logs = vm.getRecordedLogs();
        bool reported;
        bool revoked;
        for (uint256 i; i < logs.length; ++i) {
            if (logs[i].topics[0] == keccak256("EquivocationReported(address,uint64,uint32,address)")) {
                require(address(uint160(uint256(logs[i].topics[1]))) == SERVER, "issuer indexed");
                (uint64 day, uint32 serial, address reporter) = abi.decode(logs[i].data, (uint64, uint32, address));
                require(day == START && serial == 7 && reporter == STRANGER, "report data");
                reported = true;
            }
            if (logs[i].topics[0] == keccak256("IssuerRevoked(address,uint64)")) {
                require(abi.decode(logs[i].data, (uint64)) == START + 3, "revoked from tomorrow");
                revoked = true;
            }
        }
        require(reported && revoked, "report and revocation events");
        // Grants already issued today stay valid; the key mints nothing from tomorrow.
        require(issuer.issuerActiveOn(SERVER, START + 1), "earlier days keep their grants");
        require(issuer.issuerActiveOn(SERVER, START + 2), "today keeps its grants");
        require(!issuer.issuerActiveOn(SERVER, START + 3), "inactive from tomorrow");
        vm.prank(STRANGER);
        vm.expectRevert(GrantIssuer.IssuerNotActive.selector);
        issuer.reportEquivocation(SERVER, grant(BOOK, 7, SIG_GRANT), grant(OTHER_BOOK, 7, SIG_CONFLICTING));
        // The cold owner may still stop the key immediately.
        vm.prank(OWNER);
        issuer.revokeIssuer(SERVER);
        require(!issuer.issuerActiveOn(SERVER, START + 2), "owner revoked from today");
        require(issuer.issuerActiveOn(SERVER, START + 1), "earlier days untouched");
    }

    function testAReportNeverReopensAKeyTheOwnerStopped() public {
        goTo(START + 2);
        vm.prank(OWNER);
        issuer.revokeIssuer(SERVER);
        // A thief holding the key can sign a conflicting pair himself; the
        // report must not move the owner's earlier end of the window.
        vm.prank(STRANGER);
        vm.expectRevert(GrantIssuer.IssuerNotActive.selector);
        issuer.reportEquivocation(SERVER, grant(BOOK, 7, SIG_GRANT), grant(OTHER_BOOK, 7, SIG_CONFLICTING));
        require(!issuer.issuerActiveOn(SERVER, START + 2), "stopped today");
        goTo(START + 4);
        vm.prank(STRANGER);
        vm.expectRevert(GrantIssuer.IssuerNotActive.selector);
        issuer.reportEquivocation(SERVER, grant(BOOK, 7, SIG_GRANT), grant(OTHER_BOOK, 7, SIG_CONFLICTING));
        for (uint64 day = START + 2; day <= START + 4; ++day) {
            require(!issuer.issuerActiveOn(SERVER, day), "stays stopped");
        }
    }

    function testOnlyAConflictingPairOfTheIssuersSignaturesRevokes() public {
        goTo(START + 2);
        // One grant, even signed twice with different nonces, is not two grants.
        vm.expectRevert(GrantIssuer.NotEquivocation.selector);
        issuer.reportEquivocation(SERVER, grant(BOOK, 7, SIG_GRANT), grant(BOOK, 7, SIG_GRANT_ALTERNATE));
        // Different serials are different slots.
        vm.expectRevert(GrantIssuer.NotEquivocation.selector);
        issuer.reportEquivocation(SERVER, grant(BOOK, 7, SIG_GRANT), grant(OTHER_BOOK, 8, SIG_SERIAL_8));
        // Serials restart every day: the same serial tomorrow is another slot.
        GrantIssuer.SignedGrant memory tomorrow = grant(OTHER_BOOK, 7, SIG_NEXT_DAY);
        tomorrow.day = START + 1;
        vm.expectRevert(GrantIssuer.NotEquivocation.selector);
        issuer.reportEquivocation(SERVER, grant(BOOK, 7, SIG_GRANT), tomorrow);
        // A grant for another network does not verify under this domain.
        vm.expectRevert(GrantIssuer.InvalidSignature.selector);
        issuer.reportEquivocation(SERVER, grant(BOOK, 7, SIG_GRANT), grant(OTHER_BOOK, 7, SIG_FOREIGN_DOMAIN));
        // A tampered signature in either position proves nothing.
        bytes memory tampered = bytes.concat(SIG_CONFLICTING);
        tampered[3] ^= 0x01;
        vm.expectRevert(GrantIssuer.InvalidSignature.selector);
        issuer.reportEquivocation(SERVER, grant(BOOK, 7, SIG_GRANT), grant(OTHER_BOOK, 7, tampered));
        vm.expectRevert(GrantIssuer.InvalidSignature.selector);
        issuer.reportEquivocation(SERVER, grant(OTHER_BOOK, 7, tampered), grant(BOOK, 7, SIG_GRANT));
        require(issuer.issuerActiveOn(SERVER, START + 2), "still active");
        // A proof against a key this contract never registered revokes nothing.
        GrantIssuer unrelated = new GrantIssuer(OWNER, DOMAIN, 50, 30, 1_000, SECOND);
        vm.expectRevert(GrantIssuer.IssuerNotActive.selector);
        unrelated.reportEquivocation(SERVER, grant(BOOK, 7, SIG_GRANT), grant(OTHER_BOOK, 7, SIG_CONFLICTING));
    }
}
