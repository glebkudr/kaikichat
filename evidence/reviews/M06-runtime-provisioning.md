# Owner runtime provisioning review

Separate backend-test-critic /root/node_test_critic, originally without context fork. Tests preceded production: missing ProvisionRuntimeRequest/provision_runtime/list_runtimes API compilation RED; both process scenarios RED on missing owner method.

Core review accepted three tests with real MLS, encrypted persisted states and SQL trigger failure. Node REVISE requested canonical /private/tmp paths, an explicitly private symlink target, and syntactically valid values for forbidden principal/seed/path/command inputs. All corrected; signed list_runtimes uses valid empty arguments. Final ACCEPT before production.

Implementation reuses grant preparation and ProfileStore.commit_states, existing proof broker/revoke, official MCP Credentials schema. Core projection never serializes ProvisionedRuntime seed. Node credentials are private fixed-path files and can be restored from encrypted authority state.

Validation: all three new core and all six MCP process tests pass. Full scripts/check.sh passes: 143 Rust +15 frontend tests, strict formatting/Clippy/TypeScript, production UI build. No native panel or packaged MCP E2E readiness claim at this stage.
