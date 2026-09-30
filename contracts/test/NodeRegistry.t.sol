// SPDX-License-Identifier: MIT
pragma solidity 0.8.36;

import {NodeRegistry} from "../src/NodeRegistry.sol";
import {Vm} from "./Vm.sol";

interface RegistryVm is Vm {
    function roll(uint256) external;
    function setBlockhash(uint256, bytes32) external;
}

contract RegistryWithdrawer {
    NodeRegistry internal immutable registry;
    uint32 public index;
    bool public rejectPayment;
    bool public reentered;
    uint256 public callbacks;

    constructor(NodeRegistry target) {
        registry = target;
    }

    function bond(bytes32 commitment) external payable {
        index = registry.bond{value: msg.value}(commitment);
    }

    function exit() external {
        registry.requestExit(index);
    }

    function reject(bool value) external {
        rejectPayment = value;
    }

    function claim(address payable recipient) external {
        registry.withdraw(index, recipient);
    }

    receive() external payable {
        require(!rejectPayment, "recipient rejected payment");
        ++callbacks;
        if (callbacks == 1) {
            (reentered,) = address(registry).call(abi.encodeCall(registry.withdraw, (index, payable(this))));
        }
    }
}

contract NodeRegistryTest {
    RegistryVm internal constant vm = RegistryVm(address(uint160(uint256(keccak256("hevm cheat code")))));
    bytes32 internal constant GENESIS = keccak256("registry-local-fixture-genesis");
    bytes32 internal constant EMPTY_TAG = keccak256("AgenticInternet/RegistryEmpty/v1");
    bytes32 internal constant LEAF_TAG = keccak256("AgenticInternet/RegistryLeaf/v1");
    bytes32 internal constant BRANCH_TAG = keccak256("AgenticInternet/RegistryBranch/v1");
    bytes32 internal constant SEED_TAG = keccak256("AgenticInternet/RegistrySeed/evm-blockhash-v1");
    uint128 internal constant PRICE = 101;
    uint32 internal constant EPOCH = 60;
    uint32 internal constant LEASE = 120;
    uint32 internal constant OBLIGATION = 600;
    uint16 internal constant DELAY = 3;
    address internal constant ALICE = address(0xA11CE);
    address internal constant BOB = address(0xB0B);
    NodeRegistry internal registry;
    bytes32[16] internal commitments;
    address[16] internal owners;
    bool[16] internal active;
    uint32 internal registered;

    function setUp() public {
        vm.warp(1800000000);
        vm.roll(1000);
        vm.chainId(31337);
        vm.deal(ALICE, 1 ether);
        vm.deal(BOB, 1 ether);
        registry = new NodeRegistry(GENESIS, PRICE, EPOCH, LEASE, OBLIGATION, DELAY);
    }

    function join(address owner, bytes32 commitment) internal returns (uint32 index) {
        vm.prank(owner);
        index = registry.bond{value: PRICE}(commitment);
        require(index == registered, "nonmonotonic/reused bond index");
        commitments[index] = commitment;
        owners[index] = owner;
        active[index] = true;
        ++registered;
    }

    function leave(uint32 index) internal {
        vm.prank(owners[index]);
        registry.requestExit(index);
        active[index] = false;
    }

    function parent(bytes32 left, uint64 leftCount, bytes32 right, uint64 rightCount) internal pure returns (bytes32) {
        return keccak256(abi.encode(BRANCH_TAG, left, leftCount, right, rightCount));
    }

    // Independent full finite-array tree reconstruction: no production tree/hash helper.
    function expectedRoot() internal view returns (bytes32 root, uint64 count) {
        bytes32[16] memory hashes;
        uint64[16] memory counts;
        bytes32 empty = keccak256(abi.encode(EMPTY_TAG, registry.domain()));
        for (uint32 i; i < 16; ++i) {
            hashes[i] =
                active[i] ? keccak256(abi.encode(LEAF_TAG, registry.domain(), i, owners[i], commitments[i])) : empty;
            counts[i] = active[i] ? 1 : 0;
        }
        uint256 width = 16;
        for (uint256 level; level < 32; ++level) {
            if (width > 1) {
                for (uint256 i; i < width / 2; ++i) {
                    hashes[i] = parent(hashes[2 * i], counts[2 * i], hashes[2 * i + 1], counts[2 * i + 1]);
                    counts[i] = counts[2 * i] + counts[2 * i + 1];
                }
                width /= 2;
            } else {
                hashes[0] = parent(hashes[0], counts[0], empty, 0);
            }
            empty = parent(empty, 0, empty, 0);
        }
        return (hashes[0], counts[0]);
    }

    function checkRoot() internal view {
        (bytes32 expected, uint64 count) = expectedRoot();
        (bytes32 actual, uint64 observed) = registry.activeRoot();
        require(actual == expected && observed == count, "registry differs from independent full tree/count");
    }

    function balances(uint256 deposited, uint256 held, uint256 withdrawn) internal view {
        require(registry.totalDepositedWei() == deposited, "wrong total deposited");
        require(registry.heldBondWei() == held && registry.totalWithdrawnWei() == withdrawn, "wrong bond accounting");
        require(deposited == held + withdrawn, "bond conservation violated");
        require(address(registry).balance >= held, "unbacked held bond");
    }

    /// Nodes page through the active units' commitments to build their directory.
    function testActiveUnitsArePagedWithoutExitedOnes() public {
        for (uint32 i; i < 5; ++i) {
            join(i % 2 == 0 ? ALICE : BOB, keccak256(abi.encode("unit", i)));
        }
        leave(1);
        leave(3);
        (bytes32[] memory first, uint64 next) = registry.activeUnits(0, 2);
        require(next == 2 && first.length == 1 && first[0] == commitments[0], "first page");
        (bytes32[] memory rest, uint64 end) = registry.activeUnits(uint32(next), 10);
        require(end == 5 && rest.length == 2, "rest");
        require(rest[0] == commitments[2] && rest[1] == commitments[4], "rest order");
        (bytes32[] memory none, uint64 past) = registry.activeUnits(7, 10);
        require(none.length == 0 && past == 5, "past the end");
        // The node calls it by this selector.
        require(registry.activeUnits.selector == bytes4(0x5dab6b66), "selector");
    }

    function testExactFixedBondAuthenticatesEveryEntryAndConservesPrincipal() public {
        checkRoot();
        bytes32 a = keccak256("alice operator commitment");
        uint256 before = ALICE.balance;
        vm.recordLogs();
        join(ALICE, a);
        Vm.Log[] memory logs = vm.getRecordedLogs();
        require(logs.length == 1 && logs[0].emitter == address(registry), "missing bond event");
        require(logs[0].topics[0] == keccak256("Bonded(uint32,address,bytes32,uint256)"), "wrong bond event");
        require(logs[0].topics.length == 3 && logs[0].topics[1] == bytes32(0), "wrong index");
        require(logs[0].topics[2] == bytes32(uint256(uint160(ALICE))), "wrong owner");
        require(keccak256(logs[0].data) == keccak256(abi.encode(a, uint256(PRICE))), "wrong event allocation");
        require(ALICE.balance == before - PRICE, "bond did not charge principal");
        NodeRegistry.Unit memory unit = registry.unit(0);
        require(
            unit.owner == ALICE && unit.commitment == a && unit.state == NodeRegistry.UnitState.Active, "wrong unit"
        );
        require(unit.exitRequestedAt == 0 && unit.withdrawAfter == 0, "new bond is exiting");
        join(BOB, keccak256("bob commitment"));
        join(ALICE, keccak256("alice second fixed unit"));
        require(registry.nextIndex() == 3, "wrong issued unit count");
        balances(303, 303, 0);
        checkRoot();
        require(registry.currentEpoch() == 1, "wrong initial epoch");
    }

    function testWrongPaymentsDuplicatesAndZeroCommitmentLeaveRegistryAndFundsUntouched() public {
        bytes32 a = keccak256("fixed price");
        join(ALICE, a);
        (bytes32 root, uint64 count) = registry.activeRoot();
        uint256 before = BOB.balance;
        vm.expectRevert(NodeRegistry.WrongPayment.selector);
        vm.prank(BOB);
        registry.bond{value: PRICE - 1}(keccak256("underpaid"));
        vm.expectRevert(NodeRegistry.WrongPayment.selector);
        vm.prank(BOB);
        registry.bond{value: PRICE * 2}(keccak256("no implicit two-unit bonus"));
        vm.expectRevert(NodeRegistry.InvalidCommitment.selector);
        vm.prank(BOB);
        registry.bond{value: PRICE}(bytes32(0));
        vm.expectRevert(NodeRegistry.CommitmentAlreadyUsed.selector);
        vm.prank(ALICE);
        registry.bond{value: PRICE}(a);
        require(BOB.balance == before && registry.nextIndex() == 1, "failed registration changed funds/count");
        (bytes32 afterRoot, uint64 afterCount) = registry.activeRoot();
        require(afterRoot == root && afterCount == count, "failed registration changed root");
        balances(PRICE, PRICE, 0);
        // Another owner can fund the same opaque bytes, but cannot open Alice's owner-bound commitment.
        join(BOB, a);
        checkRoot();
    }

    function testSealedEpochBindsRootCountAndFutureBlockDespiteLaterBondsAndExits() public {
        join(ALICE, keccak256("a"));
        join(BOB, keccak256("b"));
        (bytes32 root, uint64 count) = expectedRoot();
        vm.recordLogs();
        uint64 epoch = registry.sealEpoch();
        require(epoch == 1, "sealed wrong epoch");
        NodeRegistry.Snapshot memory snap = registry.snapshot(epoch);
        require(snap.domain == registry.domain() && snap.root == root && snap.count == count, "wrong sealed membership");
        require(snap.sealedAt == block.timestamp && snap.sealedBlock == block.number, "wrong seal position");
        require(
            snap.beaconBlock == block.number + DELAY && snap.admissionUntil == block.timestamp + LEASE,
            "wrong finite horizon"
        );
        require(snap.seed == 0, "seed was known at seal");
        Vm.Log[] memory logs = vm.getRecordedLogs();
        require(
            logs.length == 1
                && logs[0].topics[0] == keccak256("EpochSealed(uint64,bytes32,uint64,uint64,uint64,uint64,uint64)"),
            "missing snapshot event"
        );
        require(logs[0].topics[1] == bytes32(uint256(epoch)), "wrong indexed epoch");
        require(
            keccak256(logs[0].data)
                == keccak256(
                    abi.encode(root, count, snap.sealedAt, snap.sealedBlock, snap.beaconBlock, snap.admissionUntil)
                ),
            "wrong sealed event"
        );
        bytes32 frozen = keccak256(abi.encode(snap));
        join(ALICE, keccak256("late bond"));
        leave(0);
        checkRoot();
        require(keccak256(abi.encode(registry.snapshot(epoch))) == frozen, "post-seal change rewrote assignment");
        vm.expectRevert(NodeRegistry.EpochAlreadySealed.selector);
        registry.sealEpoch();
        vm.warp(block.timestamp + EPOCH);
        require(registry.sealEpoch() == 2, "next epoch unavailable");
        (root, count) = expectedRoot();
        snap = registry.snapshot(2);
        require(snap.root == root && snap.count == count && snap.seed == 0, "next snapshot lost changes");
        require(keccak256(abi.encode(registry.snapshot(1))) == frozen, "new epoch overwrote old proof root");
    }

    function testSeedUsesExactCommittedFutureBlockAndSavedRetryCannotRenewAdmission() public {
        join(ALICE, keccak256("future beacon"));
        uint64 epoch = registry.sealEpoch();
        NodeRegistry.Snapshot memory snap = registry.snapshot(epoch);
        vm.expectRevert(NodeRegistry.BeaconUnavailable.selector);
        registry.captureSeed(epoch);
        vm.roll(snap.beaconBlock);
        vm.expectRevert(NodeRegistry.BeaconUnavailable.selector);
        registry.captureSeed(epoch);
        vm.roll(uint256(snap.beaconBlock) + 1);
        bytes32 header = keccak256("exact producer-controlled future block fixture");
        vm.setBlockhash(snap.beaconBlock, header);
        bytes32 expected =
            keccak256(abi.encode(SEED_TAG, snap.domain, epoch, snap.root, snap.count, snap.beaconBlock, header));
        vm.prank(BOB);
        bytes32 observed = registry.captureSeed(epoch);
        require(observed == expected, "caller/time/other block entered seed");
        snap.seed = observed;
        require(
            keccak256(abi.encode(registry.snapshot(epoch))) == keccak256(abi.encode(snap)),
            "seed capture changed frozen fields"
        );
        vm.warp(uint256(snap.admissionUntil) + 1);
        vm.roll(uint256(snap.beaconBlock) + 300);
        require(registry.captureSeed(epoch) == observed, "historical retry changed seed");
        require(registry.snapshot(epoch).admissionUntil == snap.admissionUntil, "seed retry renewed expired admission");
    }

    function testMissedBeaconHasNoLatestBlockFallbackAndCaptureHorizonIsExact() public {
        vm.expectRevert(NodeRegistry.EmptyRegistry.selector);
        registry.sealEpoch();
        vm.expectRevert(NodeRegistry.UnknownEpoch.selector);
        registry.captureSeed(1);
        join(ALICE, keccak256("capture deadline"));
        registry.sealEpoch();
        NodeRegistry.Snapshot memory first = registry.snapshot(1);
        vm.roll(uint256(first.beaconBlock) + 256);
        vm.setBlockhash(first.beaconBlock, keccak256("last available blockhash"));
        require(registry.captureSeed(1) != 0, "last blockhash window rejected");
        vm.warp(block.timestamp + EPOCH);
        registry.sealEpoch();
        NodeRegistry.Snapshot memory second = registry.snapshot(2);
        vm.roll(uint256(second.beaconBlock) + 257);
        vm.expectRevert(NodeRegistry.BeaconUnavailable.selector);
        registry.captureSeed(2);
        require(registry.snapshot(2).seed == 0, "expired blockhash created fallback seed");
        vm.warp(block.timestamp + EPOCH * 3);
        require(registry.sealEpoch() == 5, "empty epochs silently renumbered/replayed");
        require(registry.snapshot(3).root == 0 && registry.snapshot(4).root == 0, "past epoch manufactured");
    }

    function testExitRemovesFutureEligibilityButLocksPrincipalPastAllSnapshotObligations() public {
        bytes32 commitment = keccak256("locked operator");
        join(ALICE, commitment);
        registry.sealEpoch();
        NodeRegistry.Snapshot memory snap = registry.snapshot(1);
        vm.expectRevert(NodeRegistry.Unauthorized.selector);
        vm.prank(BOB);
        registry.requestExit(0);
        vm.expectRevert(NodeRegistry.NotExiting.selector);
        vm.prank(ALICE);
        registry.withdraw(0, payable(ALICE));
        vm.warp(block.timestamp + 7);
        leave(0);
        NodeRegistry.Unit memory exiting = registry.unit(0);
        require(
            exiting.state == NodeRegistry.UnitState.Exiting && exiting.exitRequestedAt == block.timestamp,
            "missing exit state"
        );
        require(
            exiting.withdrawAfter == block.timestamp + LEASE + OBLIGATION, "bond ignores latest possible obligation"
        );
        require(exiting.withdrawAfter >= snap.admissionUntil + OBLIGATION, "bond unlocks during sealed obligation");
        checkRoot();
        require(keccak256(abi.encode(registry.snapshot(1))) == keccak256(abi.encode(snap)), "exit rewrote old root");
        vm.expectRevert(NodeRegistry.NotActive.selector);
        vm.prank(ALICE);
        registry.requestExit(0);
        vm.warp(uint256(exiting.withdrawAfter) - 1);
        vm.expectRevert(NodeRegistry.BondLocked.selector);
        vm.prank(ALICE);
        registry.withdraw(0, payable(ALICE));
        balances(PRICE, PRICE, 0);
        vm.warp(exiting.withdrawAfter);
        vm.expectRevert(NodeRegistry.Unauthorized.selector);
        vm.prank(BOB);
        registry.withdraw(0, payable(BOB));
        vm.expectRevert(NodeRegistry.InvalidRecipient.selector);
        vm.prank(ALICE);
        registry.withdraw(0, payable(address(0)));
        uint256 before = ALICE.balance;
        vm.prank(ALICE);
        registry.withdraw(0, payable(ALICE));
        require(ALICE.balance == before + PRICE, "withdrawal lost principal");
        balances(PRICE, 0, PRICE);
        vm.expectRevert(NodeRegistry.NotExiting.selector);
        vm.prank(ALICE);
        registry.withdraw(0, payable(ALICE));
        vm.expectRevert(NodeRegistry.CommitmentAlreadyUsed.selector);
        vm.prank(ALICE);
        registry.bond{value: PRICE}(commitment);
        join(ALICE, keccak256("new commitment after prior exit"));
        require(registry.unit(0).state == NodeRegistry.UnitState.Withdrawn, "old index reused");
        checkRoot();
    }

    function testWithdrawalFailureRollsBackAndReentrantRecipientCannotWithdrawTwice() public {
        RegistryWithdrawer receiver = new RegistryWithdrawer(registry);
        vm.deal(address(this), PRICE);
        receiver.bond{value: PRICE}(keccak256("contract-owned bond"));
        receiver.exit();
        vm.warp(registry.unit(0).withdrawAfter);
        receiver.reject(true);
        vm.expectRevert(NodeRegistry.TransferFailed.selector);
        receiver.claim(payable(address(receiver)));
        require(registry.unit(0).state == NodeRegistry.UnitState.Exiting, "failed callback lost claim");
        balances(PRICE, PRICE, 0);
        vm.prank(BOB);
        uint32 bob = registry.bond{value: PRICE}(keccak256("other owner keeps working"));
        vm.prank(BOB);
        registry.requestExit(bob);
        vm.warp(registry.unit(bob).withdrawAfter);
        // A second real, matured claim backs the attempted duplicate withdrawal.
        balances(PRICE * 2, PRICE * 2, 0);
        require(address(registry).balance == PRICE * 2, "attack masked by empty balance");
        receiver.reject(false);
        receiver.claim(payable(address(receiver)));
        require(receiver.callbacks() == 1 && !receiver.reentered(), "callback released duplicate value");
        require(address(receiver).balance == PRICE, "wrong callback payment");
        balances(PRICE * 2, PRICE, PRICE);
        require(registry.unit(bob).state == NodeRegistry.UnitState.Exiting, "callback consumed another owner claim");
        uint256 bobBefore = BOB.balance;
        vm.prank(BOB);
        registry.withdraw(bob, payable(BOB));
        require(BOB.balance == bobBefore + PRICE, "callback stole another owner principal");
        balances(PRICE * 2, 0, PRICE * 2);
    }

    function testImmutableDomainConfigurationAndChangedChainCannotMutateOldRegistry() public {
        bytes32 config = keccak256(abi.encode(PRICE, EPOCH, LEASE, OBLIGATION, DELAY));
        bytes32 expected = keccak256(
            abi.encode(keccak256("AgenticInternet/NodeRegistry/v1"), uint256(31337), address(registry), GENESIS, config)
        );
        require(registry.domain() == expected && registry.configHash() == config, "wrong deployment domain");
        require(
            registry.deploymentChainId() == 31337 && registry.startedAt() == block.timestamp, "wrong deployment context"
        );
        NodeRegistry other = new NodeRegistry(GENESIS, PRICE, EPOCH, LEASE, OBLIGATION, DELAY);
        require(other.domain() != expected, "deployment address omitted from domain");
        join(ALICE, keccak256("chain scoped bond"));
        registry.sealEpoch();
        vm.chainId(31338);
        vm.expectRevert(NodeRegistry.WrongChain.selector);
        vm.prank(ALICE);
        registry.bond{value: PRICE}(keccak256("other chain"));
        vm.expectRevert(NodeRegistry.WrongChain.selector);
        vm.prank(ALICE);
        registry.requestExit(0);
        vm.expectRevert(NodeRegistry.WrongChain.selector);
        registry.sealEpoch();
        vm.expectRevert(NodeRegistry.WrongChain.selector);
        registry.captureSeed(1);
        vm.expectRevert(NodeRegistry.WrongChain.selector);
        vm.prank(ALICE);
        registry.withdraw(0, payable(ALICE));
        require(registry.domain() == expected, "historical domain drifted");
        balances(PRICE, PRICE, 0);
        vm.chainId(31337);
        leave(0);
        checkRoot();
    }

    function testInvalidConfigurationCannotCreateUnfundedOrUnboundedObligations() public {
        vm.expectRevert(NodeRegistry.InvalidConfiguration.selector);
        new NodeRegistry(bytes32(0), PRICE, EPOCH, LEASE, OBLIGATION, DELAY);
        vm.expectRevert(NodeRegistry.InvalidConfiguration.selector);
        new NodeRegistry(GENESIS, 0, EPOCH, LEASE, OBLIGATION, DELAY);
        vm.expectRevert(NodeRegistry.InvalidConfiguration.selector);
        new NodeRegistry(GENESIS, PRICE, 0, LEASE, OBLIGATION, DELAY);
        vm.expectRevert(NodeRegistry.InvalidConfiguration.selector);
        new NodeRegistry(GENESIS, PRICE, EPOCH, EPOCH - 1, OBLIGATION, DELAY);
        vm.expectRevert(NodeRegistry.InvalidConfiguration.selector);
        new NodeRegistry(GENESIS, PRICE, EPOCH, LEASE, 0, DELAY);
        vm.expectRevert(NodeRegistry.InvalidConfiguration.selector);
        new NodeRegistry(GENESIS, PRICE, EPOCH, LEASE, OBLIGATION, 0);
        vm.expectRevert(NodeRegistry.InvalidConfiguration.selector);
        new NodeRegistry(GENESIS, PRICE, EPOCH, 8 days, OBLIGATION, DELAY);
        vm.expectRevert(NodeRegistry.InvalidConfiguration.selector);
        new NodeRegistry(GENESIS, PRICE, 59, LEASE, OBLIGATION, DELAY);
        vm.expectRevert(NodeRegistry.InvalidConfiguration.selector);
        new NodeRegistry(GENESIS, PRICE, 86401, 86401, OBLIGATION, DELAY);
        vm.expectRevert(NodeRegistry.InvalidConfiguration.selector);
        new NodeRegistry(GENESIS, PRICE, EPOCH, LEASE, 367 days, DELAY);
        vm.expectRevert(NodeRegistry.InvalidConfiguration.selector);
        new NodeRegistry(GENESIS, PRICE, EPOCH, LEASE, OBLIGATION, 129);
        vm.chainId(0);
        vm.expectRevert(NodeRegistry.InvalidConfiguration.selector);
        new NodeRegistry(GENESIS, PRICE, EPOCH, LEASE, OBLIGATION, DELAY);
    }

    function testFuzzFixedUnitsConserveFundsAndIndependentRootThroughExitAndWithdrawal(uint256 entropy) public {
        uint256 deposited;
        uint256 withdrawn;
        for (uint32 step; step < 48; ++step) {
            entropy = uint256(keccak256(abi.encode(entropy, step)));
            uint256 action = entropy % 3;
            if ((action == 0 || registered == 0) && registered < 16) {
                join(entropy % 2 == 0 ? ALICE : BOB, keccak256(abi.encode("fuzz operator", step, entropy)));
                deposited += PRICE;
            } else if (registered > 0) {
                uint32 index = uint32(entropy % registered);
                NodeRegistry.Unit memory unit = registry.unit(index);
                if (unit.state == NodeRegistry.UnitState.Active && action == 1) {
                    leave(index);
                } else if (unit.state == NodeRegistry.UnitState.Exiting && action == 2) {
                    vm.warp(uint256(unit.withdrawAfter) > block.timestamp ? unit.withdrawAfter : block.timestamp);
                    vm.prank(owners[index]);
                    registry.withdraw(index, payable(owners[index]));
                    withdrawn += PRICE;
                }
            }
            balances(deposited, deposited - withdrawn, withdrawn);
            checkRoot();
        }
    }
}
