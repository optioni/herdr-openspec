# TESTCOUNT — a `cargo test` filter that matches NOTHING exits 0. Verified in a previous
# change: `cargo test --all-features this_name_does_not_exist` prints
# "0 passed; ... filtered out" and returns 0. Every VERIFY that is a filtered run therefore
# passes if the module is renamed, if the tests are never written, or if the filter has a
# typo. The gate is a counted minimum taken from the group's own RED.
#
# The SCOPE parameter is load-bearing: without it the helper sums every test binary, so a
# whole-suite gate passes against a lib baseline with zero new lib tests.
#
# The `[ $? -eq 0 ]` after the `case` reads cargo's status, not the case's: the branch's
# last command is an assignment whose value comes from a command substitution, and both
# `case` and the assignment propagate that status. Re-verified under /bin/sh, bash, zsh, and
# dash during this change's planning review.
#
# This block DEFINES a shell function; it must be SOURCED (`. $CHECKS/TESTCOUNT.sh`) in
# every shell that runs a gate, not executed with `sh`.
#   usage: testcount <scope> <filter> <minimum>   scope: --lib | --test-cli
# Carried forward from list-view BYTE-IDENTICALLY.
testcount() {
  scope=$1; filter=$2; min=$3
  case "$scope" in
    --lib)      out=$(cargo test --all-features --lib "$filter" 2>&1) ;;
    --test-cli) out=$(cargo test --all-features --test cli 2>&1) ;;
    *) echo "TESTCOUNT FAIL: bad scope '$scope'" >&2; return 1 ;;
  esac
  [ $? -eq 0 ] || { printf '%s\n' "$out" >&2; return 1; }
  n=$(printf '%s\n' "$out" | sed -n 's/^test result: ok\. \([0-9][0-9]*\) passed.*/\1/p' \
      | awk '{t+=$1} END{print t+0}')
  [ "$n" -ge "$min" ] || {
    echo "TESTCOUNT FAIL: $scope filter '$filter' ran $n tests, expected >= $min" >&2
    return 1; }
  echo "TESTCOUNT OK: $scope filter '$filter' ran $n tests (>= $min)"
}
