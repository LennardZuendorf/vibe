// vibe-spec:opencode v1
// Installed by `vibe-spec setup --opencode`. After an edit tool runs, asks
// `vibe-spec hook validate` about the file and appends its findings to the
// tool output. A missing binary or any failure is silent.
import { spawnSync } from "node:child_process"

const EDITS = { write: "Write", edit: "Edit", multiedit: "MultiEdit" }

export const VibeSpec = async ({ directory }) => {
  const pending = new Map()
  return {
    "tool.execute.before": async (input, output) => {
      if (input.callID) pending.set(input.callID, output.args)
    },
    "tool.execute.after": async (input, output) => {
      const name = EDITS[String(input.tool).toLowerCase()]
      const args = input.args ?? pending.get(input.callID) ?? output.args
      pending.delete(input.callID)
      const file = args && (args.filePath ?? args.file_path)
      if (!name || !file) return
      const payload = JSON.stringify({
        hook_event_name: "PostToolUse",
        cwd: directory,
        tool_name: name,
        tool_input: { file_path: file },
      })
      const run = spawnSync("vibe-spec", ["hook", "validate"], {
        input: payload,
        cwd: directory,
        encoding: "utf8",
        timeout: 5000,
      })
      if (run.error || !run.stdout) return
      try {
        const text = JSON.parse(run.stdout)?.hookSpecificOutput?.additionalContext
        if (text) output.output = `${output.output ?? ""}\n${text}`
      } catch {}
    },
  }
}
