#!/usr/bin/env bash
# install.sh — install the vibe workflow harness. One command, two modes.
#
#   ./install.sh                 # single command: default to THIS repo; on a TTY,
#                                #   ask local vs global first.
#   ./install.sh <repo>          # local: full vibe into <repo> (explicit target).
#   ./install.sh --global        # per-user install: spec + flow + vibe into
#                                #   every detected host's user-level skills dir (Claude,
#                                #   Codex, Droid, OpenCode) + one shared user-level
#                                #   AGENTS.md block.
#   ./install.sh --with-plugins  # also offer companion plugins (superpowers, …).
#
# No checkout required — run straight from the web (installs into the current repo):
#   curl -fsSL https://raw.githubusercontent.com/LennardZuendorf/vibe/main/install.sh | bash
# Piped from curl (no skill bundle beside the script) it fetches a repo snapshot
# first (override with VIBE_REPO / VIBE_REF), then runs the local install.
#
# LOCAL install (copy the platform-neutral core; merge the instruction file with
# markers; never blind-overwrite user content):
#   1. Copy the core   .agents/skills/{spec,flow,vibe} (skills + vibe flow engine).
#   2. Copy the Claude adapter   .claude/commands + .claude/hooks/*.sh scripts.
#   3. Wire the flow hooks into   .claude/settings.json (merge, never clobber).
#   3b. Deploy the SELECTED native host surfaces (--platform, else the host CLIs
#       found on PATH): .opencode/commands/flow.md (OpenCode), plugins/vibe +
#       .agents/plugins/marketplace.json (Codex), factory-plugin +
#       .factory-plugin/marketplace.json (Factory Droid). No hook integration and
#       no agent/droid registration is added for those hosts.
#   3c. Record every created host artifact in .vibe/install-manifest.json — the
#       ownership record re-install preserves and uninstall consults.
#   4. Seed   .agents/skills/flow/state.json from state.example.json if absent.
#   5. Ignore the mutable cursor in the target .gitignore.
#   6. Merge   AGENTS.md via the target's merge-agents.sh (agent-instructions).
#   7. Symlink requested adapters (CLAUDE.md, WARP.md) — opt-in, never clobber.
#   8. Report the configured hosts + entry points, that hooks are live (Claude
#      Code only, and only when wired), and that /flow is a native command.
#
# GLOBAL install places the three shipped skills (`spec`, `using-spec`, `vibe`) —
# each its OWN sibling skill dir, never nested one inside another (vibe#71
# sub-gap 3: OpenCode documents `skills/*/SKILL.md`) — in each selected host's
# OWN user-level skills directory, so a user of Codex, Droid or OpenCode no longer
# has to hand-copy trees out of `git archive` (vibe#71), and maintains ONE shared
# user-level AGENTS.md block (tracked template, marker-delimited, appended once).
# No host CLI is required: the payload is files. The optional Claude plugin
# (`--plugin`) stays available for the per-user doctrine hook, but is off by
# default — it would load the same skills a second time. The full STATEFUL flow
# (cursor, /flow, hooks) is a per-repo `--local` install.
#
# The `vibe-spec` BINARY is NOT part of a plain install (vibe#71 sub-gap 4): the
# `spec` skill shells out to it, and no release artifact exists for it, so a
# global install REPORTS its status (present / absent + what still works) instead
# of pretending the skill set is complete. `--build-spec-bin` opts into building
# it from this checkout with cargo and installing it into a user-owned bin dir.
#
# Flags:
#   --local            full per-repo install into the target (the default).
#   --global           per-user install: spec + flow + vibe into each
#                      selected host's user-level skills dir, plus the shared
#                      AGENTS.md block.
#   --host LIST        hosts for --global, comma-separated or `all`:
#                      claude, codex, droid, opencode. Default: detect a host when
#                      its CLI is on PATH or its user-level config dir exists.
#                      Same selector vocabulary as --platform (last one wins).
#   --plugin           (with --global) ALSO install the Claude plugin at user scope
#                      for its doctrine hook. Needs the `claude` CLI.
#   --build-spec-bin   (with --global) build the `vibe-spec` binary from this
#                      checkout (needs cargo) and install it into --bin-dir.
#   --bin-dir DIR      where --build-spec-bin puts `vibe-spec`
#                      (default: $HOME/.local/bin).
#   --with-plugins     also install companion plugins (superpowers; feature-dev slot).
#   --only spec|flow   install (or uninstall) a single half (default: both).
#   --platform LIST    native host surfaces to configure, comma-separated:
#                      claude, opencode, codex, droid, none (the LAST --platform
#                      wins). Default: detect those four CLIs on PATH and
#                      configure only the ones found; with none found, host-neutral
#                      files only. An explicit LIST overrides detection; `none` is
#                      host-neutral only and cannot be combined with a host.
#   --dry-run          print the action plan and write nothing.
#   --uninstall        remove managed artifacts; preserve docs/spec/ (and a legacy spec tree), user AGENTS.md
#                      prose, and the flow cursor (cursor removed only with --yes).
#                      Inverts vibe-owned surfaces for every host, reading
#                      .vibe/install-manifest.json (falling back to the shipped
#                      source for targets installed before the manifest existed).
#   --yes, -y          assume yes for the cursor-removal confirm on --uninstall.
#
# Re-runnable: refreshes the managed core (.agents/**, .claude/** adapter) from
# source each run, preserves the target's live flow cursor (state.json) and any
# content outside the managed AGENTS.md markers. Managed core files are owned by
# vibe — local edits to them are replaced on re-run.

set -euo pipefail

SRC="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" 2>/dev/null && pwd -P)" || SRC=""
# The legacy spec root name, kept as a value so prose and checks can name it.
LEGACY_SPEC=".spec" # literal

# ── network bootstrap ────────────────────────────────────────────────────────
# Run straight from the web (curl … | bash) with no checkout: the skill bundle
# is not beside this script, so fetch a snapshot of the repo and re-run the real
# installer from the extracted tree (which then resolves like a normal checkout).
# Skipped whenever the bundle IS present, so a local ./install.sh never downloads.
VIBE_REPO="${VIBE_REPO:-LennardZuendorf/vibe}"
VIBE_REF="${VIBE_REF:-main}"
if [[ -z "${SRC:-}" || ! -f "$SRC/flow/state-machine.json" || ! -f "$SRC/spec/SKILL.md" ]]; then
  tarball="https://codeload.github.com/$VIBE_REPO/tar.gz/$VIBE_REF"
  boot_tmp="$(mktemp -d "${TMPDIR:-/tmp}/vibe-boot.XXXXXX")"
  trap 'rm -rf "$boot_tmp"' EXIT
  echo "install: fetching vibe ($VIBE_REPO@$VIBE_REF) …" >&2
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$tarball" | tar -xzf - -C "$boot_tmp"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "$tarball" | tar -xzf - -C "$boot_tmp"
  else
    echo "install: ERROR: need curl or wget to install from the network." >&2
    exit 1
  fi
  boot_src="$(find "$boot_tmp" -mindepth 1 -maxdepth 1 -type d | head -n1)"
  if [[ -z "$boot_src" || ! -f "$boot_src/install.sh" ]]; then
    echo "install: ERROR: fetched archive is missing install.sh." >&2
    exit 1
  fi
  boot_rc=0
  # The re-exec must know it runs from a throwaway tree: the trap above deletes
  # $boot_tmp on exit, so --global cannot register a marketplace at "$SRC".
  VIBE_BOOTSTRAPPED=1 VIBE_REPO="$VIBE_REPO" VIBE_REF="$VIBE_REF" \
    bash "$boot_src/install.sh" "$@" || boot_rc=$?
  exit "$boot_rc"
fi

err() { echo "install: $1" >&2; }
note() { echo "install: $1"; }
# say — announce a mutating action. In dry-run every action is described but
# never performed, so the plan reads the same whether or not it is applied.
say() { if [[ "$DRY_RUN" -eq 1 ]]; then echo "install: [dry-run] would $1"; else echo "install: $1"; fi; }

# retire_vibe_plugin_manifest TARGET — remove a plugin manifest that a prior
# plugin-based vibe install left at `.claude-plugin/plugin.json`, together with
# the `.claude/hooks/hooks.json` that pairs with it.
#
# It is deleted ONLY when it is demonstrably vibe's (manifest `name` == "vibe").
# A repo may ship its OWN Claude plugin at those exact paths — indexed does, with
# an MCP server registration and its own PreToolUse hook — and the previous
# unconditional `rm -f` destroyed it on install. Absence of the file is not
# evidence either: a target that never had a plugin is left untouched and silent.
retire_vibe_plugin_manifest() {
  local target="$1" manifest="$1/.claude-plugin/plugin.json" name=""
  [[ -f "$manifest" ]] || return 0
  if command -v jq >/dev/null 2>&1; then
    name="$(jq -r '.name // empty' "$manifest" 2>/dev/null || true)"
  else
    # No jq: classify by the FIRST key only. A greedy match on any `"name"` takes a
    # nested one (`"author":{"name":"vibe"}`) and misclassifies a foreign manifest.
    # build-plugin.sh emits `name` first, so a shipped vibe manifest matches; one
    # that is not first-keyed is preserved (conservative).
    local flat; flat="$(<"$manifest")"; flat="${flat//[[:space:]]/}"
    case "$flat" in
      '{"name":"'*) name="${flat#'{"name":"'}"; name="${name%%\"*}" ;;
    esac
  fi
  if [[ "$name" != "vibe" ]]; then
    note "preserve ${manifest#"$target"/} — a non-vibe Claude plugin manifest (name: ${name:-unknown})"
    return 0
  fi
  # Never delete through a symlinked .claude-plugin or .claude/hooks.
  inside_target "$manifest" && rm -f "$manifest"
  inside_target "$target/.claude/hooks/hooks.json" && rm -f "$target/.claude/hooks/hooks.json"
  return 0
}

