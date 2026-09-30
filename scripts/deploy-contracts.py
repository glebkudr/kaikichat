#!/usr/bin/env python3
"""Deploy the V1 contracts to a chain and write the deployment manifest.

GrantIssuer and NodeRegistry, then RoyaltySplitter (treasury 1/10, operator
pool 9/10), BookShop and OperatorPool (Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md)
with the testnet parameters below, then every immutable is read back and
checked. The splitter pays the pool two deployments ahead, at the address the
deployer's nonce gives it. The manifest holds the addresses, the parameters
and the flags a node joins with (`kaiki daemon start …`).

With `--reuse MANIFEST` the GrantIssuer and NodeRegistry of that deployment
stay and only the splitter, the shop and the pool are new; the manifest
written lists the shop it replaces.

The deployer's key is read from a file and handed to forge/cast as an
argument (foundry takes no key from the environment); nothing prints it.
Run through the build wrapper, which puts forge and cast on PATH:

    python3 scripts/build-storage.py run python3 scripts/deploy-contracts.py \
        --rpc-url https://sepolia.base.org --chain-id 84532 \
        --key-file .local/testnet/deployer.key \
        --usdc 0x036CbD53842c5426634e7929541eC2318f3dCF7e \
        --eth-usd-feed 0x4aDC67696bA383F43DD60A9e78F2C97Fbbfc7cb1 \
        --issuer 0x… --out deployments/base-sepolia.json

Base mainnet takes its own registry genesis, so its units never pass for the
testnet's: `--rpc-url https://mainnet.base.org --chain-id 8453
--registry-genesis agentic-internet-base-v1 --usdc
0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 --eth-usd-feed
0x71041dddad3595F9CEd3DcCFBe3D1F4b0a16Bb70 --key-file
.local/mainnet/deployer.key --out deployments/base.json`.
"""
import argparse
import json
import os
import pathlib
import subprocess
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parent.parent
CONTRACTS = ROOT / "contracts"

# `agentic_node::NETWORK_DOMAIN`.
DOMAIN = "0xae2e3182ade817a3e726c29ef308eed15c6ec267ffc01a68da990e576fa0df18"

# The network's parameters: immutable; changing one means a new deployment.
PARAMS = {
    # Treasury and operator pool, in basis points.
    "royaltyBps": [1000, 9000],
    # A book of 1000 stamps for $1.00 (USDC units): a tenth of a cent a
    # message, paid in USDC or in ETH at the feed's rate.
    "bookPriceUsdc": 1_000_000,
    "bookSize": 1000,
    # ETH purchases need a feed answer at most an hour old.
    "maxPriceAgeSeconds": 3600,
    "bookValiditySeconds": 30 * 86400,
    # Free coins: a grant book of 10000 (the app's "up to 10 thousand
    # messages every month"), valid up to 30 days; 100000000 coins a day,
    # ten thousand grants.
    "grantBookSize": 10_000,
    "grantMaxValidityDays": 30,
    "grantDailyCapCoins": 100_000_000,
    # Operator prizes: $0.10 to each named holder of a winning paid stamp,
    # claimable for 360 days from the book's purchase; a day's seed is the
    # hash of a block five after arming.
    "prizeUsdc": 100_000,
    "ticketLifetimeSeconds": 360 * 86400,
    "seedDelayBlocks": 5,
    # Holder units: 0.0001 ETH bond; hourly epochs; an exit waits 7 days.
    "registryGenesis": "agentic-internet-testnet-v1",
    "unitBondWei": 100_000_000_000_000,
    "epochSeconds": 3600,
    "admissionLeaseSeconds": 7200,
    "obligationSeconds": 7 * 86400,
    "beaconDelayBlocks": 5,
}


def run(*args: str, secret: str | None = None) -> str:
    """A command's stdout; on failure its stderr, with any secret masked."""
    done = subprocess.run(args, capture_output=True, text=True, cwd=ROOT)
    if done.returncode != 0:
        message = done.stderr.strip() or done.stdout.strip()
        if secret:
            message = message.replace(secret, "<key>")
        shown = " ".join("<key>" if a == secret else a for a in args[:3])
        sys.exit(f"{shown}… failed: {message}")
    return done.stdout.strip()


