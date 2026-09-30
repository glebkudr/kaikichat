# The app is called Kaiki Chat, 2026-09-28

Branch `feature/v1-kaiki-name-20260928` on top of `implementation/v1` c9fcb43.

- Bundle: `productName` "Kaiki Chat"; `scripts/macos-release.sh` built
  `target/release/bundle/macos/Kaiki Chat.app`, ad-hoc signed, `codesign
  --verify --deep --strict` passed. `Info.plist`: `CFBundleName` and
  `CFBundleDisplayName` "Kaiki Chat", `CFBundleExecutable` `agentic-desktop`,
  `CFBundleIdentifier` `net.agenticinternet.desktop` (unchanged, so the data
  directory and existing profiles stay where they were).
- Window title and page title "Kaiki Chat"; the sidebar brand "Kaiki Chat"
  with the mark "k·c".
- Localization: the core screen's eyebrow and the first onboarding step read
  "KAIKI CHAT" in all twenty tables; no table says "Agentic Internet".
- Tests: frontend 69 (new: `tests/app-name.test.tsx`, and a table check in
  `tests/i18n.test.tsx`), `tsc`; Tauri commands 18; build evidence 20.
- Not rebuilt here: the Linux `.deb` (its package name follows the product
  name).

Screenshots from the component fixture (`tests/visual.html`, headless Chrome,
1280×800): [chat, dark, English](chat-dark-en.png),
[onboarding, light, English](onboarding-light-en.png),
[onboarding, dark, Russian](onboarding-dark-ru.png),
[onboarding, light, Arabic (RTL)](onboarding-light-ar.png).