# adapter_rows — one "<key>\t<file>\t<target>" line per adapter, read from the
# adapter catalogue. flow/reference/adapters.json says of itself "Extend this list
# to add runtimes without editing skill prose", and flow/setup.md calls it data —
# but install and uninstall both hardcoded `case claude) … warp)` and
# `for adapter in CLAUDE.md WARP.md`, so a third entry changed nothing. It is data
# now.
#
# The KEY is derived from the filename (CLAUDE.md -> claude), which is exactly the
# vocabulary `--adapters` already takes, so the manifest needs no new field.
# jq-less (and empty-manifest) targets fall back to the two shipped pairs, so the
# behaviour never silently reduces to nothing.
adapter_rows() {
  local manifest="$SRC/flow/reference/adapters.json" rows=""
  if command -v jq >/dev/null 2>&1 && [[ -f "$manifest" ]]; then
    rows="$(jq -r '
      (.canonical // "AGENTS.md") as $c
      | .adapters[]?
      | select(.file != null)
      | [(.file | ascii_downcase | sub("\\.md$"; "")), .file, (.target // $c)]
      | @tsv' "$manifest" 2>/dev/null || true)"
  fi
  if [[ -z "$rows" ]]; then
    printf 'claude\tCLAUDE.md\tAGENTS.md\nwarp\tWARP.md\tAGENTS.md\n'
    return 0
  fi
  printf '%s\n' "$rows"
}

# inside_target PATH — true when PATH's parent directory resolves physically
# inside TARGET (TARGET itself is physical, `pwd -P`), NOT inside the vibe
# source SRC, and NOT inside any other vibe checkout below TARGET. An ancestor
# between TARGET and PATH that is a symlink out of the repo (`.agents`,
# `.agents/skills` or `.claude` shared through a dotfiles dir) makes it false;
# so does an in-repo link into a vibe checkout vendored inside TARGET
# (`.claude -> vendor/vibe/.claude`), whose files are that checkout's own. The
# vendored checkout is recognised whichever vibe copy runs: by being SRC, or by
# carrying the same top-level source markers the uninstall guard checks
# (`flow/state-machine.json` and `spec/SKILL.md`) on any directory between
# TARGET and the resolved parent. An install target never has both: it ships
# them under `.agents/skills/flow/` and `.agents/skills/spec/`.
inside_target() {
  local parent d
  parent="$(cd "$(dirname "$1")" 2>/dev/null && pwd -P)" || return 1
  [[ "$parent" == "$TARGET" || "$parent" == "$TARGET"/* ]] || return 1
  [[ -z "$SRC" || ( "$parent" != "$SRC" && "$parent" != "$SRC"/* ) ]] || return 1
  d="$parent"
  while [[ "$d" == "$TARGET"/* ]]; do
    # These exact owned discovery containers now have flow/ + spec/ children;
    # their shape is not a nested source checkout. Neighboring paths stay guarded.
    case "$d" in
      "$TARGET/.agents/skills"|"$TARGET/plugins/vibe/skills"|"$TARGET/factory-plugin/skills") ;;
      *) [[ -f "$d/flow/state-machine.json" && -f "$d/spec/SKILL.md" ]] && return 1 ;;
    esac
    d="$(dirname "$d")"
  done
  return 0
}

# remove_shipped SRC_DIR DST_DIR [EXCLUDE_REL...] — delete from DST_DIR only the
# files that exist in SRC_DIR (the precise inverse of a copy), skipping any whose
# relative path equals or sits under an EXCLUDE_REL prefix (the artifacts install
# scrubs before shipping, or the per-project runtime state it must preserve), then
# prune emptied subdirs. Never touches co-located files the user added to a shared
# directory, never deletes outside the physical TARGET, and never deletes inside
# the vibe source (DST resolving to, into, or around SRC_DIR).
remove_shipped() {
  local src="$1" dst="$2"; shift 2
  local excludes=("$@") f rel ex skip psrc pdst par ppar last_par=""
  [[ -d "$src" && -d "$dst" ]] || return 0
  # A symlinked ancestor would route every delete below into another tree.
  if ! inside_target "$dst"; then
    err "WARN: $dst resolves outside $TARGET or into the vibe source via a symlinked parent directory; left untouched"
    return 0
  fi
  psrc="$(cd "$src" && pwd -P)"
  # A symlinked DST is not a directory vibe copied into: following it would delete
  # files wherever it points. Unlink it only when it points into the vibe source
  # (an install that linked rather than copied); otherwise leave it alone.
  if [[ -L "$dst" ]]; then
    pdst="$(cd "$dst" && pwd -P)"
    if [[ "$pdst" == "$psrc" || "$pdst" == "$psrc"/* ]]; then
      rm -f "$dst"
    else
      err "WARN: $dst is a symlink to $pdst, outside the vibe source; left untouched"
    fi
    return 0
  fi
  pdst="$(cd "$dst" && pwd -P)"
  if [[ "$pdst" == "$psrc" || "$pdst" == "$psrc"/* || "$psrc" == "$pdst"/* ]]; then
    err "WARN: $dst resolves to $pdst, which overlaps the vibe source; left untouched"
    return 0
  fi
  while IFS= read -r f; do
    rel="${f#"$src"/}"
    skip=0
    for ex in ${excludes[@]+"${excludes[@]}"}; do
      if [[ "$rel" == "$ex" || "$rel" == "$ex"/* ]]; then skip=1; break; fi
    done
    [[ "$skip" -eq 1 ]] && continue
    # A symlinked subdirectory of DST would make rm follow it out of DST.
    par="$(dirname "$dst/$rel")"
    if [[ "$par" != "$last_par" ]]; then
      last_par="$par"
      ppar="$(cd "$par" 2>/dev/null && pwd -P)" || ppar=""
    fi
    [[ -n "$ppar" ]] || continue
    if [[ "$ppar" != "$pdst" && "$ppar" != "$pdst"/* ]]; then
      err "WARN: $dst/$rel resolves outside $dst via a symlinked directory; skipped"
      continue
    fi
    rm -f "$dst/$rel"
  done < <(find -L "$src" -type f)
  find "$dst" -type d -empty -delete 2>/dev/null || true
}

# scrub_source_only SRC_DIR DST_DIR — remove from a freshly copied skill tree the
# artifacts that exist in the source but must never reach a target: every
# co-located `tests` directory AT ANY DEPTH, the contributor AGENTS.md, and any
# source-side evidence receipts.
#
# Enumerated from SRC_DIR rather than hardcoded, because a hardcoded list is
# exactly what failed: the previous spelling scrubbed `$TARGET/.../vibe/tests`
# only, so js-core's `flow/engine/tests` — one level deeper — shipped 17 files
# and 396K of oracle-spawning test code into every user repo, contradicting the
# comment above it and falsifying the premise the R1 primitive scan's `tests/`
# exemption is argued on. A new co-located tests dir at any depth is now covered
# without editing anything here.
#
# Source-enumerated also means DST-safe in the same way remove_shipped is: a
# path the SOURCE does not have is never touched in the target, so a user's own
# file dropped into the skill dir survives — AGENTS.md and evidence/ are each
# removed only when SRC_DIR carries that same path. The vibe-skill call site
# (install.sh's copy-vibe step) saves the target's evidence/ before this call
# and restores it after, so scrubbing a source-side evidence/ here never loses
# a target's own receipts.
scrub_source_only() {
  local src="$1" dst="$2" preserve_runtime="${3:-}" rel d
  local dirs=()
  [[ -d "$src" && -d "$dst" ]] || return 0
  # -prune: never descend into a tests dir (a nested one is removed with its
  # parent anyway), so the list is stable while the removals below run.
  while IFS= read -r d; do
    rel="${d#"$src"/}"
    [[ -n "$rel" && "$rel" != "$d" ]] || continue
    dirs+=("$rel")
  done < <(find -L "$src" -type d -name tests -prune -print 2>/dev/null)
  # ${var:?} on both halves: an empty $rel would make this `rm -rf "$dst/"`.
  for rel in ${dirs[@]+"${dirs[@]}"}; do
    rm -rf "${dst:?}/${rel:?}"
  done
  [[ -f "$src/AGENTS.md" ]] && rm -f "$dst/AGENTS.md"
  [[ "$preserve_runtime" != preserve-runtime ]] || return 0
  [[ -d "$src/evidence" ]] && rm -rf "${dst:?}/evidence"
  # warnings.log is the SOURCE checkout's own runtime relay spool (this repo
  # dogfoods the flow, so a dirty tree has one). Shipping it makes a fresh target
  # drain warnings about work nobody did there, and remove_shipped excludes the
  # path, so an uninstall could never clear it again.
  [[ -f "$src/warnings.log" ]] && rm -f "$dst/warnings.log"
  return 0
}

# gi_append FILE LINE... — append the given lines to a .gitignore, separating them
# from prior content with exactly one blank line — but only when FILE already
# exists with content whose last line is non-blank. A freshly created (or
# blank-terminated) file gains no leading blank line.
gi_append() {
  local file="$1"; shift
  if [[ -s "$file" && -n "$(tail -n 1 "$file")" ]]; then
    printf '\n' >> "$file"
  fi
  printf '%s\n' "$@" >> "$file"
}

# register_skill NAME — link TARGET/.claude/skills/NAME -> ../../.agents/skills/NAME
# so the skill resolves where the docs say. Relative, so the tree stays portable.
# Idempotent (our own symlink is left as-is); never clobbers a user's real entry or
# a symlink they aimed elsewhere — that slot is theirs, warn once and skip.
register_skill() {
  local name="$1"
  local dir="$TARGET/.claude/skills"
  local link="$dir/$name"
  local rel="../../.agents/skills/$name"
  if [[ -L "$link" ]]; then
    [[ "$(readlink "$link")" == "$rel" ]] && return 0
    err "WARN: $link is a symlink to $(readlink "$link"), not $rel — leaving it untouched."
    return 0
  fi
  if [[ -e "$link" ]]; then
    err "WARN: $link already exists (not a vibe symlink) — leaving it untouched."
    return 0
  fi
  mkdir -p "$dir"
  ln -s "$rel" "$link"
}

# unregister_skill NAME — remove TARGET/.claude/skills/NAME only when it is exactly
# the symlink register_skill created (a symlink whose target is ../../.agents/skills/NAME).
# A user's real dir/file, or a symlink they aimed elsewhere, is never touched.
unregister_skill() {
  local name="$1"
  local link="$TARGET/.claude/skills/$name"
  local rel="../../.agents/skills/$name"
  if [[ -L "$link" && "$(readlink "$link")" == "$rel" ]] && inside_target "$link"; then
    rm -f "$link"
  fi
}

# ── native host surfaces (--platform) ────────────────────────────────────────
# The supported hosts and their project surfaces are DATA:
# flow/reference/host-surfaces.json ships to every target as
# .agents/skills/flow/reference/host-surfaces.json and is the single source for
# how each host discovers the shared skills and how the user invokes a workflow.
# HOST_IDS is the selector vocabulary this installer accepts; a new host must
# appear in both, and the adapters suite fails if they diverge.
HOST_IDS=(claude opencode codex droid)

# detect_hosts — the DEFAULT selection: presence-only, offline, deterministic
# (fixed order), one `command -v` per supported CLI. Configuring a host writes
# its project surfaces, so detection never guesses: an absent CLI configures
# nothing, and the summary then prints the explicit selection syntax.
detect_hosts() {
  local h found=""
  for h in ${HOST_IDS[@]+"${HOST_IDS[@]}"}; do
    command -v "$h" >/dev/null 2>&1 || continue
    found="${found:+$found,}$h"
  done
  printf '%s' "$found"
}

# resolve_platforms RAW — validate a --platform value and set $PLATFORMS to the
# canonical, deduped, host-ordered list (empty for `none`). `none` is exclusive:
# combining it with a host is a usage error rather than a silent precedence rule,
# because "none,codex" has no defensible reading.
PLATFORMS=""
resolve_platforms() {
  local raw="$1" tok h any=0 out=""
  local toks
  # An EMPTY raw value is a usage error, exactly like an empty token inside a
  # list: `IFS=',' read -a` on "" iterates zero tokens, so without this the
  # `--platform=` form (and `--platform="$VAR"` with VAR unset) silently
  # degraded to host-neutral instead of failing loudly.
  if [[ -z "$raw" ]]; then
    err "ERROR: --platform has an empty value (claude, opencode, codex, droid, or none)"
    return 1
  fi
  IFS=',' read -r -a toks <<< "$raw"
  for tok in ${toks[@]+"${toks[@]}"}; do
    case "$tok" in
      claude|opencode|codex|droid) any=1 ;;
      none) ;;
      "") err "ERROR: --platform has an empty value (claude, opencode, codex, droid, or none)"; return 1 ;;
      *) err "ERROR: --platform takes claude, opencode, codex, droid, or none (got '$tok')"; return 1 ;;
    esac
  done
  case ",$raw," in
    *,none,*)
      if [[ "$any" -eq 1 ]]; then
        err "ERROR: --platform none cannot be combined with a host (got '$raw')"
        return 1
      fi
      PLATFORMS=""
      return 0 ;;
  esac
  for h in ${HOST_IDS[@]+"${HOST_IDS[@]}"}; do
    case ",$raw," in *",$h,"*) out="${out:+$out,}$h" ;; esac
  done
  PLATFORMS="$out"
}

# host_selected HOST — is HOST in the resolved selection?
host_selected() {
  case ",$PLATFORMS," in *",$1,"*) return 0 ;; esac
  return 1
}

# count_components PATH — depth of a repo-relative path (1 for "a", 3 for
# "a/b/c"). Used to rewrite a package's skill links for the target.
count_components() {
  local s="$1" n=1
  while [[ "$s" == */* ]]; do s="${s#*/}"; n=$((n + 1)); done
  printf '%s' "$n"
}

# rel_up N — "../" repeated N times.
rel_up() {
  local n="$1" s=""
  while [[ "$n" -gt 0 ]]; do s="$s../"; n=$((n - 1)); done
  printf '%s' "$s"
}

# ── install manifest ─────────────────────────────────────────────────────────
# The manifest is the ownership record: one action per artifact THIS install
# created, so a repeat install is idempotent and an uninstall removes only
# vibe-managed paths. It lives at .vibe/install-manifest.json and is deliberately
# NOT gitignored — a fresh clone of an installed project must still be able to
# run the uninstall that reads it. It carries no timestamp: the file must be
# byte-stable across a re-install, which is what makes "idempotent" checkable.
#
# Kinds:
#   file    — a copied file; uninstall removes exactly this path.
#   symlink — a skill link we created; uninstall removes it only while it still
#             points at the recorded relative target (a retargeted link is the
#             user's, and is left alone).
#   merge   — a merged file (Claude's .claude/settings.json); unwiring is
#             merge-settings.sh's job, never a wholesale delete.
ACTIONS=""
record_action() {
  local host="$1" kind="$2" path="$3" target="${4:-}" tab=$'\t'
  if [[ -n "$target" ]]; then
    ACTIONS="${ACTIONS}${host}${tab}${kind}${tab}${path}${tab}${target}"$'\n'
  else
    ACTIONS="${ACTIONS}${host}${tab}${kind}${tab}${path}"$'\n'
  fi
}

manifest_path() { printf '%s' "$TARGET/.vibe/install-manifest.json"; }

# PRESERVE_HOSTS — the selected hosts whose deployment THIS run SKIPPED (a
# `--only spec` pass installs no host entry point). Their artifacts were not
# re-created, so their existing manifest records are still the truth and must
# survive the rewrite below; dropping them strands those files at uninstall (the
# uninstall would take the manifest pass — which some other host's records keep
# alive — and never fall back to the shipped-source inverse).
#
# Keyed on the DEPLOY-SKIP, deliberately not on "produced no actions": a
# collision refusal also records nothing, and preserving that host's stale
# records would make a later uninstall delete a USER's file.
PRESERVE_HOSTS=""
deploy_skipped() {
  case ",$PRESERVE_HOSTS," in *",$1,"*) return 0 ;; esac
  return 1
}

# render_manifest BODY_TSV — write the manifest JSON for the given action body
# (TSV, sorted for a byte-stable file: the same selection twice is byte-identical,
# which is what makes "idempotent" checkable). Shared by write_manifest (install)
# and prune_manifest_dead (an uninstall pass that leaves the manifest in place).
render_manifest() {
  local body="$1" m tmp first=1 ahost akind apath atarget
  m="$(manifest_path)"
  tmp="$(mktemp)"
  {
    printf '{\n'
    printf '  "version": 1,\n'
    printf '  "comment": "vibe install manifest - artifacts this install created, per host. Written by install.sh; read by --uninstall. Do not edit by hand.",\n'
    printf '  "actions": [\n'
    while IFS=$'\t' read -r ahost akind apath atarget; do
      [[ -n "$ahost" ]] || continue
      if [[ "$first" -eq 1 ]]; then first=0; else printf ',\n'; fi
      if [[ -n "$atarget" ]]; then
        printf '    {"host": "%s", "kind": "%s", "path": "%s", "target": "%s"}' \
          "$ahost" "$akind" "$apath" "$atarget"
      else
        printf '    {"host": "%s", "kind": "%s", "path": "%s"}' "$ahost" "$akind" "$apath"
      fi
    done <<< "$(LC_ALL=C sort "$body")"
    printf '\n  ]\n}\n'
  } > "$tmp"
  mv -f "$tmp" "$m"
}

# write_manifest — render $ACTIONS as JSON, sorted for a byte-stable file.
#
# MERGE, not replace: an install refreshes the records for the hosts it SELECTED
# (it just re-created their artifacts) and PRESERVES the records for every other
# host — those artifacts are still on disk, still vibe-owned, and a later
# uninstall must still be able to remove them. Without the merge, installing
# `--platform codex` after `--platform claude` would erase the Claude records and
# strand those files. A selected host whose deployment was SKIPPED this run
# (`deploy_skipped`) is preserved too, and for the same reason: nothing was
# re-created for it, so its old records are still the account of what is on disk.
write_manifest() {
  local m body ahost akind apath atarget
  m="$(manifest_path)"
  body="$(mktemp)"
  mkdir -p "$TARGET/.vibe"
  printf '%s' "$ACTIONS" > "$body"
  if [[ -f "$m" ]]; then
    while IFS=$'\t' read -r ahost akind apath atarget; do
      [[ -n "$ahost" ]] || continue
      host_selected "$ahost" && ! deploy_skipped "$ahost" && continue
      printf '%s\t%s\t%s\t%s\n' "$ahost" "$akind" "$apath" "$atarget" >> "$body"
    done <<< "$(manifest_actions)"
  fi
  render_manifest "$body"
  rm -f "$body"
}

# prune_manifest_dead — rewrite the manifest keeping only the records whose
# recorded path still exists on disk. An uninstall pass that LEAVES the manifest
# in place (--only spec removes one half) must not keep a record claiming a path
# it just deleted: a stale record is a false ownership claim. Records for paths
# that survive (the flow half's host artifacts) stay, so a later full uninstall
# can still remove them. This is an uninstall-time write, so it takes the same
# target_owned guard as the full uninstall's manifest rm: a `.vibe` that links out
# of the target holds someone else's manifest, which is warned about and left
# byte-identical (review-hardening/15).
prune_manifest_dead() {
  local m body ahost akind apath atarget
  m="$(manifest_path)"
  [[ -f "$m" ]] || return 0
  target_owned "${m#"$TARGET"/}" || return 0
  body="$(mktemp)"
  while IFS=$'\t' read -r ahost akind apath atarget; do
    [[ -n "$ahost" ]] || continue
    if [[ -e "$TARGET/$apath" || -L "$TARGET/$apath" ]]; then
      printf '%s\t%s\t%s\t%s\n' "$ahost" "$akind" "$apath" "$atarget" >> "$body"
    fi
  done <<< "$(manifest_actions)"
  render_manifest "$body"
  rm -f "$body"
}

# manifest_actions — print the recorded actions as TSV (host, kind, path,
# target). jq when present; otherwise a reader over this file's own emitted
# format (jq is recommended, not required). Deliberately NOT a sed expression
# with a `\t` replacement: BSD sed writes a literal `t`, and this installer is
# macOS-safe. A missing or unparsable manifest prints nothing, which uninstall
# treats as a legacy target and answers with the shipped-source inverse.
manifest_actions() {
  local m line ahost akind apath atarget
  m="$(manifest_path)"
  [[ -f "$m" ]] || return 0
  if command -v jq >/dev/null 2>&1; then
    jq -r '.actions[]? | [.host, .kind, .path, (.target // "")] | @tsv' "$m" 2>/dev/null && return 0
  fi
  while IFS= read -r line; do
    case "$line" in
      *'"host": "'*'"kind": "'*'"path": "'*) ;;
      *) continue ;;
    esac
    ahost="${line#*\"host\": \"}"; ahost="${ahost%%\"*}"
    akind="${line#*\"kind\": \"}"; akind="${akind%%\"*}"
    apath="${line#*\"path\": \"}"; apath="${apath%%\"*}"
    atarget=""
    case "$line" in
      *'"target": "'*)
        atarget="${line#*\"target\": \"}"; atarget="${atarget%%\"*}" ;;
    esac
    printf '%s\t%s\t%s\t%s\n' "$ahost" "$akind" "$apath" "$atarget"
  done < "$m"
}

