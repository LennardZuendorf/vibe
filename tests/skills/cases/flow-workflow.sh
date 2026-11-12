# shellcheck shell=bash
# flow-workflow Tasks 1–4: discovery and installed resource contracts.
_fw_out="$(node "$REPO_ROOT/flow/tests/evals/contracts.mjs" 2>&1)"; _fw_rc=$?
assert_rc "flow-workflow/1-4" "toolkit, Flow, installed procedures and scenarios ${_fw_out:+— $_fw_out}" 0 "$_fw_rc"

for _fw_test in migration.test.mjs legacy-writers.test.mjs adopted-hooks.test.mjs installer-runtime.test.mjs; do
  _fw_out="$(cd "$REPO_ROOT" && node "flow/tests/evals/$_fw_test" 2>&1)"; _fw_rc=$?
  assert_rc "flow-workflow/3" "$_fw_test ${_fw_out:+— $_fw_out}" 0 "$_fw_rc"
done
