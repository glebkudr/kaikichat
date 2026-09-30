// SPDX-License-Identifier: MIT
pragma solidity 0.8.36;

import {RoyaltySplitter} from "../src/RoyaltySplitter.sol";
import {MockUsdc} from "./PaymentMocks.sol";

interface VmRoyalty {
    function addr(uint256 privateKey) external returns (address);
    function deal(address account, uint256 amount) external;
    function expectRevert(bytes4 selector) external;
    function prank(address sender) external;
    function warp(uint256 timestamp) external;
    function chainId(uint256 id) external;
}

contract RoyaltySplitterTest {
    VmRoyalty internal constant vm = VmRoyalty(address(uint160(uint256(keccak256("hevm cheat code")))));
    uint256 internal constant ALICE_KEY = 0xA11CE;
    uint256 internal constant BOB_KEY = 0xB0B;
    address internal alice;
    address internal bob;

    function setUp() public {
        vm.warp(1_800_000_000);
        vm.chainId(31_337);
        alice = vm.addr(ALICE_KEY);
        bob = vm.addr(BOB_KEY);
        vm.deal(alice, 1 ether);
        vm.deal(bob, 1 ether);
    }

    function testRoyaltySplitterConservesPullCreditsWithoutGovernanceSurface() public {
        address payable[] memory recipients = new address payable[](2);
        recipients[0] = payable(alice);
        recipients[1] = payable(bob);
        uint16[] memory shares = new uint16[](2);
        shares[0] = 2_500;
        shares[1] = 7_500;
        RoyaltySplitter splitter = new RoyaltySplitter(recipients, shares);

        splitter.deposit{value: 101}();
        require(splitter.totalReceivedWei() == 101 && splitter.totalCreditedWei() == 101, "royalty totals");
        require(splitter.credits(alice) == 25 && splitter.credits(bob) == 76, "royalty split");

        uint256 aliceBefore = alice.balance;
        vm.prank(alice);
        splitter.withdrawRoyalty(payable(alice));
        require(alice.balance == aliceBefore + 25 && splitter.credits(alice) == 0, "alice pull");
        vm.expectRevert(RoyaltySplitter.NoCredit.selector);
        vm.prank(alice);
        splitter.withdrawRoyalty(payable(alice));
        require(splitter.credits(bob) == 76, "bob credit changed by alice");
        (bool success,) = address(splitter).call(abi.encodeWithSignature("setOwner(address)", alice));
        require(!success && splitter.recipientCount() == 2, "governance surface");
    }

    /// A token (USDC) that reached the splitter is credited by the shares
    /// once; each recipient pulls its own credit.
    function testTokensThatArrivedAreCreditedOnceAndPulledByTheirRecipient() public {
        address payable[] memory recipients = new address payable[](2);
        recipients[0] = payable(alice);
        recipients[1] = payable(bob);
        uint16[] memory shares = new uint16[](2);
        shares[0] = 1_000;
        shares[1] = 9_000;
        RoyaltySplitter splitter = new RoyaltySplitter(recipients, shares);
        MockUsdc usdc = new MockUsdc();
        address token = address(usdc);

        usdc.mint(address(splitter), 1_000_000);
        splitter.depositToken(token);
        require(splitter.tokenCredits(token, alice) == 100_000, "alice share");
        require(splitter.tokenCredits(token, bob) == 900_000, "bob share");
        // Nothing new arrived: nothing to credit twice.
        vm.expectRevert(RoyaltySplitter.InvalidAmount.selector);
        splitter.depositToken(token);

        vm.prank(alice);
        splitter.withdrawTokenRoyalty(token, alice);
        require(usdc.balanceOf(alice) == 100_000 && splitter.tokenCredits(token, alice) == 0, "alice pull");
        vm.expectRevert(RoyaltySplitter.NoCredit.selector);
        vm.prank(alice);
        splitter.withdrawTokenRoyalty(token, alice);

        // What Alice took is not credited again; the next arrival is.
        usdc.mint(address(splitter), 10);
        splitter.depositToken(token);
        require(splitter.tokenCredits(token, alice) == 1, "alice share of the next deposit");
        require(splitter.tokenCredits(token, bob) == 900_009, "bob keeps his credit");
        require(splitter.credits(bob) == 0, "no ETH credited");
    }
}