# deploy_file HOST SRC_FILE REL — copy one managed file into the target and record
# it. An existing entry that DIFFERS from the package is user-owned: report it
# with a recovery path and leave it (design: report a collision, never silently
# replace). An identical file is refreshed — that is the idempotent re-install.
# The collision check runs FIRST, in BOTH modes: a dry-run has to plan what a real
# run would DO, and a real run refuses to overwrite a user's differing entry — so
# checking after the dry-run branch made the plan claim a deploy the run refuses.
deploy_file() {
  local host="$1" srcf="$2" rel="$3" dst="$TARGET/$3" reason=""
  if [[ ! -f "$srcf" ]]; then
    err "WARN: $rel is missing from the source package — skipping it."
    return 0
  fi
  if [[ -L "$dst" ]]; then
    reason="exists as a symlink"
  elif [[ -e "$dst" ]] && ! cmp -s "$srcf" "$dst"; then
    reason="exists and differs from the vibe package"
  fi
  if [[ "$DRY_RUN" -eq 1 ]]; then
    if [[ -n "$reason" ]]; then
      say "skip $rel ($reason — would leave untouched)"
    else
      say "deploy $rel -> $dst"
    fi
    return 0
  fi
  if [[ -n "$reason" ]]; then
    err "WARN: $rel already $reason — leaving it untouched (move it aside and re-run to install vibe's copy)."
    return 0
  fi
  mkdir -p "$(dirname "$dst")"
  cp -R "$srcf" "$dst"
  record_action "$host" file "$rel"
}

# skill_present NAME — is the shared skill NAME available in this target's core,
# or (in a dry-run) would this run put it there? A dry-run writes nothing, so
# answering from the filesystem alone would report EVERY package link as skipped
# — the opposite of what the real run does. The selection is the honest oracle
# there: WANT_SPEC/WANT_FLOW say which halves this run installs, and a real run
# has already copied them into $TARGET/.agents/skills before any host package is
# deployed, so both clauses agree in that mode.
skill_present() {
  [[ -e "$TARGET/.agents/skills/$1" ]] && return 0
  case "$1" in
    spec) [[ "$WANT_SPEC" -eq 1 ]] && return 0 ;;
    flow) [[ "$WANT_FLOW" -eq 1 ]] && return 0 ;;
    vibe) return 0 ;;
  esac
  return 1
}

# deploy_package HOST PKG — copy a host package directory into the target and
# LINK its skills to the target's own core (.agents/skills/<name>).
#
# The package ships REAL skill dirs (build-plugin.sh materializes them, because
# `codex plugin add` and `droid plugin install` DROP a symlinked dir from their
# per-user cache — vibe#71), but a repo-scoped install must not fork a second
# copy of a skill into the target: the target's own .agents/skills/<name> is the
# copy the flow updates, and it is what every other host surface points at. So
# the skills/ subtree is pruned from the file copy and re-created as the same
# relative symlink this package used to carry, keeping the skip-on-missing-half,
# collision and ownership semantics unchanged. Enumerated WITHOUT following
# symlinks, so the copy never walks into a real skill tree.
deploy_package() {
  local host="$1" pkg="$2" src="$SRC/$2" f l rel dst name d want reason
  if [[ ! -d "$src" ]]; then
    err "WARN: host package $pkg is missing from the source — skipping it."
    return 0
  fi
  while IFS= read -r f; do
    rel="${f#"$src"/}"
    deploy_file "$host" "$f" "$pkg/$rel"
  done < <(find "$src" -path "$src/skills" -prune -o -type f -print 2>/dev/null)
  while IFS= read -r l; do
    [[ -n "$l" ]] || continue
    name="$(basename "$l")"
    rel="skills/$name"
    d="$(dirname "$pkg/$rel")"
    want="$(rel_up "$(count_components "$d")").agents/skills/$name"
    dst="$TARGET/$pkg/$rel"
    # A package skill link is only useful while the skill it POINTS AT exists in
    # this target. The link target is the target's own core, so under --only flow
    # the spec half was never copied and `skills/spec` would resolve to nothing —
    # a dangling link that advertises a skill the target does not have (the mirror
    # of the --only spec case, which deploys no host entry point at all). Skip it
    # and leave the SLOT FREE: nothing occupies it, so a later install that does
    # carry the half creates the link here.
    if ! skill_present "$name"; then
      if [[ "$name" == "spec" && "$WANT_SPEC" -eq 0 ]]; then
        reason="the spec skill is not installed (--only flow installs no spec half)"
      else
        reason="the $name skill is not installed under .agents/skills/"
      fi
      if [[ "$DRY_RUN" -eq 1 ]]; then
        say "skip $pkg/$rel ($reason)"
      else
        err "WARN: $pkg/$rel not linked — $reason."
      fi
      continue
    fi
    # Collision next, in both modes — same reason as deploy_file: a slot the user
    # owns (a symlink they aimed elsewhere, or a real file/dir of their own) is
    # reported and left alone by a real run, so the dry-run plan must say so
    # rather than claim a link it would refuse to create.
    if [[ -L "$dst" ]]; then
      if [[ "$(readlink "$dst")" == "$want" ]]; then
        if [[ "$DRY_RUN" -eq 1 ]]; then
          say "link $pkg/$rel -> $want"
        else
          record_action "$host" symlink "$pkg/$rel" "$want"
        fi
      elif [[ "$DRY_RUN" -eq 1 ]]; then
        say "skip $pkg/$rel (symlink to $(readlink "$dst") — would leave untouched)"
      else
        err "WARN: $pkg/$rel is a symlink to $(readlink "$dst") — leaving it untouched."
      fi
      continue
    fi
    if [[ -e "$dst" ]]; then
      if [[ "$DRY_RUN" -eq 1 ]]; then
        say "skip $pkg/$rel (exists — would leave untouched)"
      else
        err "WARN: $pkg/$rel already exists (not a vibe symlink) — leaving it untouched."
      fi
      continue
    fi
    if [[ "$DRY_RUN" -eq 1 ]]; then
      say "link $pkg/$rel -> $want"
      continue
    fi
    mkdir -p "$(dirname "$dst")"
    ln -s "$want" "$dst"
    record_action "$host" symlink "$pkg/$rel" "$want"
  done < <(find "$src/skills" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort)
}

# remove_host_file REL — remove one recorded vibe artifact. Never a directory:
# the caller prunes emptied dirs afterwards, and only ever ones the shipped
# package (or the manifest) proves were vibe's.
remove_host_file() { if target_owned "$1"; then rm -f "$TARGET/$1"; fi; }

# target_owned REL — true when TARGET/REL may be deleted: its parent resolves
# physically inside TARGET and outside the vibe source (inside_target). On a
# miss it warns, but only for a path that is really there, so an absent record
# stays silent.
target_owned() {
  local p="$TARGET/$1"
  inside_target "$p" && return 0
  if [[ -e "$p" || -L "$p" ]]; then
    err "WARN: $1 is outside the target; left alone"
  fi
  return 1
}

# prune_empty DIR... — delete the empty directories under each DIR (DIR too),
# but only for a DIR that resolves inside TARGET: a symlinked ancestor would
# route the prune into another tree. Best-effort and silent.
prune_empty() {
  local d
  for d in "$@"; do
    inside_target "$d/." || continue
    find "$d" -type d -empty -delete 2>/dev/null || true
  done
  return 0
}

# rmdir_inside DIR... — rmdir each DIR that resolves inside TARGET; silent.
rmdir_inside() {
  local d
  for d in "$@"; do
    inside_target "$d/." || continue
    rmdir "$d" 2>/dev/null || true
  done
  return 0
}

# dry_remove_rel REL — the dry-run half of a removal pass: print the exact target
# path that would be removed, and only when the path is really there (`-L` too,
# so a dangling symlink we would delete still appears). The plan therefore names
# concrete project paths and never invents one. A path outside the target is
# shown as the skip a real run would make.
dry_remove_rel() {
  local p="$TARGET/$1"
  [[ -e "$p" || -L "$p" ]] || return 0
  if ! inside_target "$p"; then
    say "skip $1 (outside the target; left alone)"
    return 0
  fi
  say "remove $1"
}

# remove_host_package PKG — the per-file inverse of deploy_package. Removes the
# files the SOURCE package ships (never a path the source lacks, which is how a
# user's co-located file survives), and removes a skill link only while it is a
# symlink still pointing at the target-relative path deploy_package created — a
# user's real directory, or a link they aimed elsewhere, is never touched. The
# skills/ subtree is skipped in the file pass for the same reason deploy_package
# skips it: it is a link to the target's own core, not a copy. Then prunes dirs
# this emptied. In dry-run it removes nothing and prints the paths it would. A
# package dir that resolves outside the target (a symlink to a shared copy) is
# left whole with a warning, and so is any file behind a symlinked subdirectory.
remove_host_package() {
  local pkg="$1" src="$SRC/$1" rel l f name d want dst
  [[ -d "$src" ]] || return 0
  if [[ -e "$TARGET/$pkg" || -L "$TARGET/$pkg" ]] && ! inside_target "$TARGET/$pkg/."; then
    if [[ "$DRY_RUN" -eq 1 ]]; then
      say "skip $pkg (outside the target; left alone)"
    else
      err "WARN: $pkg is outside the target; left alone"
    fi
    return 0
  fi
  if [[ "$DRY_RUN" -eq 1 ]]; then
    while IFS= read -r f; do
      dry_remove_rel "${f#"$SRC"/}"
    done < <(find "$src" -path "$src/skills" -prune -o -type f -print 2>/dev/null)
    while IFS= read -r l; do
      [[ -n "$l" ]] || continue
      name="$(basename "$l")"
      rel="skills/$name"
      d="$(dirname "$pkg/$rel")"
      want="$(rel_up "$(count_components "$d")").agents/skills/$name"
      dst="$TARGET/$pkg/$rel"
      if [[ -L "$dst" && "$(readlink "$dst")" == "$want" ]]; then dry_remove_rel "$pkg/$rel"; fi
    done < <(find "$src/skills" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort)
    return 0
  fi
  [[ -d "$TARGET/$pkg" ]] || return 0
  while IFS= read -r f; do
    rel="${f#"$src"/}"
    if target_owned "$pkg/$rel"; then rm -f "$TARGET/$pkg/$rel"; fi
  done < <(find "$src" -path "$src/skills" -prune -o -type f -print 2>/dev/null)
  while IFS= read -r l; do
    [[ -n "$l" ]] || continue
    name="$(basename "$l")"
    rel="skills/$name"
    d="$(dirname "$pkg/$rel")"
    want="$(rel_up "$(count_components "$d")").agents/skills/$name"
    dst="$TARGET/$pkg/$rel"
    if [[ -L "$dst" && "$(readlink "$dst")" == "$want" ]] && target_owned "$pkg/$rel"; then rm -f "$dst"; fi
  done < <(find "$src/skills" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort)
  prune_empty "$TARGET/$pkg"
}

# host_entry HOST — set HOST_KIND / HOST_INV / HOST_PATH from the SHIPPED surface
# matrix (host-surfaces.json lands inside the target's vibe skill), so the
# completion report cannot drift from the data that declares the surfaces. A
# jq-less target falls back to the same values; the adapters suite pins that
# fallback against the matrix, so the two cannot diverge silently.
HOST_KIND=""; HOST_INV=""; HOST_PATH=""
host_entry() {
  local h="$1" m="$TARGET/.agents/skills/flow/reference/host-surfaces.json"
  HOST_KIND=""; HOST_INV=""; HOST_PATH=""
  if command -v jq >/dev/null 2>&1 && [[ -f "$m" ]]; then
    # `|| true` per read: a corrupted matrix degrades to the fallback below, it
    # never kills a finished install at the report step.
    HOST_KIND="$(jq -r --arg h "$h" '.hosts[] | select(.id == $h) | .entry.kind' "$m" 2>/dev/null || true)"
    HOST_INV="$(jq -r --arg h "$h" '.hosts[] | select(.id == $h) | .entry.invocation' "$m" 2>/dev/null || true)"
    HOST_PATH="$(jq -r --arg h "$h" '.hosts[] | select(.id == $h) | .entry.path // empty' "$m" 2>/dev/null || true)"
  fi
  if [[ -z "$HOST_INV" ]]; then
    case "$h" in
      claude)   HOST_KIND="command"; HOST_INV="/flow"; HOST_PATH=".claude/commands/flow.md" ;;
      opencode) HOST_KIND="command"; HOST_INV="/flow"; HOST_PATH=".opencode/commands/flow.md" ;;
      # The literal skill token Codex's picker uses — no synthetic slash command.
      codex)    HOST_KIND="skill";   HOST_INV="\$flow"; HOST_PATH="" ;;
      droid)    HOST_KIND="command"; HOST_INV="/flow"; HOST_PATH="factory-plugin/commands/flow.md" ;;
    esac
  fi
  return 0
}

# ── companion plugins (opt-in --with-plugins) ──────────────────────────────────
# superpowers is verified against the live marketplace. feature-dev has no stable
# public marketplace id, so it ships as a documented slot — fill it in when known.
# Response style is intentionally absent: brief technical English ships as the
# injected `style.ste100` content block, not a plugin. Each entry:
# "name@marketplace|marketplace-source".
VIBE_COMPANIONS=(
  "superpowers@superpowers-marketplace|obra/superpowers-marketplace"
  # "feature-dev@<marketplace>|<owner/repo>"   # add when its marketplace id is known
)

install_companion_plugins() {
  if ! command -v claude >/dev/null 2>&1; then
    err "WARN: --with-plugins needs the 'claude' CLI on PATH; skipping companion plugins."
    return 0
  fi
  local entry id src name
  for entry in ${VIBE_COMPANIONS[@]+"${VIBE_COMPANIONS[@]}"}; do
    id="${entry%%|*}"; src="${entry#*|}"; name="${id%%@*}"
    if claude plugin list 2>/dev/null | grep -q "$name@"; then
      note "companion '$name' already installed — skipping"; continue
    fi
    say "add marketplace '$src' and install '$id' (user scope)"
    [[ "$DRY_RUN" -eq 1 ]] && continue
    if ! claude plugin marketplace add "$src" --scope user >/dev/null 2>&1; then
      err "WARN: could not add marketplace '$src' (offline?); skipping '$name'."; continue
    fi
    if claude plugin install "$id" --scope user >/dev/null 2>&1; then
      note "installed companion '$name'"
    else
      err "WARN: could not install '$id'; skipping."
    fi
  done
}

# ── global (per-user skills + shared instruction block) install ───────────────
# vibe#71 sub-gap 2: this path used to be Claude-only (it shelled out to
# `claude plugin install` and wrote nothing else), so a Codex, Droid or OpenCode
# user had to hand-copy skill trees out of `git archive`. It now writes the two
# skills into each selected host's OWN user-level skills directory and maintains
# one shared user-level AGENTS.md block. Files only — no host CLI required.

