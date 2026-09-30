// Independent test fixture generator. No production Rust code is imported.
import { createPrivateKey, createPublicKey, createHash, sign, verify } from 'node:crypto';
import { writeFileSync } from 'node:fs';
import assert from 'node:assert/strict';

const seed = '9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60';
const key = createPrivateKey({ key: Buffer.from(`302e020100300506032b657004220420${seed}`, 'hex'), format: 'der', type: 'pkcs8' });
const publicKey = createPublicKey(key);
const author = publicKey.export({ format: 'der', type: 'spki' }).subarray(-32).toString('hex');
assert.equal(author, 'd75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a');
const prefix = Buffer.from('AgenticInternet/signed-document/v1\0', 'utf8');
// [h'00' * 32, 1, 3, author, 0, 100, 200, h'68656c6c6f', {}]
const unsignedHex = `895820${'00'.repeat(32)}01035820${author}00186418c84568656c6c6fa0`;
function head(major, number) {
  const n = BigInt(number);
  if (n < 24n) return Buffer.from([(major << 5) | Number(n)]);
  const width = n <= 255n ? 1 : n <= 65535n ? 2 : n <= 4294967295n ? 4 : 8;
  const buffer = Buffer.alloc(1 + width);
  buffer[0] = (major << 5) | ({ 1: 24, 2: 25, 4: 26, 8: 27 })[width];
  if (width === 8) buffer.writeBigUInt64BE(n, 1); else buffer.writeUIntBE(Number(n), 1, width);
  return buffer;
}
function byteString(data) { return Buffer.concat([head(2, data.length), data]); }
function wrap(unsigned, signature) {
  return Buffer.concat([Buffer.from([0x82]), byteString(unsigned), byteString(signature)]);
}
function record(bytes, signature, wire = wrap(bytes, signature)) {
  return { unsigned_hex: bytes.toString('hex'), signature_hex: signature.toString('hex'), wire_hex: wire.toString('hex'), id_hex: createHash('sha256').update(wire).digest('hex') };
}
function signed(bytes) {
  const signature = sign(null, Buffer.concat([prefix, bytes]), key);
  assert(verify(null, Buffer.concat([prefix, bytes]), publicKey, signature));
  return record(bytes, signature);
}
const unsigned = Buffer.from(unsignedHex, 'hex');
const nonminimal = Buffer.concat([unsigned.subarray(0, 35), Buffer.from([0x18, 0x01]), unsigned.subarray(36)]);
const futureVersion = Buffer.from(unsigned); futureVersion[35] = 2;
const unknownKind = Buffer.concat([unsigned.subarray(0, 36), Buffer.from([0x18, 0x63]), unsigned.subarray(37)]);
const duplicateExtension = Buffer.concat([unsigned.subarray(0, -1), Buffer.from('a2014161014162', 'hex')]);
const criticalExtension = Buffer.concat([unsigned.subarray(0, -1), Buffer.from('a11980004178', 'hex')]);
const invalidLifetime = Buffer.from(unsigned); invalidLifetime[75] = 100;
const oversizedBody = Buffer.concat([unsigned.subarray(0, 76), Buffer.from([0x59, 0xc0, 0x01]), Buffer.alloc(49_153, 7), Buffer.from([0xa0])]);
const outOfOrderExtensions = Buffer.concat([unsigned.subarray(0, -1), Buffer.from('a2024162014161', 'hex')]);
const indefiniteInner = Buffer.concat([Buffer.from([0x9f]), unsigned.subarray(1), Buffer.from([0xff])]);
function extensions(count, valueSize) {
  return Buffer.concat([unsigned.subarray(0, -1), head(5, count), ...Array.from({length:count}, (_, id) => Buffer.concat([head(0,id),byteString(Buffer.alloc(valueSize, id))]))]);
}
const modern = Buffer.concat([unsigned.subarray(0, 71), head(0, 7), head(0, 1_788_480_000), head(0, 1_788_483_600), unsigned.subarray(76)]);
const variants = Object.fromEntries(Object.entries({ nonminimal_version: nonminimal, future_version: futureVersion, unknown_kind: unknownKind, duplicate_extension: duplicateExtension, critical_extension: criticalExtension, invalid_lifetime: invalidLifetime, oversized_body: oversizedBody, out_of_order_extensions:outOfOrderExtensions, indefinite_inner:indefiniteInner, extensions_at_limit:extensions(16,1024), too_many_extensions:extensions(17,1), oversized_extension:extensions(1,1025), modern_epoch:modern }).map(([name, bytes]) => [name, signed(bytes)]));
const baseSignature=Buffer.from(signed(unsigned).signature_hex,'hex');
const baseWire=wrap(unsigned,baseSignature);
const signatureOffset=3+unsigned.length;
for (const [name, wire] of Object.entries({
  nonminimal_outer_array:Buffer.concat([Buffer.from([0x98,0x02]),baseWire.subarray(1)]),
  nonminimal_outer_unsigned_length:Buffer.concat([Buffer.from([0x82,0x59,0x00,unsigned.length]),baseWire.subarray(3)]),
  nonminimal_outer_signature_length:Buffer.concat([baseWire.subarray(0,signatureOffset),Buffer.from([0x59,0x00,0x40]),baseSignature]),
  indefinite_outer:Buffer.concat([Buffer.from([0x9f]),baseWire.subarray(1),Buffer.from([0xff])])
})) variants[name]=record(unsigned,baseSignature,wire);
// Identity-point public key and R, S=0: accepted by lax Ed25519, invalid under strict verification.
const weakUnsigned=Buffer.from(unsigned); weakUnsigned.fill(0,39,71); weakUnsigned[39]=1;
const weakSignature=Buffer.alloc(64); weakSignature[0]=1;
variants.weak_key_forgery=record(weakUnsigned,weakSignature);
const fixture = { provenance: 'Literal RFC 8949 CBOR and independent Node/OpenSSL Ed25519; RFC 8032 public test seed. No Rust code.', seed_hex: seed, author_hex: author, ...signed(unsigned), variants };
writeFileSync(new URL('./signed-document-v1.json', import.meta.url), `${JSON.stringify(fixture, null, 2)}\n`);