def cast_call(rpc: str, address: str, signature: str, *args: str) -> str:
    return run("cast", "call", address, signature, *args, "--rpc-url", rpc)


class Nonces:
    """The deployer's nonces, counted here: a public RPC behind a balancer
    may still answer an older one right after a transaction."""

    def __init__(self, rpc: str, account: str):
        self.next = int(run("cast", "nonce", account, "--block", "pending", "--rpc-url", rpc))

    def take(self) -> str:
        self.next += 1
        return str(self.next - 1)


def deploy(rpc: str, key: str, nonces: Nonces, contract: str, *arguments: str) -> dict:
    command = [
        "forge", "create", "--root", str(CONTRACTS), "--rpc-url", rpc,
        "--private-key", key, "--broadcast", "--nonce", nonces.take(),
    ]
    if os.environ.get("AIN_SOLC"):
        command += ["--use", os.environ["AIN_SOLC"]]
    command.append(contract)
    if arguments:
        command += ["--constructor-args", *arguments]
    output = run(*command, secret=key)
    fields = {}
    for line in output.splitlines():
        for label, name in (("Deployed to: ", "address"), ("Transaction hash: ", "tx")):
            if line.startswith(label):
                fields[name] = line[len(label):].strip()
    if "address" not in fields:
        sys.exit(f"{contract}: no address in forge output")
    print(f"{contract.split(':')[-1]}: {fields['address']}", flush=True)
    return fields