# The marker pair around the shared user-level AGENTS.md block. The block is
# rewritten IN PLACE between the markers, so a re-run can neither duplicate nor
# reorder it, and everything outside them stays user-owned.
GLOBAL_MARK_START='<!-- vibe:global:start -->'
GLOBAL_MARK_END='<!-- vibe:global:end -->'
# The block ships as a TRACKED template (never as a file named AGENTS.md at a
# path this installer writes blindly): the markers live inside it.
GLOBAL_TEMPLATE="flow/reference/templates/agents-global.md"

# global_host_root HOST — a host's per-user config root. Each is the user-scope
# mirror of that host's project-level root in flow/reference/host-surfaces.json
# (`claude` -> .claude, `opencode` -> .config/opencode, ...). The documented env
# override wins where the host has one, so a relocated install still lands right.
global_host_root() {
  case "$1" in
    claude)   printf '%s' "${CLAUDE_CONFIG_DIR:-$HOME/.claude}" ;;
    codex)    printf '%s' "${CODEX_HOME:-$HOME/.codex}" ;;
    droid)    printf '%s' "${FACTORY_HOME:-$HOME/.factory}" ;;
    opencode) printf '%s' "${XDG_CONFIG_HOME:-$HOME/.config}/opencode" ;;
    *) return 1 ;;
  esac
}

# global_skills_dir HOST — where that host discovers USER-level skills.
# OpenCode deliberately gets `~/.config/opencode/skills` (its own documented
# global root), NOT the shared `~/.agents/skills`: the point of this path is that
# each host's discovery is real and checkable on its own, and `~/.agents` is
# already the tree the #71 workaround puts hand-copies in.
global_skills_dir() { printf '%s/skills' "$(global_host_root "$1")"; }

# global_host_detected HOST — the default global selection: the host's CLI is on
# PATH, or its user-level config dir already exists (a user who installed the CLI
# but has it off PATH still gets their skills). Never guessed from another host.
global_host_detected() {
  local host="$1" root
  command -v "$host" >/dev/null 2>&1 && return 0
  root="$(global_host_root "$host")" || return 1
  [[ -n "$root" && -d "$root" ]]
}

detect_global_hosts() {
  local h found=""
  for h in ${HOST_IDS[@]+"${HOST_IDS[@]}"}; do
    global_host_detected "$h" || continue
    found="${found:+$found,}$h"
  done
  printf '%s' "$found"
}

# global_stage_skill NAME STAGE_DIR — copy the SHIPPED tree of skill NAME into
# STAGE_DIR/NAME, scrubbed exactly as a repo install scrubs it (co-located tests
# at any depth, the contributor AGENTS.md, source-side evidence/warnings.log) and
# minus the source checkout's runtime cursor. Nothing is written to the target
# here: the caller compares first, so an unchanged install touches no file.
global_stage_skill() {
  local name="$1" stage="$2" src
  case "$name" in spec) src="$SRC/spec" ;; flow) src="$SRC/flow" ;; vibe) src="$SRC/skills/vibe" ;; *) return 1 ;; esac
  [[ -f "$src/SKILL.md" ]] || return 1
  cp -RpL "$src" "$stage/" || return 1
  scrub_source_only "$src" "$stage/$name"
  rm -f "$stage/$name/state.json"
  # Flatten (vibe#71 sub-gap 3): `using-spec` is installed as its own sibling
  # skill dir, so the copy nested inside the spec tree must not ship a second
  # time — a duplicate registers the same skill twice on a host that walks the
  # tree recursively, and a single-level scan (OpenCode's documented
  # `skills/*/SKILL.md`) would miss it anyway.
  rm -rf "$stage/${name:?}/using-spec"
  return 0
}

# global_tree_matches STAGED DEST — content-identical? A missing DEST is a
# mismatch. Content is the criterion (the suite hashes it); modes come from the
# staged copy when the tree is written.
global_tree_matches() {
  [[ -d "$2" ]] || return 1
  diff -r "$1" "$2" >/dev/null 2>&1
}

# install_global_skill HOST NAME — install one skill for one host, idempotently.
install_global_skill() {
  local host="$1" name="$2" dir dest stage
  dir="$(global_skills_dir "$host")" || return 1
  dest="$dir/$name"
  stage="$(mktemp -d "${TMPDIR:-/tmp}/vibe-global.XXXXXX")" || return 1
  if ! global_stage_skill "$name" "$stage"; then
    rm -rf "$stage"
    err "WARN: could not stage the '$name' skill for $host; skipping."
    return 1
  fi
  if [[ "$DRY_RUN" -eq 0 ]]; then
    local hostroot
    hostroot="$(global_host_root "$host")" || return 1
    if ! bash "$SRC/flow/scripts/migrate-discovery.sh" "$hostroot" "$dir"; then rm -rf "$stage"; return 1; fi
    if [[ -e "$dest" || -L "$dest" ]]; then
      case "$name" in
        vibe) grep -q '^# Vibe toolkit$' "$dest/SKILL.md" 2>/dev/null || { err "WARN: foreign vibe skill preserved: $dest"; rm -rf "$stage"; return 1; } ;;
        flow) [[ -f "$dest/state-machine.json" ]] || { err "WARN: foreign flow skill preserved: $dest"; rm -rf "$stage"; return 1; } ;;
      esac
    fi
  fi
  if global_tree_matches "$stage/$name" "$dest"; then
    note "already installed: $name -> $dest ($host, unchanged)"
    rm -rf "$stage"
    return 0
  fi
  say "install the $name skill -> $dest ($host)"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    mkdir -p "$dir"
    mkdir -p "$dest"
    if ! cp -Rp "$stage/$name/." "$dest/"; then
      rm -rf "$stage"
      err "WARN: could not install the '$name' skill for $host (under $dir)."
      return 1
    fi
  fi
  rm -rf "$stage"
  return 0
}

# install_global_agents_block — maintain the shared user-level AGENTS.md block.
# Appended once between the marker pair; on a re-run the region between the
# markers is REWRITTEN IN PLACE from the tracked template, so it is never
# duplicated, never reordered, and a run whose result is byte-identical writes
# nothing at all. Everything outside the markers is left exactly as it was.
install_global_agents_block() {
  local file tpl tmp
  file="$(global_agents_file)"
  tpl="$SRC/$GLOBAL_TEMPLATE"
  if [[ ! -f "$tpl" ]]; then
    err "WARN: the global AGENTS.md template is missing ($tpl); skipping the shared block."
    return 1
  fi
  # An unterminated marker is ambiguous: never guess where the block ended.
  if [[ -f "$file" ]] && grep -qF "$GLOBAL_MARK_START" "$file" && ! grep -qF "$GLOBAL_MARK_END" "$file"; then
    err "WARN: $file has an unterminated vibe:global block — leaving it alone; close it by hand."
    return 1
  fi
  tmp="$(mktemp "${TMPDIR:-/tmp}/vibe-global-agents.XXXXXX")" || return 1
  # A missing file is the "create it" case: awk reads /dev/null and the END rule
  # still emits the block, so no branch is needed here.
  local input="$file"
  [[ -f "$file" ]] || input=/dev/null
  awk -v start="$GLOBAL_MARK_START" -v end="$GLOBAL_MARK_END" -v body="$tpl" '
    function emit(   line) {
      while ((getline line < body) > 0) print line
      close(body)
      done = 1
    }
    {
      if ($0 == start) { if (!done) emit(); inblock = 1; next }
      if (inblock) { if ($0 == end) inblock = 0; next }
      print
      last = $0
    }
    END { if (!done) { if (last != "") print ""; emit() } }
  ' "$input" > "$tmp" || { rm -f "$tmp"; err "WARN: could not compose the shared AGENTS.md block."; return 1; }
  if [[ -f "$file" ]] && cmp -s "$tmp" "$file"; then
    note "already installed: shared AGENTS.md block -> $file (unchanged)"
    rm -f "$tmp"
    return 0
  fi
  say "maintain the shared AGENTS.md block -> $file"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    mkdir -p "$(dirname "$file")"
    # `cat >` (not `mv`) so an existing file keeps its mode and a new one gets
    # the invoking user's umask — the block never lands 0600 or root-owned.
    cat "$tmp" > "$file"
  fi
  rm -f "$tmp"
  return 0
}

# global_agents_file — the ONE shared user-level instruction file. The hosts each
# read their own (Claude `~/.claude/CLAUDE.md`, Codex `~/.codex/AGENTS.md`, Droid
# `~/.factory/AGENTS.md`, OpenCode `~/.config/opencode/AGENTS.md`), so vibe keeps
# a single host-neutral block here and prints the include lines; writing into a
# host's own memory file would clobber user prose the installer does not own.
global_agents_file() { printf '%s/AGENTS.md' "$HOME/.agents"; }

# ── the vibe-spec binary (vibe#71 sub-gap 4) ───────────────────────────────────
# The shipped `spec` skill shells out to `vibe-spec` (root, validate, promote,
# check-drift). install.sh ships SKILL FILES, and a global-install user has no
# checkout to `cargo build` in, so the honest contract is:
#
#   - a plain `--global` REPORTS the binary's status and never fails on it: an
#     absent binary degrades the spec skill (flat legacy roots still validate
#     through the bundled bash scripts), it does not break the install;
#   - `--build-spec-bin` OPTS INTO the real thing: cargo-build from this
#     checkout and copy the artifact into a user-owned bin dir (~/.local/bin),
#     noting when that dir is not on PATH — install.sh never edits a shell
#     profile.
#
# REJECTED: fetching prebuilt release artifacts with a pinned checksum. That
# needs published per-platform release assets (a release action this installer
# cannot take on its own) and it would ship a binary nothing in the repo can
# rebuild reproducibly per target today. Recorded in the PR body, not here.

# vibe_spec_on_path — the `vibe-spec` an agent would actually run: the documented
# $VIBE_SPEC_BIN override first (the same variable spec/hooks/run-hook.sh reads),
# then PATH. Prints its path; fails when there is none.
vibe_spec_on_path() {
  # -f AND -x: `[[ -x ]]` alone is true for a DIRECTORY, and a directory at
  # $VIBE_SPEC_BIN made this report an install that does not exist. `command -v`
  # already requires a regular executable.
  if [[ -n "${VIBE_SPEC_BIN:-}" && -f "${VIBE_SPEC_BIN}" && -x "${VIBE_SPEC_BIN}" ]]; then
    printf '%s\n' "$VIBE_SPEC_BIN"
    return 0
  fi
  command -v vibe-spec 2>/dev/null
}

# vibe_spec_checkout_build — this checkout's own build artifact, if one exists
# (release preferred). CARGO_TARGET_DIR wins, so a user with a redirected target
# dir is told about the binary they actually have.
vibe_spec_checkout_build() {
  local tdir="${CARGO_TARGET_DIR:-$SRC/target}"
  if [[ -f "$tdir/release/vibe-spec" && -x "$tdir/release/vibe-spec" ]]; then
    printf '%s\n' "$tdir/release/vibe-spec"
    return 0
  fi
  if [[ -f "$tdir/debug/vibe-spec" && -x "$tdir/debug/vibe-spec" ]]; then
    printf '%s\n' "$tdir/debug/vibe-spec"
    return 0
  fi
  return 1
}

# vibe_spec_bin_dir_copy — the copy a previous --build-spec-bin left in the user
# bin dir. It is not on PATH (the installer never edits a profile), but on a
# second run it is the thing the user should add, not this checkout's target/.
vibe_spec_bin_dir_copy() {
  local candidate="${BIN_DIR:-$HOME/.local/bin}/vibe-spec"
  [[ -f "$candidate" && -x "$candidate" ]] || return 1
  printf '%s\n' "$candidate"
}

# spec_bin_status — one report line, and the degradation paragraph when absent.
# Never returns non-zero: this is a report, not a gate.
spec_bin_status() {
  local found
  if found="$(vibe_spec_on_path)"; then
    note "vibe-spec: installed ($found)"
    return 0
  fi
  if found="$(vibe_spec_bin_dir_copy)"; then
    note "vibe-spec: installed ($found, not on PATH — add $(dirname "$found") to PATH)"
    return 0
  fi
  if found="$(vibe_spec_checkout_build)"; then
    note "vibe-spec: built in this checkout but NOT on PATH — add $(dirname "$found") to PATH"
    return 0
  fi
  err "vibe-spec: NOT installed — the spec skill's binary commands (root, validate,"
  err "  promote, init) are unavailable. What still works: the bundled scripts beside"
  err "  the skill validate a FLAT legacy root only (bash <skill>/scripts/validate.sh);"
  err "  a layered docs/spec/ tree cannot be validated at all."
  err "  To install it: re-run with --build-spec-bin (needs cargo), or from a clone:"
  err "    cargo install --locked --path crates/vibe-spec"
  return 0
}

# build_spec_bin — cargo build the binary and copy it into the user bin dir.
# Every failure is a WARN (+ a non-zero return the caller may ignore): a build
# that cannot run must leave the documented degradation in place, not abort an
# install that has already written the skills.
build_spec_bin() {
  local bin_dir="${BIN_DIR:-$HOME/.local/bin}"
  local tdir="${CARGO_TARGET_DIR:-$SRC/target}"
  local dest="$bin_dir/vibe-spec" artifact="$tdir/release/vibe-spec"
  if ! command -v cargo >/dev/null 2>&1; then
    err "WARN: --build-spec-bin needs cargo on PATH — skipping the build (skills degrade as reported below)."
    return 1
  fi
  if [[ ! -f "$SRC/Cargo.toml" || ! -d "$SRC/crates/vibe-spec" ]]; then
    err "WARN: no crates/vibe-spec in this tree — nothing to build (skills degrade as reported below)."
    return 1
  fi
  # Never clobber a file at that path that is demonstrably not ours.
  if [[ -e "$dest" ]] && ! "$dest" --version 2>/dev/null | grep -q 'vibe-spec'; then
    err "WARN: $dest exists and is not a vibe-spec binary — leaving it alone."
    return 1
  fi
  say "build vibe-spec (cargo build --release -p vibe-spec) and install it to $dest"
  [[ "$DRY_RUN" -eq 1 ]] && return 0
  if ! ( cd "$SRC" && cargo build --release -p vibe-spec ); then
    err "WARN: cargo build failed — vibe-spec not installed (skills degrade as reported below)."
    return 1
  fi
  if [[ ! -f "$artifact" ]]; then
    err "WARN: cargo reported success but $artifact is missing — not installed."
    return 1
  fi
  if ! mkdir -p "$bin_dir" || ! cp "$artifact" "$dest" || ! chmod 755 "$dest"; then
    err "WARN: could not install $dest (is $bin_dir writable?) — not installed."
    return 1
  fi
  note "vibe-spec: installed ($dest, $("$dest" --version 2>/dev/null || echo 'version unknown'))"
  # PATH is the user's to change: a profile edit here would be a guess about
  # their shell, and install.sh never touches one.
  case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) err "note: $bin_dir is not on PATH — add it (export PATH=\"$bin_dir:\$PATH\") and restart the host CLI." ;;
  esac
  return 0
}

