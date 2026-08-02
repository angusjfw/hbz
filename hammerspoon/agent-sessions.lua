-- Agent session switcher: Hyper+A..R (sent by the keyboard's agent
-- layer) -> switch the tmux client to that slot's session and focus
-- the terminal. State comes from agent-status (see
-- keyboard/session-leds/).

local M = {}

local HYPER = { "ctrl", "alt", "shift", "cmd" }
local LETTERS = "abcdefghijklmnopqr" -- letter i = slot i
local STATE_DIR = os.getenv("HOME") .. "/.local/state/agent-status"
local TERMINAL_APP = "Ghostty"

local function sessionForSlot(slot)
  local ok, iter, dirObj = pcall(hs.fs.dir, STATE_DIR)
  if not ok then return nil end
  for file in iter, dirObj do
    if file:match("%.json$") then
      local entry = hs.json.read(STATE_DIR .. "/" .. file)
      if entry and entry.slot == slot then
        return entry.tmux_session
      end
    end
  end
  return nil
end

local function switchTo(slot)
  local session = sessionForSlot(slot)
  if not session then
    hs.alert.show("no session on key " .. LETTERS:sub(slot, slot):upper())
    return
  end
  -- switch the most recently active tmux client; hs.task avoids the
  -- quote-mangling of hs.execute's login-shell wrapper
  local script = string.format([[
export PATH="/opt/homebrew/bin:/usr/local/bin:$PATH"
client=$(tmux list-clients -F '#{client_activity} #{client_tty}' | sort -rn | head -1 | cut -d' ' -f2)
[ -n "$client" ] && tmux switch-client -c "$client" -t %q
]], session)
  hs.task.new("/bin/sh", nil, { "-c", script }):start()
  hs.application.launchOrFocus(TERMINAL_APP)
end

function M.start()
  for i = 1, #LETTERS do
    hs.hotkey.bind(HYPER, LETTERS:sub(i, i), function() switchTo(i) end)
  end
end

return M