def check(what: str, actual: str, expected: str) -> None:
    # cast appends a number's short form: "2592000 [2.592e6]".
    actual = actual.split()[0] if actual else actual
    if actual.lower() != expected.lower():
        sys.exit(f"{what}: {actual} on chain, {expected} expected")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--rpc-url", required=True)
    parser.add_argument("--chain-id", type=int, required=True)
    parser.add_argument("--key-file", required=True, type=pathlib.Path)
    parser.add_argument("--treasury", help="treasury share; default: the deployer")
    parser.add_argument("--issuer", help="identity server's first key (not with --reuse)")
    parser.add_argument("--reuse", type=pathlib.Path,
                        help="keep this deployment's GrantIssuer and NodeRegistry")
    parser.add_argument("--usdc", required=True, help="the USDC token")
    parser.add_argument("--eth-usd-feed", required=True, help="Chainlink ETH/USD feed")
    parser.add_argument("--confirmations", type=int, default=5)
    parser.add_argument("--registry-genesis", default=PARAMS["registryGenesis"],
                        help="names the network's NodeRegistry (default: the testnet's)")
    parser.add_argument("--out", required=True, type=pathlib.Path)
    options = parser.parse_args()
    rpc = options.rpc_url

    if options.out.exists():
        sys.exit(f"{options.out} exists: a deployment is immutable, use another path")
    key = options.key_file.read_text().strip()
    if not key.startswith("0x"):
        key = "0x" + key
    if len(key) != 66 or any(c not in "0123456789abcdefABCDEF" for c in key[2:]):
        sys.exit(f"{options.key_file} must hold one 32-byte hex private key")
    chain = int(run("cast", "chain-id", "--rpc-url", rpc))
    if chain != options.chain_id:
        sys.exit(f"the RPC serves chain {chain}, not {options.chain_id}")
    deployer = run("cast", "wallet", "address", "--private-key", key, secret=key)
    balance = run("cast", "balance", "--ether", deployer, "--rpc-url", rpc)
    print(f"deployer {deployer} on chain {chain}: {balance} ETH", flush=True)
    treasury = options.treasury or deployer
    p = dict(PARAMS, registryGenesis=options.registry_genesis)
    nonces = Nonces(rpc, deployer)

    previous = json.loads(options.reuse.read_text()) if options.reuse else None
    if previous:
        if previous["chainId"] != chain or previous["domain"] != DOMAIN:
            sys.exit(f"{options.reuse} is another chain or network")
        issuer = previous["contracts"]["grantIssuer"]
        registry = previous["contracts"]["nodeRegistry"]
        genesis = previous["registryGenesisDigest"]
        first_issuer = previous["issuer"]
        for kept in ("grantBookSize", "grantMaxValidityDays", "grantDailyCapCoins", "registryGenesis",
                     "unitBondWei", "epochSeconds", "admissionLeaseSeconds", "obligationSeconds",
                     "beaconDelayBlocks"):
            if previous["params"][kept] != p[kept]:
                sys.exit(f"{kept}: {previous['params'][kept]} deployed, {p[kept]} here")
    else:
        if not options.issuer:
            sys.exit("--issuer is needed for a new GrantIssuer")
        first_issuer = options.issuer
        issuer = deploy(
            rpc, key, nonces, "src/GrantIssuer.sol:GrantIssuer",
            deployer, DOMAIN, str(p["grantBookSize"]), str(p["grantMaxValidityDays"]),
            str(p["grantDailyCapCoins"]), options.issuer,
        )
        genesis = run("cast", "keccak", p["registryGenesis"])
        registry = deploy(
            rpc, key, nonces, "src/NodeRegistry.sol:NodeRegistry",
            genesis, str(p["unitBondWei"]), str(p["epochSeconds"]),
            str(p["admissionLeaseSeconds"]), str(p["obligationSeconds"]),
            str(p["beaconDelayBlocks"]),
        )
    # The splitter, the shop and the pool take three nonces in a row.
    pool_nonce = nonces.next + 2
    predicted = run("cast", "compute-address", deployer, "--nonce", str(pool_nonce)).split()[-1]
    first_day = int(run("cast", "block", "latest", "--field", "timestamp", "--rpc-url", rpc)) // 86400
    splitter = deploy(
        rpc, key, nonces, "src/RoyaltySplitter.sol:RoyaltySplitter",
        f"[{treasury},{predicted}]", f"[{p['royaltyBps'][0]},{p['royaltyBps'][1]}]",
    )
    shop = deploy(
        rpc, key, nonces, "src/BookShop.sol:BookShop",
        DOMAIN, str(p["bookPriceUsdc"]), str(p["bookSize"]), str(p["bookValiditySeconds"]),
        splitter["address"], options.usdc, options.eth_usd_feed, str(p["maxPriceAgeSeconds"]),
    )
    pool = deploy(
        rpc, key, nonces, "src/OperatorPool.sol:OperatorPool",
        shop["address"], registry["address"], treasury, str(p["prizeUsdc"]),
        str(p["ticketLifetimeSeconds"]), str(p["seedDelayBlocks"]), str(first_day),
    )
    if pool["address"].lower() != predicted.lower():
        sys.exit(f"the pool landed at {pool['address']}, not {predicted}: the splitter pays nobody")

    s, b, g, r, o = (splitter["address"], shop["address"], issuer["address"],
                     registry["address"], pool["address"])
    flags = [
        "--chain-rpc", rpc, "--chain-id", str(chain), "--book-shop", b,
        "--grant-issuer", g, "--registry", r,
        "--chain-confirmations", str(options.confirmations),
        "--operator-pool", o,
    ]
    manifest = {
        "chainId": chain,
        "rpc": rpc,
        "confirmations": options.confirmations,
        "domain": DOMAIN,
        "deployer": deployer,
        "treasury": treasury,
        "pool": o,
        "issuer": first_issuer,
        "usdc": options.usdc,
        "ethUsdFeed": options.eth_usd_feed,
        "contracts": {
            "royaltySplitter": splitter,
            "bookShop": shop,
            "grantIssuer": issuer,
            "nodeRegistry": registry,
            "operatorPool": pool,
        },
        "registryGenesisDigest": genesis,
        "poolFirstDay": first_day,
        "params": p,
        "nodeFlags": flags,
        "verified": False,
    }
    if previous:
        for kept in ("supersededGrantIssuers", "supersededShops"):
            if previous.get(kept):
                manifest[kept] = list(previous[kept])
        manifest.setdefault("supersededShops", []).append({
            "bookShop": previous["contracts"]["bookShop"]["address"],
            "royaltySplitter": previous["contracts"]["royaltySplitter"]["address"],
            "pool": previous["pool"],
            "replaced": time.strftime("%Y-%m-%d", time.gmtime()),
            "why": "the operator share goes to OperatorPool (Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md)",
        })
    options.out.parent.mkdir(parents=True, exist_ok=True)
    options.out.write_text(json.dumps(manifest, indent=2) + "\n")
    # Read every immutable back (a public RPC may lag a block behind).
    time.sleep(4)
    for index, (account, share) in enumerate(zip([treasury, o], p["royaltyBps"])):
        got = cast_call(rpc, s, "recipient(uint256)(address,uint16)", str(index)).split()
        check(f"splitter recipient {index}", got[0], account)
        check(f"splitter share {index}", got[1], str(share))
    check("shop domain", cast_call(rpc, b, "domain()(bytes32)"), DOMAIN)
    check("shop price", cast_call(rpc, b, "priceUsdc()(uint256)"), str(p["bookPriceUsdc"]))
    check("shop usdc", cast_call(rpc, b, "usdc()(address)"), options.usdc)
    check("shop feed", cast_call(rpc, b, "ethUsd()(address)"), options.eth_usd_feed)
    check("shop price age", cast_call(rpc, b, "maxPriceAge()(uint32)"), str(p["maxPriceAgeSeconds"]))
    quote = cast_call(rpc, b, "quote()(uint256)").split()[0]
    print(f"a book costs {int(quote) / 1e18:.8f} ETH at the feed's rate", flush=True)
    check("shop book size", cast_call(rpc, b, "bookSize()(uint32)"), str(p["bookSize"]))
    check("shop validity", cast_call(rpc, b, "validity()(uint64)"), str(p["bookValiditySeconds"]))
    check("shop splitter", cast_call(rpc, b, "splitter()(address)"), s)
    check("pool shop", cast_call(rpc, o, "shop()(address)"), b)
    check("pool registry", cast_call(rpc, o, "registry()(address)"), r)
    check("pool treasury", cast_call(rpc, o, "treasury()(address)"), treasury)
    check("pool prize", cast_call(rpc, o, "prizeUsdc()(uint256)"), str(p["prizeUsdc"]))
    check("pool lifetime", cast_call(rpc, o, "ticketLifetime()(uint64)"), str(p["ticketLifetimeSeconds"]))
    check("pool seed delay", cast_call(rpc, o, "seedDelay()(uint16)"), str(p["seedDelayBlocks"]))
    check("pool first day", cast_call(rpc, o, "firstDay()(uint64)"), str(first_day))
    maximum = 2**256 - 1
    odds = p["prizeUsdc"] * 10 * p["bookSize"] * 10_000
    threshold = (maximum // odds) * (p["bookPriceUsdc"] * p["royaltyBps"][1])
    check("pool threshold", cast_call(rpc, o, "winThreshold()(uint256)"), str(threshold))
    check("issuer domain", cast_call(rpc, g, "domain()(bytes32)"), DOMAIN)
    check("issuer book size", cast_call(rpc, g, "bookSize()(uint32)"), str(p["grantBookSize"]))
    today = cast_call(rpc, g, "today()(uint64)").split()[0]
    check("daily cap", cast_call(rpc, g, "capForDay(uint64)(uint64)", today), str(p["grantDailyCapCoins"]))
    if not previous:
        check("issuer active", cast_call(rpc, g, "issuerActiveOn(address,uint64)(bool)", first_issuer, today), "true")
    check("unit bond", cast_call(rpc, r, "unitBondWei()(uint128)"), str(p["unitBondWei"]))

    manifest["verified"] = True
    options.out.write_text(json.dumps(manifest, indent=2) + "\n")
    left = run("cast", "balance", "--ether", deployer, "--rpc-url", rpc)
    print(f"manifest {options.out}; deployer has {left} ETH left")


if __name__ == "__main__":
    main()