install_global() {
  local hosts h rc=0 built=0
  # Every write lands under $HOME, so it must belong to the invoking user. Root
  # would leave files the user cannot edit or remove.
  if [[ "${EUID:-$(id -u)}" -eq 0 ]]; then
    err "ERROR: --global installs into \$HOME and must not run as root (the files would be root-owned)."
    err "       Run it as yourself; a shared/system-wide install is out of scope."
    exit 1
  fi
  if [[ -z "${HOME:-}" ]]; then
    err "ERROR: --global needs \$HOME (it installs the skills per user)."
    exit 1
  fi
  if [[ "$PLATFORM_GIVEN" -eq 1 ]]; then
    hosts="$PLATFORMS"
    HOSTS_SOURCE="selected with --host/--platform"
  else
    hosts="$(detect_global_hosts)"
    HOSTS_SOURCE="detected by CLI or an existing user config dir"
  fi
  say "install the spec + flow + vibe skills at user scope for ${hosts:-no host} ($HOSTS_SOURCE)"
  # shellcheck disable=SC2086  # ${hosts//,/ } is the intended word split
  for h in ${hosts//,/ }; do
    install_global_skill "$h" spec || rc=1
    install_global_skill "$h" flow || rc=1
    install_global_skill "$h" vibe || rc=1
    if [[ "$DRY_RUN" -eq 0 ]]; then
      local old
      old="$(global_skills_dir "$h")/using-spec"
      if [[ -f "$old/SKILL.md" ]] && cmp -s "$old/SKILL.md" "$SRC/flow/reference/legacy-using-spec.md"; then
        rm -f "$old/SKILL.md"
        rmdir "$old" 2>/dev/null || true
      fi
    fi
  done
  install_global_agents_block || rc=1
  # The `vibe-spec` binary (vibe#71 sub-gap 4). Always REPORTED, because the spec
  # skill's commands depend on it and a silent absence is the bug this fixes;
  # built only when asked, and a build that cannot run must never fail the run.
  if [[ "$BUILD_SPEC_BIN" -eq 1 ]]; then
    if build_spec_bin; then built=1; fi
  fi
  if [[ "$built" -eq 0 ]]; then
    spec_bin_status
  fi
  # The Claude per-user plugin stays available, but only on request: it carries
  # the doctrine hook AND the same three skills, so a default run would load every
  # skill twice (the duplicate-load problem #71's workaround note calls out).
  if [[ "$WANT_PLUGIN" -eq 1 ]]; then
    if ! command -v claude >/dev/null 2>&1; then
      err "WARN: --plugin needs the 'claude' CLI on PATH; skipping the per-user plugin."
    else
      local market="$SRC"
      if [[ "${VIBE_BOOTSTRAPPED:-0}" == "1" ]]; then market="$VIBE_REPO"; fi
      say "add the vibe marketplace ($market) at user scope"
      say "install plugin vibe@vibe at user scope (doctrine hook)"
      if [[ "$DRY_RUN" -eq 0 ]]; then
        claude plugin marketplace add "$market" --scope user >/dev/null 2>&1 \
          || err "WARN: could not add the vibe marketplace (already added?); continuing."
        if claude plugin install vibe@vibe --scope user >/dev/null 2>&1; then
          note "installed vibe@vibe (user scope)"
        else
          err "WARN: could not install vibe@vibe — run 'claude plugin install vibe@vibe' by hand."
        fi
      fi
    fi
  fi
  # Per-repo home for the spec framework: seed the legacy tree in the current repo
  # if it is a git repo with neither root (docs/spec or the legacy tree); the skills
  # apply across repos, each repo still needs its own spec memory. The seed is the
  # bash setup.sh until F22 retires it. The full stateful flow is a separate --local
  # install.
  if [[ -e "$TARGET/.git" && ! -d "$TARGET/docs/spec" && ! -d "$TARGET/$LEGACY_SPEC" ]]; then
    say "seed $LEGACY_SPEC/ in $TARGET (spec framework home for this repo; docs/spec/ also counts)"
    if [[ "$DRY_RUN" -eq 0 && -f "$SRC/spec/scripts/setup.sh" ]]; then
      ( cd "$TARGET" && bash "$SRC/spec/scripts/setup.sh" ) >/dev/null 2>&1 \
        || err "WARN: $LEGACY_SPEC/ seed skipped (run 'bash spec/scripts/setup.sh' in the repo)." # literal
    fi
  fi
  [[ "$WITH_PLUGINS" -eq 1 ]] && install_companion_plugins
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "install: [dry-run] global plan complete — nothing was written."
    exit 0
  fi
  cat <<EOF
install: done (global / per-user skills).
install:   ${hosts:-no host} — spec + flow + vibe skill dirs placed in the user-level skills dir.
install:   ${hosts:+Restart the host CLI (or reload its skills) to pick them up.}
install:   Shared AGENTS.md block: $(global_agents_file)
install:     each host reads its OWN instruction file, so include the block there if
install:     you want it applied globally: ~/.claude/CLAUDE.md (Claude Code),
install:     ~/.codex/AGENTS.md (Codex), ~/.factory/AGENTS.md (Droid),
install:     ~/.config/opencode/AGENTS.md (OpenCode).
install:   vibe-spec: see the status above — without it /spec validate falls back to the
install:     bundled bash scripts (flat legacy roots only); --build-spec-bin installs it.
install:   For the FULL stateful flow (cursor, /flow, hooks) in a repo, run:
install:     ./install.sh <repo> --local
EOF
  exit "$rc"
}

# ── interactive mode prompt (bare TTY run only) ────────────────────────────────
prompt_mode() {
  local ans
  {
    echo "vibe install — choose a mode:"
    echo "  [1] local   full vibe into THIS repo ($TARGET): spec + flow + Claude adapter (default)"
    echo "  [2] global  per-user plugin (spec + flow + vibe skills + doctrine, every repo)"
    printf 'Mode [1/2]? '
  } >&2
  read -r ans || ans=""
  case "$ans" in
    2|g|global) echo global ;;
    *)          echo local ;;
  esac
}

DRY_RUN=0
WANT_SPEC=1
WANT_FLOW=1
# --only is LAST-WINS, resolved once after parsing. Clearing the other half
# inside the parse loop meant `--only spec --only flow` cleared BOTH and
# installed nothing while still printing a success banner.
ONLY=""
UNINSTALL=0
ASSUME_YES=0
MODE=""
WITH_PLUGINS=0
WANT_PLUGIN=0
BUILD_SPEC_BIN=0
BIN_DIR=""
TARGET=""
TARGET_GIVEN=0
ADAPTERS=""
PLATFORM_RAW=""
PLATFORM_GIVEN=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --adapters)
      [[ $# -ge 2 ]] || { err "ERROR: --adapters needs a value (e.g. --adapters claude,warp)"; exit 1; }
      ADAPTERS="$2"; shift 2 ;;
    --adapters=*) ADAPTERS="${1#*=}"; shift ;;
    --platform)
      [[ $# -ge 2 && -n "$2" ]] || { err "ERROR: --platform needs a value: claude, opencode, codex, droid, or none"; exit 1; }
      PLATFORM_RAW="$2"; PLATFORM_GIVEN=1; shift 2 ;;
    --platform=*)
      PLATFORM_RAW="${1#*=}"; PLATFORM_GIVEN=1; shift ;;
    --host)
      [[ $# -ge 2 && -n "$2" ]] || { err "ERROR: --host needs a value: claude, codex, droid, opencode, or all"; exit 1; }
      PLATFORM_RAW="$2"; PLATFORM_GIVEN=1; shift 2 ;;
    --host=*)
      PLATFORM_RAW="${1#*=}"; PLATFORM_GIVEN=1; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    --uninstall) UNINSTALL=1; shift ;;
    --yes|-y) ASSUME_YES=1; shift ;;
    --local)  MODE=local;  shift ;;
    --global) MODE=global; shift ;;
    --with-plugins) WITH_PLUGINS=1; shift ;;
    --plugin) WANT_PLUGIN=1; shift ;;
    --build-spec-bin) BUILD_SPEC_BIN=1; shift ;;
    --bin-dir)
      [[ $# -ge 2 && -n "$2" ]] || { err "ERROR: --bin-dir needs a directory (e.g. --bin-dir \"\$HOME/.local/bin\")"; exit 1; }
      BIN_DIR="$2"; shift 2 ;;
    --bin-dir=*)
      [[ -n "${1#*=}" ]] || { err "ERROR: --bin-dir needs a directory (e.g. --bin-dir=\"\$HOME/.local/bin\")"; exit 1; }
      BIN_DIR="${1#*=}"; shift ;;
    --only)
      case "${2:-}" in
        spec|flow) ONLY="$2" ;;
        *) err "ERROR: --only takes 'spec' or 'flow' (got '${2:-}')"; exit 1 ;;
      esac
      shift 2 ;;
    --only=*)
      case "${1#*=}" in
        spec|flow) ONLY="${1#*=}" ;;
        *) err "ERROR: --only takes 'spec' or 'flow' (got '${1#*=}')"; exit 1 ;;
      esac
      shift ;;
    -h|--help) sed -n '2,/^set -euo pipefail/p' "$0" | sed '$d'; exit 0 ;;
    -*) err "ERROR: unknown option '$1'"; exit 1 ;;
    *) TARGET="$1"; TARGET_GIVEN=1; shift ;;
  esac
done

case "$ONLY" in
  spec) WANT_FLOW=0 ;;
  flow) WANT_SPEC=0 ;;
esac

# Host selection: an explicit --platform overrides detection entirely — with the
# CLIs present AND absent, so `--platform codex` on a machine that has all four
# configures Codex alone, and `--platform claude` on a machine with none still
# configures Claude. With no --platform, detection decides. Validated here, so a
# bad token is a usage error before anything is created.
HOSTS_SOURCE="detected on PATH"
if [[ "$PLATFORM_GIVEN" -eq 1 ]]; then
  # `all` is the --host shorthand for the whole supported set; it expands to the
  # canonical list before validation so one code path decides the ordering.
  [[ "$PLATFORM_RAW" == "all" ]] && PLATFORM_RAW="claude,opencode,codex,droid"
  resolve_platforms "$PLATFORM_RAW" || exit 1
  HOSTS_SOURCE="selected with --platform"
else
  PLATFORMS="$(detect_hosts)"
fi

# Single-command resolution. A bare run (no target) defaults to THIS directory; an
# explicit target behaves exactly as before. Mode: --local/--global win; else a bare
# interactive run prompts (local vs global); else local. Explicit target or
# uninstall never prompts (backward-compatible with scripted/CI invocations).
if [[ "$TARGET_GIVEN" -eq 0 ]]; then
  # Bare run: install into the ENCLOSING repo, not just the cwd. Search upward for a
  # docs/spec/, legacy spec or .git marker (same rule as the suites' _find_repo_root; `-e` catches a
  # .git file in worktrees/submodules), so `install.sh` from a subdirectory targets
  # the repo root. No marker at all (a brand-new project) falls back to the cwd.
  TARGET="$PWD"
  _d="$PWD"
  while [[ "$_d" != "/" ]]; do
    if [[ -d "$_d/docs/spec" || -d "$_d/$LEGACY_SPEC" || -e "$_d/.git" ]]; then TARGET="$_d"; break; fi
    _d="$(dirname "$_d")"
  done
fi
# An explicitly-passed empty/whitespace target (`install.sh ""`) must be rejected
# outright — never silently fall through to the current directory (that would run a
# --uninstall against cwd). Do not rely on a later `mkdir -p ""` failing.
if [[ -z "${TARGET//[[:space:]]/}" ]]; then
  err "ERROR: empty target. Pass a repo path, or run with NO argument to use the current directory."
  exit 1
fi
if [[ -z "$MODE" ]]; then
  if [[ "$TARGET_GIVEN" -eq 1 || "$UNINSTALL" -eq 1 ]]; then
    MODE=local
  elif [[ -t 0 && "$DRY_RUN" -eq 0 ]]; then
    MODE="$(prompt_mode)"
  else
    MODE=local
  fi
fi

# --build-spec-bin is a GLOBAL-mode action: the global user is the one with no
# checkout. Silently ignoring it elsewhere would read as "the binary was built".
SPEC_BIN_REQUESTED="$BUILD_SPEC_BIN"
if [[ "$BUILD_SPEC_BIN" -eq 1 && "$MODE" != "global" ]]; then
  err "WARN: --build-spec-bin applies to --global; ignoring it for a local install."
  BUILD_SPEC_BIN=0
fi
# --bin-dir only names a destination for --build-spec-bin; on its own it does
# nothing, and saying so beats letting a user believe the binary moved. When
# --build-spec-bin WAS given (the branch above just disabled it), the warning
# above already says why: adding "bin-dir without build-spec-bin" here would
# misstate what the user typed.
if [[ -n "$BIN_DIR" && "$BUILD_SPEC_BIN" -eq 0 ]]; then
  [[ "$SPEC_BIN_REQUESTED" -eq 1 ]] || err "WARN: --bin-dir only applies with --build-spec-bin; ignoring it."
  BIN_DIR=""
fi

# Resolve TARGET to an absolute path. Create it only for a real install; a
# dry-run must not touch the filesystem, so normalize without mkdir.
if [[ "$DRY_RUN" -eq 0 ]]; then
  mkdir -p "$TARGET"
fi
if [[ -d "$TARGET" ]]; then
  TARGET="$(cd "$TARGET" && pwd -P)"
