# EXTENDED — NEW in agent-attribution. Every landed test this change EXTENDS rather than
# replaces actually gained its extension. An aggregate `testcount` floor is satisfied by the
# NEW tests alone, so skipping every extension leaves the whole suite green; this check is what
# makes the fourteen modified tests real rather than aspirational.
#
# One line per pair: <file>:<test fn>:<token the extension must add>. The span is the named
# function's body, cut at the next line-anchored `    fn ` at the same indent, so a token
# appearing in a NEIGHBOURING test does not satisfy this one. Adding a pair is how a later
# change records an extension; deleting one is a deliberate act a reviewer can see.
[ -f src/ui/list.rs ] || { echo "EXTENDED FAIL: src/ui/list.rs missing" >&2; exit 1; }
[ -f src/ui/view.rs ] || { echo "EXTENDED FAIL: src/ui/view.rs missing" >&2; exit 1; }
PAIRS="${PAIRS:-src/ui/list.rs:active_row_grammar_at_38_and_58:badge
src/ui/list.rs:every_row_is_exactly_the_requested_width:badge
src/ui/list.rs:a_long_name_is_truncated_with_an_ellipsis:badge
src/ui/list.rs:refresh_and_change_problems_in_order:badge
src/ui/list.rs:the_four_message_states_are_distinct:badge
src/ui/view.rs:the_prompt_replaces_the_hints_while_filtering:unattributed
src/ui/view.rs:an_accepted_query_leads_the_hint_list:unattributed
src/ui/view.rs:agents_change_no_pixel:assert_ne!
src/ui/app.rs:no_action_mutates_changes:LaunchApply
src/ui/app.rs:quit_keys_and_their_near_misses:LaunchContinue
src/ui/app.rs:esc_dismisses_one_layer_at_a_time:launch
src/ui/app.rs:enter_and_esc_map_to_routes:launch
src/ui/app.rs:non_key_events_are_ignored:Paste("a"
src/ui/app.rs:dashboard_is_clone_and_eq_with_agents:launch
src/ui/app.rs:attribution_follows_adopt_by_name:panes
src/ui/app.rs:attribution_ignores_the_filter:panes
src/ui/view.rs:agents_change_no_pixel:g focus
src/ui/view.rs:the_unattributed_count_is_the_last_hint:a/c/s launch
src/ui/view.rs:the_count_drops_before_the_key_hints:g focus
src/ui/view.rs:the_count_survives_a_filter:g focus
src/ui/view.rs:the_count_is_reported_with_an_empty_list:reachable
src/ui/view.rs:frame_rows_at_60_and_120:reachable
src/ui/view.rs:footer_drops_whole_hints:reachable
src/ui/view.rs:an_unreachable_socket_renders_the_agentless_pane:launch
src/ui/view.rs:a_prompt_longer_than_the_footer_keeps_its_tail:reachable
src/ui/view.rs:one_column_frame_does_not_panic:reachable
src/ui/list.rs:refresh_and_change_problems_in_order:launch
src/ui/list.rs:refresh_problems_lead_the_rows:launch
src/ui/list.rs:row_order_is_problems_then_active_then_separator_then_archived:launch
src/ui/driver.rs:an_inert_live_tier_takes_nothing_and_requests_nothing:launch::none
src/ui/driver.rs:ctrl_c_ends_the_loop:launcher
src/ui/driver.rs:an_unreachable_socket_is_not_a_problem_row:launch.problems
src/ui/driver.rs:timeouts_are_not_events:launcher
src/ui/view.rs:the_prompt_replaces_the_hints_while_filtering:reachable
src/ui/view.rs:an_accepted_query_leads_the_hint_list:reachable
src/agents.rs:an_unmatched_in_scope_agent_is_counted:panes
src/agents.rs:the_name_tier_never_reads_the_kind:panes
src/agents.rs:a_terminal_title_attributes_nothing:panes
src/agents.rs:every_empty_input_is_total:panes
src/agents.rs:precedence_is_total_and_order_independent:panes
src/agents.rs:one_agent_per_change_keeps_its_status:panes
src/ui/mod.rs:the_real_wiring_polls_a_scratch_herdr:pane split
src/ui/mod.rs:a_polled_agent_reaches_a_rendered_badge:a/c/s launch
src/ui/mod.rs:an_unreachable_scratch_herdr_is_a_standalone_tui:launch
src/ui/mod.rs:no_repository_still_polls_for_agents:g focus
src/ui/mod.rs:tasks_tab_is_read_only:launcher}"
PAIRS="$PAIRS" python3 - <<'PY' || exit 1
import os, re, sys
pairs = [l for l in os.environ["PAIRS"].splitlines() if l.strip()]
if len(pairs) < 8:
    print(f"EXTENDED FAIL: only {len(pairs)} pairs given (expected >= 8) - the list was gutted",
          file=sys.stderr); sys.exit(1)
bad, missing_fn = [], []
for line in pairs:
    path, fn, token = line.split(":", 2)
    src = open(path).read()
    m = re.search(r"(?m)^([ \t]*)fn %s\(" % re.escape(fn), src)
    if not m:
        missing_fn.append(f"{path}::{fn}"); continue
    indent = m.group(1)
    rest = src[m.end():]
    nxt = re.search(r"(?m)^%sfn [a-z0-9_]+\(" % re.escape(indent), rest)
    body = rest[: nxt.start()] if nxt else rest
    if token not in body:
        bad.append(f"{path}::{fn} does not name {token!r}")
if missing_fn:
    print("EXTENDED FAIL: named test not found (renamed or deleted): " + ", ".join(missing_fn),
          file=sys.stderr); sys.exit(1)
if bad:
    print("EXTENDED FAIL: a landed test was not extended:\n  " + "\n  ".join(bad),
          file=sys.stderr); sys.exit(1)
print(f"EXTENDED OK: all {len(pairs)} landed tests carry their extension token")
PY
