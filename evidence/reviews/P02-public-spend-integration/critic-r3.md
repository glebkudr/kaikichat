# Independent critic R3 — FINAL ACCEPT

Only support.rs changed: the explicit path resolves the same helper when imported
and when compiled as a standalone integration target. Removing the attribute
reproduces the exact R2 hash. All R3 hashes match; assertions and scenarios are
unchanged and no bypass was introduced. The critic ran no builds and changed no
files. Workspace validation resumes against the frozen R3 inputs.
