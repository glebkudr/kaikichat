// Keychain identity probe: `probe write|read KEYCHAIN PASSWORD ACCOUNT [SECRET]`.
// Works on one explicitly opened keychain file, never the login keychain, and
// with user interaction off, so macOS shows no dialog: a read the item's
// access list does not allow answers `refused -25293` (errSecAuthFailed)
// where the dialog would have asked; -25308 (errSecInteractionNotAllowed)
// would mean a locked keychain. Uses the same legacy calls as the keyring
// crate (find/add generic password).
import Foundation
import Security

#if VARIANT_B
let build = "b"
#else
let build = "a"
#endif
let service = "net.agenticinternet.keychain-probe"
let args = CommandLine.arguments
guard args.count >= 5 else { print("usage"); exit(2) }
guard SecKeychainSetUserInteractionAllowed(false) == errSecSuccess else { print("no-interaction-off"); exit(3) }
var opened: SecKeychain?
var status = SecKeychainOpen(args[2], &opened)
guard status == errSecSuccess, let keychain = opened else { print("open-error \(status)"); exit(3) }
let password = args[3]
status = SecKeychainUnlock(keychain, UInt32(password.utf8.count), password, true)
guard status == errSecSuccess else { print("unlock-error \(status)"); exit(3) }
let account = args[4]
switch args[1] {
case "write":
    let secret = args[5]
    status = SecKeychainAddGenericPassword(
        keychain, UInt32(service.utf8.count), service, UInt32(account.utf8.count), account,
        UInt32(secret.utf8.count), secret, nil)
    print(status == errSecSuccess ? "written(\(build))" : "write-error \(status)")
case "read":
    var length: UInt32 = 0
    var data: UnsafeMutableRawPointer?
    status = SecKeychainFindGenericPassword(
        keychain, UInt32(service.utf8.count), service, UInt32(account.utf8.count), account,
        &length, &data, nil)
    if status == errSecSuccess, let data {
        let secret = String(decoding: Data(bytes: data, count: Int(length)), as: UTF8.self)
        SecKeychainItemFreeContent(nil, data)
        print("read(\(build)) \(secret)")
    } else {
        print("refused(\(build)) \(status)")
    }
default:
    print("usage"); exit(2)
}