else
  case "$TARGET" in
    /*) ;;
    *)  TARGET="$PWD/$TARGET" ;;
  esac
fi

# A LOCAL install into the vibe source repo itself is meaningless (it already IS
# vibe), and ANY uninstall there is destructive: remove_shipped deletes every file
# the source ships from the target, which here is the source tree itself. Global
# install is fine from here — it installs the per-user plugin, not into this tree —
# but `--global --uninstall` still reaches the uninstall block against TARGET, so
# the guard covers uninstall in every mode. SRC and TARGET are both physical
# (`pwd -P`), so a symlink to the source repo is caught too. A vibe checkout that
# is NOT the running SRC (a curl bootstrap snapshot run against a checkout, or a
# second clone or worktree as the target) is caught by content: it carries the
# same source-bundle markers SRC is detected by, which an install target only
# has under .agents/skills/, never at its top level.
if [[ ( "$MODE" == "local" || "$UNINSTALL" -eq 1 ) \
      && ( "$TARGET" == "$SRC" \
           || ( -f "$TARGET/flow/state-machine.json" && -f "$TARGET/spec/SKILL.md" ) ) ]]; then
  if [[ "$UNINSTALL" -eq 1 ]]; then
    err "ERROR: target is a vibe source repo; refusing to uninstall (it would delete its own spec/ and flow/)."
    err "       Pass the repo vibe was installed into as the target."
  else
    err "ERROR: target is a vibe source repo; nothing to install."
    err "       Run from inside another repo, pass a target, or use --global."
  fi
  exit 1
fi

# Global (per-user plugin) install is a distinct, self-contained path.
if [[ "$MODE" == "global" && "$UNINSTALL" -eq 0 ]]; then
  install_global
fi

# Cursor initialization and explicit runtime removal use the same exclusion as
# Rust transitions. This lock grants no approval and never evicts another owner.
install_runtime_lock() {
  local parent
  for parent in "$TARGET/.vibe" "$TARGET/.vibe/run" "$TARGET/.vibe/run/flow"; do
    [[ ! -L "$parent" ]] || { err "ERROR: runtime parent is a symlink: $parent"; return 1; }
  done
  mkdir -p "$TARGET/.vibe/run/flow"
  INSTALL_RUNTIME_LOCK="$TARGET/.vibe/run/flow/runtime.lock"
  INSTALL_RUNTIME_TOKEN="installer-$$-$RANDOM-$(date -u +%s)"
  if [[ -e "$INSTALL_RUNTIME_LOCK" || -L "$INSTALL_RUNTIME_LOCK" ]]; then
    err "ERROR: runtime busy; preserve the existing lock and retry"
    return 1
  fi
  if ! (set -o noclobber; printf '%s\n' "$INSTALL_RUNTIME_TOKEN" > "$INSTALL_RUNTIME_LOCK") 2>/dev/null; then
    err "ERROR: runtime busy; preserve the existing lock and retry"
    return 1
  fi
  trap install_runtime_unlock EXIT
}
install_runtime_unlock() {
  if [[ -f "$INSTALL_RUNTIME_LOCK" && ! -L "$INSTALL_RUNTIME_LOCK" ]] && [[ "$(cat "$INSTALL_RUNTIME_LOCK")" == "$INSTALL_RUNTIME_TOKEN" ]]; then
    rm -f "$INSTALL_RUNTIME_LOCK"
    rmdir "$TARGET/.vibe/run/flow" "$TARGET/.vibe/run" "$TARGET/.vibe" 2>/dev/null || true
  fi
}
install_runtime_adopted() {
  local cursor kind
  [[ ! -e "$TARGET/.vibe/run/flow/runs" && ! -L "$TARGET/.vibe/run/flow/runs" ]] || return 0
  # shellcheck source=flow/scripts/runtime-state.sh
  source "$SRC/flow/scripts/runtime-state.sh"
  for cursor in "$TARGET/.vibe/run/flow/state.json" "$TARGET/.agents/skills/flow/state.json" "$TARGET/.agents/skills/vibe/state.json"; do
    kind="$(runtime_cursor_kind "$cursor")"
    case "$kind" in legacy|absent) ;; *) return 0 ;; esac
  done
  return 1
}

seed_runtime_cursor() (
  install_runtime_lock || exit 1
  local current="$TARGET/.vibe/run/flow/state.json" temp
  # Inspect under the lock. Existing state is preserved, never restored from v1.
  if [[ -e "$current" || -L "$current" ]]; then
    [[ -f "$current" && ! -L "$current" ]] || { err "ERROR: invalid runtime cursor requires explicit recovery"; exit 1; }
    exit 0
  fi
  if install_runtime_adopted; then
    err "ERROR: adopted runtime has no current cursor; use explicit Rust recovery, never a v1 seed"
    exit 1
  fi
  temp="$(mktemp "$TARGET/.vibe/run/flow/.state-seed.XXXXXX")"
  if [[ -f "$TARGET/.agents/skills/flow/state.json" ]]; then
    cp "$TARGET/.agents/skills/flow/state.json" "$temp"
  else
    cp "$TARGET/.agents/skills/flow/state.example.json" "$temp"
  fi
  mv -f "$temp" "$current"
)

# ── uninstall ────────────────────────────────────────────────────────────────
# Remove managed artifacts while preserving user content: the spec root (docs/spec/ or the legacy tree) is never
# touched, user AGENTS.md prose outside the markers is preserved (block removed
# via merge-agents.sh's marker-pairing guard), and the flow cursor survives
# unless --yes. Composes with --only (remove one half) and --dry-run (preview).
if [[ "$UNINSTALL" -eq 1 ]]; then
  if [[ "$WANT_FLOW" -eq 1 && "$ASSUME_YES" -eq 1 && "$DRY_RUN" -eq 0 ]]; then
    install_runtime_lock || exit 1
    if install_runtime_adopted; then
      err "ERROR: adopted runtime removal requires coordinated Rust recovery; state and installed resources preserved"
      exit 1
    fi
  fi
  note "uninstalling vibe from $TARGET"
  SRC_MERGE="$SRC/flow/scripts/merge-agents.sh"
  SRC_MERGE_SETTINGS="$SRC/flow/scripts/merge-settings.sh"

  if [[ "$WANT_FLOW" -eq 1 && -f "$TARGET/.agents/skills/vibe/SKILL.md" ]] && grep -q '^# Vibe toolkit$' "$TARGET/.agents/skills/vibe/SKILL.md"; then
    [[ "$DRY_RUN" -eq 1 ]] || remove_shipped "$SRC/skills/vibe" "$TARGET/.agents/skills/vibe" tests AGENTS.md
    [[ "$DRY_RUN" -eq 1 ]] || unregister_skill vibe
  fi

  # Per-file inverse of the install copy (uninstall lesson): remove exactly the
  # relative paths the source bundle ships — minus the artifacts install scrubs
  # before shipping (tests/, contributor AGENTS.md) — and prune emptied dirs.
  # A user file dropped into either shared skills dir is never touched.
  #
  # The exclude lists below are deliberately TOP-LEVEL-ONLY, and are NOT the
  # mirror image of scrub_source_only()'s any-depth walk. A deeper source-only
  # artifact that a PREVIOUS version of this installer shipped (js-core's
  # `engine/tests`, 17 files) is still in the source bundle, so remove_shipped
  # names it and cleans it up; excluding it here would strand 396K of dead test
  # code in every target that was installed before the scrub was fixed. Both
  # legs are exercised by the adapters suite, on a legacy-shaped target.
  if [[ "$WANT_SPEC" -eq 1 && -e "$TARGET/.agents/skills/spec" ]]; then
    say "remove the shipped files under .agents/skills/spec (user files preserved)"
    [[ "$DRY_RUN" -eq 1 ]] || remove_shipped \
      "$SRC/spec" "$TARGET/.agents/skills/spec" tests AGENTS.md
  fi
  if [[ "$WANT_SPEC" -eq 1 && -e "$TARGET/.agents/skills/using-spec" ]]; then
    say "remove the shipped files under .agents/skills/using-spec (user files preserved)"
    [[ "$DRY_RUN" -eq 1 ]] || remove_shipped \
      "$SRC/spec/using-spec" "$TARGET/.agents/skills/using-spec" tests AGENTS.md
  fi
  # Undo the spec skill registration — only our own .claude/skills/spec symlink.
  if [[ "$WANT_SPEC" -eq 1 ]]; then
    say "unregister .claude/skills/spec (only the vibe symlink; a user entry is kept)"
    [[ "$DRY_RUN" -eq 1 ]] || unregister_skill spec
    say "unregister .claude/skills/using-spec (only the vibe symlink; a user entry is kept)"
    [[ "$DRY_RUN" -eq 1 ]] || unregister_skill using-spec
  fi

  if [[ "$WANT_FLOW" -eq 1 ]]; then
    # Undo the vibe skill registration — only our own .claude/skills/vibe symlink.
    say "unregister .claude/skills/vibe (only the vibe symlink; a user entry is kept)"
    [[ "$DRY_RUN" -eq 1 ]] || unregister_skill flow
    if [[ -e "$TARGET/.agents/skills/flow" ]]; then
      # Exclude the top-level artifacts install scrubs (tests/, AGENTS.md,
      # evidence/) plus the per-project runtime state install never ships
      # (state.json, warnings.log): remove_shipped therefore leaves the cursor +
      # receipts intact by construction. --yes then removes those runtime files
      # too. Deeper source-only artifacts are intentionally left OUT of this list
      # so a legacy target's shipped `engine/tests` is cleaned up — see the
      # header note above remove_shipped's first call site.
      if [[ "$ASSUME_YES" -eq 0 ]]; then
        say "remove the shipped files under .agents/skills/flow (preserving the flow cursor and evidence receipts; re-run with --yes to remove them)"
      else
        say "remove .agents/skills/flow including the flow cursor and evidence receipts"
      fi
      if [[ "$DRY_RUN" -eq 0 ]]; then
        remove_shipped "$SRC/flow" "$TARGET/.agents/skills/flow" \
          tests AGENTS.md evidence state.json warnings.log
        if [[ "$ASSUME_YES" -eq 1 ]]; then
          # A symlinked skill dir is one remove_shipped just left untouched (or
          # unlinked): deleting the runtime files through it would contradict
          # that warning, even when the link resolves inside TARGET.
          if [[ ! -L "$TARGET/.agents/skills/flow" ]] \
            && inside_target "$TARGET/.agents/skills/flow/state.json"; then
            rm -f "$TARGET/.agents/skills/flow/state.json" \
                  "$TARGET/.agents/skills/flow/warnings.log"
            rm -rf "$TARGET/.agents/skills/flow/evidence"
            find "$TARGET/.agents/skills/flow" -type d -empty -delete 2>/dev/null || true
          fi
          if [[ -d "$TARGET/.vibe/run/flow" && ! -L "$TARGET/.vibe/run/flow" ]] && inside_target "$TARGET/.vibe/run/flow/state.json"; then
            rm -f "$TARGET/.vibe/run/flow/state.json"
            [[ -L "$TARGET/.vibe/run/flow/evidence" ]] || rm -rf "$TARGET/.vibe/run/flow/evidence"
          fi
          # The inject marker is runtime state like the cursor. Remove the FILE
          # only, then the directory if that emptied it — `.vibe/blocks/**` is
          # the project's own authored content and is never install's to delete.
          if inside_target "$TARGET/.vibe/last-inject"; then
            rm -f "$TARGET/.vibe/last-inject"
            rmdir "$TARGET/.vibe" 2>/dev/null || true
          fi
        fi
      fi
    fi
    # Runtime cleanup is independent of skill payload presence: a first
    # preserving uninstall may already have removed every shipped skill file.
    if [[ "$ASSUME_YES" -eq 1 && "$DRY_RUN" -eq 0 ]]; then
      if [[ -d "$TARGET/.vibe/run/flow" && ! -L "$TARGET/.vibe/run/flow" ]] && inside_target "$TARGET/.vibe/run/flow/state.json"; then
        rm -f "$TARGET/.vibe/run/flow/state.json"
        [[ -L "$TARGET/.vibe/run/flow/evidence" ]] || rm -rf "$TARGET/.vibe/run/flow/evidence"
      fi
      if inside_target "$TARGET/.vibe/last-inject"; then rm -f "$TARGET/.vibe/last-inject"; fi
    fi
    # --yes removes the cursor + receipts, so fully invert install: strip every
    # stanza install appended to .gitignore, leaving other ignore rules intact.
    # A symlinked .gitignore is not rewritten: the rename would replace the link
    # with a regular file, and the file it points at may be shared. Name the
    # lines to remove by hand instead.
    if [[ "$ASSUME_YES" -eq 1 && -L "$TARGET/.gitignore" ]]; then
      err "WARN: .gitignore is a symlink; left alone. Remove the vibe lines by hand: .agents/skills/flow/state.json, .agents/skills/flow/evidence/, .agents/skills/flow/warnings.log, .vibe/last-inject (and the '# vibe' comment above each)."
    elif [[ "$ASSUME_YES" -eq 1 && -f "$TARGET/.gitignore" ]]; then
      say "strip the vibe cursor, evidence, and warnings stanzas from .gitignore"
      if [[ "$DRY_RUN" -eq 0 ]]; then
        GI="$TARGET/.gitignore"
  if ! { [[ -f "$GI" ]] && grep -qFx ".vibe/run/flow/" "$GI"; }; then
    [[ "$DRY_RUN" -eq 1 ]] || gi_append "$GI" "# vibe canonical runtime" ".vibe/run/flow/"
  fi; GI_TMP="$(mktemp)"
        grep -vxF \
          -e '# vibe mutable flow cursor (runtime; version state-machine.json, not this)' \
          -e '.agents/skills/flow/state.json' \
          -e '# vibe evidence receipts (runtime verification output, not memory)' \
          -e '.agents/skills/flow/evidence/' \
          -e '# vibe warnings relay (runtime warn-first channel; surfaced then truncated)' \
          -e '.agents/skills/flow/warnings.log' \
          -e '# vibe inject edge-detection marker (runtime; records the cursor the last inject saw)' \
          -e '.vibe/last-inject' "$GI" 2>/dev/null \
          | awk 'NF{last=NR} {line[NR]=$0} END{for(i=1;i<=last;i++) print line[i]}' >"$GI_TMP" || true
        if [[ -s "$GI_TMP" ]]; then mv -f "$GI_TMP" "$GI"; else rm -f "$GI_TMP" "$GI"; fi
      fi
    fi
    # Unwire the settings.json entries FIRST — before deleting the hook scripts
    # they reference. If the unwire cannot complete (jq absent), the still-wired
    # settings.json would point at deleted scripts, so LEAVE the hook scripts in
    # place, warn once with the manual step, and continue the rest of uninstall.
    # A .claude that resolves outside TARGET (a symlink shared with another repo)
    # or into a vibe checkout vendored inside it (the source's own settings)
    # is not this repo's to unwire or prune: leave it whole and say so.
    UNWIRE_OK=1 CLAUDE_OURS=1
    if [[ -e "$TARGET/.claude" ]] && ! inside_target "$TARGET/.claude/settings.json"; then
      CLAUDE_OURS=0
      err "WARN: $TARGET/.claude resolves outside $TARGET or into the vibe source via a symlink; its settings, hooks and commands are left untouched."
    fi
    say "unwire the flow hooks from .claude/settings.json (user settings preserved)"
    if [[ "$DRY_RUN" -eq 0 && "$CLAUDE_OURS" -eq 1 && -f "$SRC_MERGE_SETTINGS" ]]; then
      bash "$SRC_MERGE_SETTINGS" unmerge "$TARGET" || UNWIRE_OK=0
    fi
    say "remove the Claude adapter files vibe installed"
    if [[ "$DRY_RUN" -eq 0 && "$CLAUDE_OURS" -eq 1 ]]; then
      if [[ "$UNWIRE_OK" -eq 1 ]]; then
        remove_shipped "$SRC/.claude/commands" "$TARGET/.claude/commands"
        remove_shipped "$SRC/.claude/hooks" "$TARGET/.claude/hooks"
        # Retire artifacts a prior (plugin-based) install left behind — vibe's only.
        retire_vibe_plugin_manifest "$TARGET"
        prune_empty "$TARGET/.claude/hooks"
        rmdir_inside "$TARGET/.claude/skills" "$TARGET/.claude-plugin" "$TARGET/.claude"
      else
        err "WARN: settings.json still wires the flow hooks (jq unavailable) — leaving .claude/hooks/*.sh and .claude/commands in place so the wiring keeps pointing at real files. Remove the vibe hook entries from .claude/settings.json by hand, then delete .claude/hooks and .claude/commands."
      fi
    fi
    # Remove the opt-in adapter symlinks vibe created — but ONLY when they are
    # symlinks that point at AGENTS.md. A user's real file of the same name (or a
    # symlink they aimed elsewhere) is left untouched.
    while IFS=$'\t' read -r _akey afile atarget; do
      [[ -n "$afile" ]] || continue
      link="$TARGET/$afile"
      if [[ -L "$link" && "$(readlink "$link")" == "$atarget" ]]; then
        say "remove adapter symlink $afile -> $atarget"
        [[ "$DRY_RUN" -eq 1 ]] || rm -f "$link"
      fi
    done <<< "$(adapter_rows)"
    if [[ -f "$TARGET/AGENTS.md" ]]; then
      say "remove the managed vibe blocks from AGENTS.md (user prose preserved; a vibe-only stub is deleted unless AGENTS.md is a symlink)"
      [[ "$DRY_RUN" -eq 1 ]] || bash "$SRC_MERGE" unmerge "$TARGET" \
        || err "WARN: AGENTS.md blocks not removed (reversed markers?); left untouched."
    fi

    # Native host surfaces for EVERY host, not just the ones this target
    # installed: a user removing vibe wants it gone from every host. Ownership is
    # decided by the install manifest — the plan's rule is "never remove a
    # shared-directory entry unless the install manifest proves vibe ownership",
    # so a manifest-recorded path is removed and an unrecorded one is not. A
    # target with no usable manifest (installed before the manifest existed) falls
    # back to the shipped-source inverse: the per-file/per-link inverse of the
    # package, which is also what makes a retargeted or collided entry survive.
    # Claude's own surfaces are deliberately NOT manifest-driven: they keep the
    # tested UNWIRE_OK-aware path above, so a failed settings.json unwire still
    # leaves the hook scripts it references in place.
    MANIFEST="$(manifest_path)"
    MAN_ACTIONS=""
    HAVE_MANIFEST=0
    if [[ -f "$MANIFEST" ]]; then
      MAN_ACTIONS="$(manifest_actions)"
      [[ -n "$MAN_ACTIONS" ]] && HAVE_MANIFEST=1
    fi
    if [[ "$HAVE_MANIFEST" -eq 1 ]]; then
      say "remove the host artifacts recorded in the install manifest (user files preserved)"
      while IFS=$'\t' read -r mhost mkind mpath mtarget; do
        [[ -n "$mhost" ]] || continue
        [[ "$mhost" == "claude" ]] && continue
        case "$mkind" in
          file)    if [[ "$DRY_RUN" -eq 1 ]]; then dry_remove_rel "$mpath"; else remove_host_file "$mpath"; fi ;;
          symlink) if [[ -L "$TARGET/$mpath" && "$(readlink "$TARGET/$mpath")" == "$mtarget" ]]; then
                     if [[ "$DRY_RUN" -eq 1 ]]; then dry_remove_rel "$mpath"
                     elif target_owned "$mpath"; then rm -f "$TARGET/$mpath"; fi
                   fi ;;
          merge)   : ;;
        esac
      done <<< "$MAN_ACTIONS"
      if [[ "$DRY_RUN" -eq 0 ]]; then
        # Prune the directories that removal emptied — a co-located user file
        # keeps its directory alive. `plugins/` is only ever the PARENT of our
        # package, so it is not a root above; prune it here too, and only here (a
        # `plugins/` we never wrote into is never touched). A directory behind a
        # symlinked ancestor is not ours to prune.
        prune_empty "$TARGET/.opencode" "$TARGET/plugins/vibe" "$TARGET/factory-plugin" \
             "$TARGET/.agents/plugins" "$TARGET/.factory-plugin"
        rmdir_inside "$TARGET/plugins"
      fi
    else
      note "no usable install manifest — falling back to the shipped-source inverse for host surfaces."
      say "remove the vibe-owned host surfaces shipped by the source (user files preserved)"
      # Both modes: remove_host_package prints the concrete paths it would take
      # in dry-run, so --uninstall --dry-run shows the exact project paths.
      remove_host_package ".opencode"
      remove_host_package "plugins/vibe"
      remove_host_package "factory-plugin"
      for mp in ".agents/plugins/marketplace.json" ".factory-plugin/marketplace.json"; do
        [[ -f "$SRC/$mp" ]] || continue
        if [[ "$DRY_RUN" -eq 1 ]]; then dry_remove_rel "$mp"
        elif target_owned "$mp"; then rm -f "$TARGET/$mp"; fi
      done
      if [[ "$DRY_RUN" -eq 0 ]]; then
        rmdir_inside "$TARGET/.agents/plugins" "$TARGET/.factory-plugin" "$TARGET/plugins"
      fi
    fi
    # The install record goes with the install. `.vibe/blocks/**` is the
    # project's own authored content and is never install's to delete.
    say "remove the install manifest (.vibe/install-manifest.json)"
    if [[ "$DRY_RUN" -eq 0 ]]; then
      if target_owned "${MANIFEST#"$TARGET"/}"; then rm -f "$MANIFEST"; fi
      rmdir_inside "$TARGET/.vibe"
    fi
  fi

  # A half-uninstall that LEAVES the manifest in place (--only spec removes the
  # spec half only) must not leave records naming paths this pass deleted: a stale
  # record is a false claim of vibe ownership. Records whose path survives (the
  # flow half's host artifacts) stay, so a later full uninstall still removes them.
  # A manifest behind a symlinked `.vibe` is left alone with a WARN, as the full
  # uninstall leaves it.
  if [[ "$WANT_FLOW" -eq 0 && -f "$(manifest_path)" ]]; then
    say "prune the install manifest of records for paths this pass removed"
    [[ "$DRY_RUN" -eq 1 ]] || prune_manifest_dead
  fi

  if [[ "$DRY_RUN" -eq 0 ]]; then
    rmdir_inside "$TARGET/.claude/skills" "$TARGET/.claude"
    rmdir_inside "$TARGET/.agents/skills" "$TARGET/.agents"
  fi

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "install: [dry-run] uninstall plan complete — nothing was written."
  else
    note "uninstall done. Preserved: docs/spec/, $LEGACY_SPEC/ and user AGENTS.md prose."
  fi
  exit 0
fi

# Preserve the old workflow and runtime before its public name becomes toolkit.
if [[ "$WANT_FLOW" -eq 1 && "$DRY_RUN" -eq 0 ]]; then
  bash "$SRC/flow/scripts/migrate-discovery.sh" "$TARGET" || exit 1
  for owned in vibe flow; do
    entry="$TARGET/.agents/skills/$owned"
    if [[ -e "$entry" || -L "$entry" ]]; then
      case "$owned" in
        vibe) grep -q '^# Vibe toolkit$' "$entry/SKILL.md" 2>/dev/null || { err "ERROR: foreign vibe skill preserved: $entry"; exit 1; } ;;
        flow) [[ -f "$entry/state-machine.json" && -f "$entry/scripts/set-state.sh" ]] || { err "ERROR: foreign flow skill preserved: $entry"; exit 1; } ;;
      esac
    fi
  done
fi
if [[ "$DRY_RUN" -eq 0 ]]; then
  # Retire only an unchanged shipped bootstrap. Preserve modified/user content.
  old="$TARGET/.agents/skills/using-spec"
  if [[ -f "$old/SKILL.md" ]] && cmp -s "$old/SKILL.md" "$SRC/flow/reference/legacy-using-spec.md"; then
    rm -f "$old/SKILL.md"
    rmdir "$old" 2>/dev/null || true
  elif [[ -e "$old" ]]; then
    err "WARN: foreign or modified using-spec preserved; remove via its owner if obsolete"
  fi
  unregister_skill using-spec
fi
if [[ "$WANT_FLOW" -eq 1 ]]; then
say "copy Vibe toolkit guide -> $TARGET/.agents/skills/vibe"
if [[ "$DRY_RUN" -eq 0 ]]; then
  mkdir -p "$TARGET/.agents/skills"
  cp -RL "$SRC/skills/vibe" "$TARGET/.agents/skills/"
fi
if host_selected claude; then
  [[ "$DRY_RUN" -eq 1 ]] || register_skill vibe
fi
fi

# 1. Core (platform-neutral). Copy the spec, using-spec and vibe skill trees —
# three SIBLING dirs, using-spec never left nested inside spec/ (vibe#71 sub-gap
# 3: a host that scans `skills/*/SKILL.md` would miss it). The cursor is
# per-project runtime state: never inherit the source's, never clobber the
# target's — preserve any live target cursor across the copy. Halves are copied
# independently so --only can install one without the other.
if [[ "$WANT_SPEC" -eq 1 || "$WANT_FLOW" -eq 1 ]]; then
  say "copy core skills into $TARGET/.agents/skills/"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    mkdir -p "$TARGET/.agents/skills"
  fi
fi
# -L dereferences: source stores canonical dirs at spec/ + flow/ behind
# .agents/skills/* symlinks; targets always get real directories.
if [[ "$WANT_SPEC" -eq 1 ]]; then
  say "copy spec skill -> $TARGET/.agents/skills/spec"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    cp -RL "$SRC/spec" "$TARGET/.agents/skills/"
    # Source-only artifacts (co-located tests at ANY depth, contributor
    # AGENTS.md) never ship — enumerated from the source, not hardcoded.
    scrub_source_only "$SRC/spec" "$TARGET/.agents/skills/spec"
    # Flatten (vibe#71 sub-gap 3): `using-spec` is copied below as its OWN sibling
    # skill dir. Leaving a second copy nested inside `spec/` registers it twice on
    # a host that walks a skill tree recursively and hides it on a host that does
    # not (OpenCode documents `skills/*/SKILL.md`).
    rm -rf "$TARGET/.agents/skills/spec/using-spec"
  fi

fi
if [[ "$WANT_FLOW" -eq 1 ]]; then
  say "copy vibe flow skill -> $TARGET/.agents/skills/flow"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    # Stage and scrub before overlay: never copy source runtime files over a
    # target cursor/receipt and then attempt to restore them outside its lock.
    FLOW_STAGE="$(mktemp -d)"
    cp -RL "$SRC/flow" "$FLOW_STAGE/flow"
    scrub_source_only "$SRC/flow" "$FLOW_STAGE/flow"
    rm -f "$FLOW_STAGE/flow/state.json"
    cp -RL "$FLOW_STAGE/flow" "$TARGET/.agents/skills/"
    scrub_source_only "$SRC/flow" "$TARGET/.agents/skills/flow" preserve-runtime
    rm -rf "$FLOW_STAGE"

  fi
fi

# 1b. Register the copied skills under .claude/skills so /spec and the vibe skill
# resolve where the docs say. Each is a relative symlink into the core copied in
# step 1; a user entry already occupying the slot is preserved (register_skill
# warns + skips). Registration follows the half that was installed, AND the
# platform selection: `.claude/skills` is a Claude Code surface, so `--platform
# none` (or any selection without claude) registers nothing.
if [[ "$WANT_SPEC" -eq 1 ]]; then
  # Belongs to the spec half, not the adapter step: --only spec must still get
  # runnable scripts if the source ever loses its exec bits
  # (core.fileMode=false, a zip download).
  [[ "$DRY_RUN" -eq 1 ]] || chmod +x "$TARGET"/.agents/skills/spec/scripts/*.sh 2>/dev/null || true
  if host_selected claude; then
    say "register spec skill at .claude/skills/spec -> ../../.agents/skills/spec"
    [[ "$DRY_RUN" -eq 1 ]] || register_skill spec

  fi
fi
if [[ "$WANT_FLOW" -eq 1 ]] && host_selected claude; then
  say "register flow skill at .claude/skills/flow -> ../../.agents/skills/flow"
  [[ "$DRY_RUN" -eq 1 ]] || register_skill flow
fi

# 2. Claude Code adapter (native /flow command + hook scripts). Adapter belongs
# to the flow half — its hooks drive the flow — and to the claude platform: no
# other host gets these files. Hooks are wired via the target's
# .claude/settings.json (step 3), not a plugin (issue #12: one firing path).
if [[ "$WANT_FLOW" -eq 1 ]] && host_selected claude; then
  say "copy Claude adapter (.claude/commands, .claude/hooks/*.sh)"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    mkdir -p "$TARGET/.claude"
    cp -R "$SRC/.claude/commands" "$TARGET/.claude/"
    cp -R "$SRC/.claude/hooks" "$TARGET/.claude/"
    # vibe's own plugin manifest is retired in 2b, outside this host branch.
    chmod +x "$TARGET"/.claude/hooks/*.sh \
             "$TARGET"/.agents/skills/flow/scripts/*.sh 2>/dev/null || true
  fi
fi

# 2b. Retire vibe's own legacy plugin manifest whenever the FLOW half is
# installed — not only when Claude is a selected host. Host detection is a
# point-in-time answer and the manifest outlives it: the moment Claude Code (or
# any later install that does select claude) opens this target, a dead vibe
# plugin beside the settings.json wiring double-fires every hook. Keeping the
# retire inside the claude-only branch silently skipped it on a host-less
# machine — including CI, where no host CLI is on PATH. A target's OWN plugin
# manifest is still never touched (retire_vibe_plugin_manifest checks the name).
if [[ "$WANT_FLOW" -eq 1 && "$DRY_RUN" -eq 0 ]]; then
  retire_vibe_plugin_manifest "$TARGET"
  rmdir "$TARGET/.claude-plugin" 2>/dev/null || true
fi

# 3. Wire the flow hooks into .claude/settings.json — merge only vibe's three
# entries, idempotently, never clobbering user settings. Graceful-degrade if the
# helper is missing or jq is absent (it prints the snippet to paste). Flow half,
# Claude Code platform.
MERGE_SETTINGS="$SRC/flow/scripts/merge-settings.sh"
# 0 = not wired, 1 = wired. Step 8 reads this instead of claiming success
# unconditionally: a jq-less target gets a pasteable snippet and a banner saying
# the hooks are NOT live, rather than "the flow hooks fire on every turn" over an
# absent settings.json. It also gates the capability report, which may only claim
# hooks when the wiring actually installed.
HOOKS_WIRED=0
if [[ "$WANT_FLOW" -eq 1 ]] && host_selected claude; then
  say "wire flow hooks into .claude/settings.json"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    if [[ -f "$MERGE_SETTINGS" ]]; then
      if bash "$MERGE_SETTINGS" merge "$TARGET"; then
        HOOKS_WIRED=1
      else
        err "WARN: settings.json not wired (see message above)."
      fi
    else
      err "WARN: merge-settings.sh not found; hooks not wired into settings.json."
    fi
  else
    HOOKS_WIRED=1
  fi
fi

# 3b. Native host surfaces for the selected hosts. Each host gets only its
# documented project surfaces (host-surfaces.json is the data); no hook
# integration and no agent/droid registration is added for OpenCode, Codex, or
# Factory Droid. The Claude surfaces were deployed by steps 1b–3 above, so
# `claude` only records them (3c). Host packages carry the FLOW entry point, so
# they belong to the flow half — `--only spec` stays a pure spec install.
if [[ -n "$PLATFORMS" ]]; then
  IFS=',' read -r -a _selected <<< "$PLATFORMS"
  for _h in ${_selected[@]+"${_selected[@]}"}; do
    case "$_h" in
      claude) : ;;  # steps 1b–3; recorded in 3c
      opencode|codex|droid)
        if [[ "$WANT_FLOW" -eq 0 ]]; then
          # Nothing is deployed for this host, so write_manifest must keep its
          # existing records instead of refreshing them to nothing (they are still
          # the account of the files on disk).
          PRESERVE_HOSTS="${PRESERVE_HOSTS:+$PRESERVE_HOSTS,}$_h"
          note "host surface '$_h' skipped: --only spec installs no host entry points."
          continue
        fi
        case "$_h" in
          opencode)
            deploy_package opencode ".opencode" ;;
          codex)
            deploy_package codex "plugins/vibe"
            deploy_file codex "$SRC/.agents/plugins/marketplace.json" ".agents/plugins/marketplace.json" ;;
          droid)
            deploy_package droid "factory-plugin"
            deploy_file droid "$SRC/.factory-plugin/marketplace.json" ".factory-plugin/marketplace.json" ;;
        esac ;;
    esac
  done
elif [[ "$DRY_RUN" -eq 1 ]]; then
  say "configure no native host surface (no host CLI detected; use --platform claude,opencode,codex,droid to select)"
fi

# 3c. Record the Claude surfaces for the manifest. Steps 1b–3 deploy them and
# Claude keeps its own, tested uninstall path (remove_shipped + readlink
# matching), so these entries are the manifest's honest account of what install
# created — not a second removal mechanism for the same files.
if host_selected claude; then
  if [[ "$WANT_FLOW" -eq 1 ]]; then
    while IFS= read -r _f; do
      record_action claude file "${_f#"$SRC"/}"
    done < <(find "$SRC/.claude/commands" "$SRC/.claude/hooks" -type f 2>/dev/null)
    if [[ "$HOOKS_WIRED" -eq 1 ]]; then
      record_action claude merge ".claude/settings.json"
    fi
  fi
  [[ "$WANT_SPEC" -eq 1 ]] && record_action claude symlink ".claude/skills/spec" "../../.agents/skills/spec"
  [[ "$WANT_FLOW" -eq 1 ]] && record_action claude symlink ".claude/skills/vibe" "../../.agents/skills/vibe"
  [[ "$WANT_FLOW" -eq 1 ]] && record_action claude symlink ".claude/skills/flow" "../../.agents/skills/flow"
fi

# 3d. Write the install manifest (step 3c + the host deployments above). Not
# gitignored: a fresh clone must still be able to run the uninstall that reads it.
say "record the created host artifacts in $TARGET/.vibe/install-manifest.json"
[[ "$DRY_RUN" -eq 1 ]] || write_manifest

# 4. Initialize only an unadopted runtime under the shared exclusive lock.
if [[ "$WANT_FLOW" -eq 1 ]]; then
  if [[ "$DRY_RUN" -eq 1 ]]; then
    say "check canonical runtime; seed only absent unadopted state under its lock"
  else
    seed_runtime_cursor || exit 1
  fi
fi

# 5. Ignore the mutable runtime state (flow half only): the flow cursor, the
# evidence receipts, and the warnings relay log. Each line is guarded
# independently so re-runs never duplicate and an upgrade adds only the missing
# line. gi_append keeps a freshly created .gitignore free of a leading blank line.
if [[ "$WANT_FLOW" -eq 1 ]]; then
  GI="$TARGET/.gitignore"
  if ! { [[ -f "$GI" ]] && grep -qFx ".vibe/run/flow/" "$GI"; }; then
    [[ "$DRY_RUN" -eq 1 ]] || gi_append "$GI" "# vibe canonical runtime" ".vibe/run/flow/"
  fi
  if ! { [[ -f "$GI" ]] && grep -qF ".agents/skills/flow/state.json" "$GI"; }; then
    say "add .agents/skills/flow/state.json to .gitignore"
    [[ "$DRY_RUN" -eq 1 ]] || gi_append "$GI" \
      "# vibe mutable flow cursor (runtime; version state-machine.json, not this)" \
      ".agents/skills/flow/state.json"
  fi
  if ! { [[ -f "$GI" ]] && grep -qF ".agents/skills/flow/evidence/" "$GI"; }; then
    say "add .agents/skills/flow/evidence/ to .gitignore"
    [[ "$DRY_RUN" -eq 1 ]] || gi_append "$GI" \
      "# vibe evidence receipts (runtime verification output, not memory)" \
      ".agents/skills/flow/evidence/"
  fi
  if ! { [[ -f "$GI" ]] && grep -qF ".agents/skills/flow/warnings.log" "$GI"; }; then
    say "add .agents/skills/flow/warnings.log to .gitignore"
    [[ "$DRY_RUN" -eq 1 ]] || gi_append "$GI" \
      "# vibe warnings relay (runtime warn-first channel; surfaced then truncated)" \
      ".agents/skills/flow/warnings.log"
  fi
  # The inject hook's edge-detection marker (content.mjs recordInject). NOT
  # optional tidiness: left untracked, git reports the collapsed `?? .vibe/`
  # directory on every turn after the first inject, and the Stop gate's
  # receipt-staleness scan reads that as "changed after the receipt was
  # written" — blocking every *.verify state over a path nobody touched.
  # `.vibe/blocks/**` (a project's own authored content blocks) stays tracked:
  # only the marker file is ignored.
  if ! { [[ -f "$GI" ]] && grep -qF ".vibe/last-inject" "$GI"; }; then
    say "add .vibe/last-inject to .gitignore"
    [[ "$DRY_RUN" -eq 1 ]] || gi_append "$GI" \
      "# vibe inject edge-detection marker (runtime; records the cursor the last inject saw)" \
      ".vibe/last-inject"
  fi
fi

# 6. Merge AGENTS.md via the copied merge script (agent-instructions). The merge
# script and its instruction template live in the flow half, so the merge runs
# only when the flow half is installed (--only spec stays a pure spec install).
MERGE="$TARGET/.agents/skills/flow/scripts/merge-agents.sh"
if [[ "$WANT_FLOW" -eq 1 ]]; then
  if [[ "$DRY_RUN" -eq 1 ]]; then
    say "create or merge AGENTS.md via merge-agents.sh (managed markers only)"
  elif [[ -f "$MERGE" ]]; then
    # merge-agents.sh refuses (exit 1) a dangling or outside-repo AGENTS.md link;
    # that is a warning here, so the render, adapters and banner still run.
    bash "$MERGE" "$TARGET" \
      || err "WARN: AGENTS.md not merged (see message above); the rest of the install continues."
  else
    err "WARN: merge-agents.sh not found; skipping AGENTS.md merge."
  fi

  # 6b. Content layer: render the `agents-md` channel into AGENTS.md's managed
  # vibe:rules block. Needs node (the engine); without it the instructions block
  # above is still complete, so this degrades to a note, never a failure.
  ENGINE_CLI="$TARGET/.agents/skills/flow/engine/cli.mjs"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    say "render the agents-md content channel into AGENTS.md (vibe:rules block)"
  elif [[ -f "$ENGINE_CLI" ]] && command -v node >/dev/null 2>&1; then
    ( cd "$TARGET" && node "$ENGINE_CLI" render agents-md --write ) \
      || err "WARN: agents-md channel not rendered (run 'node .agents/skills/flow/engine/cli.mjs render agents-md --write')."
  else
    err "WARN: node not found; skipping the agents-md content render (AGENTS.md instructions block is still installed)."
  fi
fi

# 7. Opt-in adapter symlinks. These point at AGENTS.md, which only the flow half
# provisions — so they are meaningless (and merge-agents.sh is absent) under
# --only spec.
if [[ -n "$ADAPTERS" ]]; then
  if [[ "$WANT_FLOW" -eq 0 ]]; then
    err "WARN: --adapters is skipped under --only spec (adapter symlinks need the flow half's AGENTS.md)."
  else
    IFS=',' read -r -a chosen <<< "$ADAPTERS"
    rows="$(adapter_rows)"
    known=""
    while IFS=$'\t' read -r akey _af _at; do
      [[ -n "$akey" ]] && known="${known:+$known, }$akey"
    done <<< "$rows"
    for a in ${chosen[@]+"${chosen[@]}"}; do
      afile=""
      while IFS=$'\t' read -r akey af _at; do
        if [[ "$akey" == "$a" ]]; then afile="$af"; break; fi
      done <<< "$rows"
      if [[ -z "$afile" ]]; then
        err "WARN: unknown adapter '$a' (known: $known)."
        continue
      fi
      say "symlink $afile -> AGENTS.md"
      if [[ "$DRY_RUN" -eq 0 ]] && ! bash "$MERGE" link "$afile" "$TARGET"; then
        held="$(readlink "$TARGET/$afile" 2>/dev/null || true)"
        err "WARN: $afile not linked (existing file or symlink, not replaced${held:+; it points at $held})."
      fi
    done
  fi
fi

# 7b. Companion plugins (opt-in --with-plugins). A user-level set applied across
# repos, installed via the claude CLI; graceful-degrade when it is absent.
if [[ "$WITH_PLUGINS" -eq 1 ]]; then
  install_companion_plugins
fi

# 8. Completion guidance — tailored to what was actually installed.
if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "install: [dry-run] plan complete — nothing was written."
  exit 0
fi

# entry_recorded HOST PATH — did THIS run record PATH for HOST? The report may
# only claim a surface that actually deployed, and the manifest record is the
# proof of it: a colliding entry (a user file we refused to overwrite) records
# nothing, and neither does a package missing from the source.
entry_recorded() {
  local h="$1" p="$2" ahost akind apath atarget
  while IFS=$'\t' read -r ahost akind apath atarget; do
    [[ "$ahost" == "$h" && "$apath" == "$p" ]] && return 0
  done <<< "$ACTIONS"
  return 1
}

# report_hosts — the capability report (R5): one line per CONFIGURED host, with
# the entry mechanism read from the shipped surface matrix, so the report cannot
# drift from the data that declares the surfaces. Hooks are credited to Claude
# Code only, and only when their wiring actually installed; no other host is ever
# credited with hooks or with native agent/droid registration (this feature adds
# none), and the line for each host says only what it got.
#
# R5 also means the claims follow DEPLOYMENT REALITY, not the selection alone: a
# host whose entry surface was not created this run (a --only spec pass deploys
# none; a collision is refused) is reported as not installed rather than named.
report_hosts() {
  local _rh _rinv _rskills _rlabel _rentry _rcl
  if [[ -z "$PLATFORMS" ]]; then
    if [[ "$PLATFORM_GIVEN" -eq 1 ]]; then
      echo "install: hosts: none (--platform none) — no native host surface; host-neutral files only."
    else
      cat <<EOF
install: hosts: none — no supported host CLI detected (claude, opencode, codex, droid).
install:   No native host surface was configured; the host-neutral core above is complete.
install:   Select hosts explicitly with:
install:     ./install.sh <repo> --platform claude,codex   (or --platform none for host-neutral only)
EOF
    fi
    return 0
  fi
  echo "install: hosts: $PLATFORMS ($HOSTS_SOURCE)"
  IFS=',' read -r -a _rep <<< "$PLATFORMS"
  for _rh in ${_rep[@]+"${_rep[@]}"}; do
    host_entry "$_rh"
    # R5 again: name only the skills that ARE installed. --only flow installs no
    # spec half, and the completion banner directly above says so — a skills claim
    # of `{spec,vibe}` contradicted it.
    _rskills=".agents/skills/{spec,flow,vibe}"
    _rcl=".claude/skills/{spec,flow,vibe}"
    if [[ "$WANT_SPEC" -eq 0 ]]; then
      _rskills=".agents/skills/{vibe}"
      _rcl=".claude/skills/vibe"
    fi
    case "$_rh" in
      claude)   _rlabel="claude    "; _rskills="$_rskills + $_rcl" ;;
      opencode) _rlabel="opencode  " ;;
      codex)    _rlabel="codex     "; _rskills="$_rskills + plugins/vibe/skills" ;;
      droid)    _rlabel="droid     "; _rskills="$_rskills + factory-plugin/skills" ;;
      *)        _rlabel="$_rh " ;;
    esac
    # --only spec deploys no host entry point at all (step 3b skips them), so name
    # no surface: the previous line named /flow, .claude/skills, and a hooks state
    # for steps that never ran.
    if [[ "$WANT_FLOW" -eq 0 ]]; then
      echo "install:   ${_rlabel}— not deployed (--only spec installs no host entry point)"
      continue
    fi
    _rinv="$HOST_INV"
    if [[ "$HOST_KIND" == "skill" ]]; then
      _rinv="$HOST_INV (skill picker; no synthetic slash command)"
    elif [[ -n "$HOST_PATH" ]]; then
      _rinv="$HOST_INV ($HOST_PATH)"
    fi
    # The one path each host's deployment really creates: its entry point, or (for
    # Codex) the package skill link that IS its entry mechanism.
    case "$_rh" in
      claude)   _rentry=".claude/commands/flow.md" ;;
      opencode) _rentry=".opencode/commands/flow.md" ;;
      codex)    _rentry="plugins/vibe/skills/flow" ;;
      droid)    _rentry="factory-plugin/commands/flow.md" ;;
      *)        _rentry="" ;;
    esac
    if [[ -n "$_rentry" ]] && ! entry_recorded "$_rh" "$_rentry"; then
      # Name no invocation: this host's entry surface is NOT there (the colliding
      # path is the user's), so claiming /flow here would be the R5 drift again.
      echo "install:   ${_rlabel}— NOT installed (collision — see the warning above; your file was left untouched)"
      continue
    fi
    if [[ "$_rh" == "claude" ]]; then
      # Hooks are credited ONLY when their wiring installed (R5). Reaching here
      # means the wiring step ran, so a NOT-wired line points at a real warning.
      if [[ "$HOOKS_WIRED" -eq 1 ]]; then
        echo "install:   $_rlabel$_rinv · skills $_rskills · hooks wired via .claude/settings.json"
      else
        echo "install:   $_rlabel$_rinv · skills $_rskills · hooks NOT wired (see the warning above)"
      fi
    else
      echo "install:   $_rlabel$_rinv · skills $_rskills"
    fi
  done
  return 0
}

if [[ "$WANT_FLOW" -eq 1 ]]; then
  if host_selected claude && [[ "$HOOKS_WIRED" -eq 1 ]]; then
    cat <<EOF
install: done. The Claude Code hooks are wired automatically via
install:   .claude/settings.json — no plugin to register. Reload the project (or
install:   restart Claude Code) and the flow hooks fire on every turn; /flow works
install:   as a native project command from .claude/commands.
install:   Hook scripts self-resolve this project's flow state under
install:   \${CLAUDE_PROJECT_DIR}/.agents/skills/flow.
EOF
  elif host_selected claude; then
    cat <<EOF
install: done, BUT the flow hooks are NOT live: .claude/settings.json could not be
install:   wired (see the warning above — usually a missing jq, or a settings.json
install:   that is not valid JSON). /flow and the skills work; the inject, guard,
install:   and Stop teeth do nothing until the printed snippet is merged in.
EOF
  else
    echo "install: done. The host-neutral core is installed; no Claude Code adapter was selected."
  fi
  if [[ "$WANT_SPEC" -eq 1 ]]; then
    echo "install: the spec, using-spec and vibe skills are installed as project files under .agents/skills/."
    # A local install deliberately creates NO project content — the spec tree is
    # yours, and orders.sh's self-location is regression-tested against a target
    # with no marker at all. But AGENTS.md opens by telling the agent to read the
    # spec lessons, so name the one command that makes that true.
    if [[ ! -d "$TARGET/docs/spec" && ! -d "$TARGET/$LEGACY_SPEC" ]]; then
      echo "install: next: seed your spec tree (docs/spec/ or $LEGACY_SPEC/) —"
      echo "install:   bash .agents/skills/spec/scripts/setup.sh   (or /flow setup.detect)"
    fi
  else
    cat <<EOF
install: the vibe (flow) skill is installed under .agents/skills/flow.
install: no spec framework was installed (--only flow) — the spec authoring
install:   commands and spec/scripts/validate.sh are absent in this target.
EOF
  fi
else
  cat <<EOF
install: done (spec framework only).
install: the spec skill is installed as a project file under .agents/skills/spec.
install: no flow harness or Claude adapter was installed (--only spec).
install: run without --only to add the flow harness + Claude Code adapter.
EOF
fi
report_hosts
