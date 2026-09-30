// SPDX-License-Identifier: MIT
pragma solidity 0.8.36;

/// @notice Fixed paid operator units and immutable, finite admission snapshots.
/// @dev Capital commitments do not prove operator independence or availability. Consumers
/// must authenticate chain state, verify owner-bound openings/key possession and enforce
/// admission/obligation horizons. The labelled EVM blockhash seed is producer-biasable.
contract NodeRegistry {
    enum UnitState {
        None,
        Active,
        Exiting,
        Withdrawn
    }

    struct Unit {
        address owner;
        bytes32 commitment;
        uint64 exitRequestedAt;
        uint64 withdrawAfter;
        UnitState state;
    }

    struct Snapshot {
        bytes32 domain;
        bytes32 root;
        uint64 count;
        uint64 sealedAt;
        uint64 sealedBlock;
        uint64 beaconBlock;
        uint64 admissionUntil;
        bytes32 seed;
    }

    struct TreeNode {
        bytes32 hash;
        uint64 count;
    }

    error InvalidConfiguration();
    error WrongChain();
    error WrongPayment();
    error InvalidCommitment();
    error CommitmentAlreadyUsed();
    error RegistryFull();
    error PositionOverflow();
    error Unauthorized();
    error NotActive();
    error NotExiting();
    error BondLocked();
    error InvalidRecipient();
    error TransferFailed();
    error EmptyRegistry();
    error EpochAlreadySealed();
    error UnknownEpoch();
    error BeaconUnavailable();

    event Bonded(uint32 indexed index, address indexed owner, bytes32 commitment, uint256 amount);
    event ExitRequested(uint32 indexed index, uint64 exitRequestedAt, uint64 withdrawAfter);
    event Withdrawn(uint32 indexed index, address indexed recipient, uint256 amount);
    event EpochSealed(
        uint64 indexed epoch,
        bytes32 root,
        uint64 count,
        uint64 sealedAt,
        uint64 sealedBlock,
        uint64 beaconBlock,
        uint64 admissionUntil
    );
    event SeedCaptured(uint64 indexed epoch, bytes32 seed);

    bytes32 private constant DOMAIN_TAG = keccak256("AgenticInternet/NodeRegistry/v1");
    bytes32 private constant EMPTY_TAG = keccak256("AgenticInternet/RegistryEmpty/v1");
    bytes32 private constant LEAF_TAG = keccak256("AgenticInternet/RegistryLeaf/v1");
    bytes32 private constant BRANCH_TAG = keccak256("AgenticInternet/RegistryBranch/v1");
    bytes32 private constant SEED_TAG = keccak256("AgenticInternet/RegistrySeed/evm-blockhash-v1");

    bytes32 public immutable genesis;
    bytes32 public immutable configHash;
    bytes32 public immutable domain;
    uint256 public immutable deploymentChainId;
    uint64 public immutable startedAt;
    uint128 public immutable unitBondWei;
    uint32 public immutable epochSeconds;
    uint32 public immutable snapshotLeaseSeconds;
    uint32 public immutable maxObligationSeconds;
    uint16 public immutable futureBlockDelay;

    uint64 public nextIndex;
    uint256 public totalDepositedWei;
    uint256 public heldBondWei;
    uint256 public totalWithdrawnWei;
    mapping(uint32 index => Unit) private units;
    mapping(address owner => mapping(bytes32 commitment => bool)) private usedCommitments;
    mapping(uint64 epoch => Snapshot) private snapshots;
    // Key = level * 2^32 + position. Each mutation changes one bounded 32-level path.
    mapping(uint64 key => TreeNode) private nodes;
    bytes32[33] private emptyHashes;

    constructor(
        bytes32 genesisDigest,
        uint128 unitPrice,
        uint32 epochDuration,
        uint32 admissionLease,
        uint32 obligationDuration,
        uint16 beaconDelay
    ) {
        if (
            genesisDigest == 0 || block.chainid == 0 || unitPrice == 0 || epochDuration < 60 || epochDuration > 1 days
                || admissionLease < epochDuration || admissionLease > 7 days || obligationDuration == 0
                || obligationDuration > 366 days || beaconDelay == 0 || beaconDelay > 128
        ) revert InvalidConfiguration();
        genesis = genesisDigest;
        unitBondWei = unitPrice;
        epochSeconds = epochDuration;
        snapshotLeaseSeconds = admissionLease;
        maxObligationSeconds = obligationDuration;
        futureBlockDelay = beaconDelay;
        deploymentChainId = block.chainid;
        startedAt = position64(block.timestamp);
        configHash = keccak256(abi.encode(unitPrice, epochDuration, admissionLease, obligationDuration, beaconDelay));
        domain = keccak256(abi.encode(DOMAIN_TAG, block.chainid, address(this), genesisDigest, configHash));
        emptyHashes[0] = keccak256(abi.encode(EMPTY_TAG, domain));
        for (uint8 level; level < 32; ++level) {
            emptyHashes[level + 1] =
                keccak256(abi.encode(BRANCH_TAG, emptyHashes[level], uint64(0), emptyHashes[level], uint64(0)));
        }
    }

    modifier sameChain() {
        if (block.chainid != deploymentChainId) revert WrongChain();
        _;
    }

    function currentEpoch() public view returns (uint64) {
        return position64((block.timestamp - startedAt) / epochSeconds + 1);
    }

    function activeRoot() public view returns (bytes32 hash, uint64 count) {
        TreeNode memory root = treeNode(32, 0);
        return (root.hash, root.count);
    }

    /// @notice Unknown indices return state None; indices are never recycled.
    function unit(uint32 index) external view returns (Unit memory) {
        return units[index];
    }

    /// @notice Commitments of the active units among registry positions [start, start + limit),
    /// and where the next page starts; a page can be short before the end.
    function activeUnits(uint32 start, uint32 limit) external view returns (bytes32[] memory commitments, uint64 next) {
        next = uint64(start) + limit;
        if (next > nextIndex) next = nextIndex;
        uint64 first = start < next ? start : next;
        uint256 found;
        for (uint64 index = first; index < next; ++index) {
            if (units[uint32(index)].state == UnitState.Active) ++found;
        }
        commitments = new bytes32[](found);
        found = 0;
        for (uint64 index = first; index < next; ++index) {
            Unit storage entry = units[uint32(index)];
            if (entry.state == UnitState.Active) commitments[found++] = entry.commitment;
        }
    }

    /// @notice A zero count identifies an epoch that was never frozen.
    function snapshot(uint64 epoch) external view returns (Snapshot memory) {
        return snapshots[epoch];
    }

    function bond(bytes32 commitment) external payable virtual sameChain returns (uint32 index) {
        return _bond(commitment);
    }

    /// @dev Derived registries may enforce role eligibility before the shared accounting path.
    /// Their external entry point must also enforce sameChain.
    function _bond(bytes32 commitment) internal returns (uint32 index) {
        if (msg.value != unitBondWei) revert WrongPayment();
        if (commitment == 0) revert InvalidCommitment();
        if (usedCommitments[msg.sender][commitment]) revert CommitmentAlreadyUsed();
        if (nextIndex >= uint64(1) << 32) revert RegistryFull();
        index = uint32(nextIndex++);
        usedCommitments[msg.sender][commitment] = true;
        units[index] = Unit(msg.sender, commitment, 0, 0, UnitState.Active);
        totalDepositedWei += msg.value;
        heldBondWei += msg.value;
        updateLeaf(index, TreeNode(keccak256(abi.encode(LEAF_TAG, domain, index, msg.sender, commitment)), 1));
        emit Bonded(index, msg.sender, commitment, msg.value);
    }

    function requestExit(uint32 index) external sameChain {
        Unit storage entry = units[index];
        if (entry.owner != msg.sender) revert Unauthorized();
        if (entry.state != UnitState.Active) revert NotActive();
        entry.exitRequestedAt = position64(block.timestamp);
        entry.withdrawAfter = position64(block.timestamp + snapshotLeaseSeconds + maxObligationSeconds);
        entry.state = UnitState.Exiting;
        updateLeaf(index, TreeNode(emptyHashes[0], 0));
        emit ExitRequested(index, entry.exitRequestedAt, entry.withdrawAfter);
    }

    function withdraw(uint32 index, address payable recipient) external sameChain {
        Unit storage entry = units[index];
        if (entry.owner != msg.sender) revert Unauthorized();
        if (entry.state != UnitState.Exiting) revert NotExiting();
        if (block.timestamp < entry.withdrawAfter) revert BondLocked();
        if (recipient == address(0)) revert InvalidRecipient();
        entry.state = UnitState.Withdrawn;
        heldBondWei -= unitBondWei;
        totalWithdrawnWei += unitBondWei;
        (bool paid,) = recipient.call{value: unitBondWei}("");
        if (!paid) revert TransferFailed();
        emit Withdrawn(index, recipient, unitBondWei);
    }

    function sealEpoch() external sameChain returns (uint64 epoch) {
        epoch = currentEpoch();
        if (snapshots[epoch].count != 0) revert EpochAlreadySealed();
        (bytes32 root, uint64 count) = activeRoot();
        if (count == 0) revert EmptyRegistry();
        Snapshot memory frozen = Snapshot(
            domain,
            root,
            count,
            position64(block.timestamp),
            position64(block.number),
            position64(block.number + futureBlockDelay),
            position64(block.timestamp + snapshotLeaseSeconds),
            bytes32(0)
        );
        snapshots[epoch] = frozen;
        emit EpochSealed(
            epoch, root, count, frozen.sealedAt, frozen.sealedBlock, frozen.beaconBlock, frozen.admissionUntil
        );
    }

    function captureSeed(uint64 epoch) external sameChain returns (bytes32 seed) {
        Snapshot storage frozen = snapshots[epoch];
        if (frozen.count == 0) revert UnknownEpoch();
        if (frozen.seed != 0) return frozen.seed;
        if (block.number <= frozen.beaconBlock || block.number - frozen.beaconBlock > 256) revert BeaconUnavailable();
        bytes32 beacon = blockhash(frozen.beaconBlock);
        if (beacon == 0) revert BeaconUnavailable();
        seed = keccak256(abi.encode(SEED_TAG, domain, epoch, frozen.root, frozen.count, frozen.beaconBlock, beacon));
        frozen.seed = seed;
        emit SeedCaptured(epoch, seed);
    }

    function position64(uint256 value) private pure returns (uint64) {
        if (value > type(uint64).max) revert PositionOverflow();
        return uint64(value);
    }

    function treeNode(uint8 level, uint32 index) private view returns (TreeNode memory node) {
        node = nodes[(uint64(level) << 32) | index];
        if (node.count == 0) node.hash = emptyHashes[level];
    }

    function writeNode(uint8 level, uint32 index, TreeNode memory node) private {
        uint64 key = (uint64(level) << 32) | index;
        if (node.count == 0) delete nodes[key];
        else nodes[key] = node;
    }

    function updateLeaf(uint32 index, TreeNode memory node) private {
        writeNode(0, index, node);
        for (uint8 level; level < 32; ++level) {
            TreeNode memory sibling = treeNode(level, index ^ 1);
            (TreeNode memory left, TreeNode memory right) = index & 1 == 0 ? (node, sibling) : (sibling, node);
            node = TreeNode(
                keccak256(abi.encode(BRANCH_TAG, left.hash, left.count, right.hash, right.count)),
                left.count + right.count
            );
            index >>= 1;
            writeNode(level + 1, index, node);
        }
    }
}
