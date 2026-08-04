-- Agent session switcher + HUD: Hyper+A..R (sent by the keyboard's
-- agent layer) switches the tmux client to that slot's session; the
-- agent-leds daemon drives a heads-up display and toasts via
-- hammerspoon:// URL events. State comes from agent-status (see
-- keyboard/session-leds/).

local M = {}

local HYPER = { "ctrl", "alt", "shift", "cmd" }
local LETTERS = "abcdefghijklmnopqr" -- what the firmware sends for slot i
-- what the user sees: the physical key for each slot
local KEYS = { "Y", "U", "I", "O", "P", "\\", "H", "J", "K", "L", ";",
               "'", "N", "M", ",", ".", "/", "⇧" }
local STATE_DIR = os.getenv("HOME") .. "/.local/state/agent-status"
local KEYMAPP_SOCK = os.getenv("HOME")
    .. "/Library/Application Support/.keymapp/keymapp.sock"
local TERMINAL_APP = "Ghostty"

local STATE_COLORS = {
  idle        = { red = 1, green = 1, blue = 1 },
  working     = { red = 0, green = 0.4, blue = 1 },
  done        = { red = 0, green = 0.8, blue = 0.2 },
  needs_input = { red = 1, green = 0.8, blue = 0 },
  error       = { red = 1, green = 0, blue = 0 },
  off         = { red = 0.35, green = 0.35, blue = 0.35 },
}
local HUD_FILE = STATE_DIR .. "/hud-visible"

local function entries()
  local out = {}
  local ok, iter, dirObj = pcall(hs.fs.dir, STATE_DIR)
  if not ok then return out end
  for file in iter, dirObj do
    if file:match("%.json$") then
      local e = hs.json.read(STATE_DIR .. "/" .. file)
      if e and e.slot then table.insert(out, e) end
    end
  end
  table.sort(out, function(a, b) return a.slot < b.slot end)
  return out
end

local function sessionForSlot(slot)
  for _, e in ipairs(entries()) do
    if e.slot == slot then return e.tmux_session end
  end
  return nil
end

local function switchTo(slot)
  local session = sessionForSlot(slot)
  if not session then
    hs.alert.show("no session on key " .. KEYS[slot])
    return
  end
  -- switch the most recently active tmux client, then dismiss the
  -- agent layer; hs.task avoids the quote-mangling of hs.execute's
  -- login-shell wrapper
  local script = string.format([[
export PATH="$HOME/.local/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"
client=$(tmux list-clients -F '#{client_activity} #{client_tty}' | sort -rn | head -1 | cut -d' ' -f2)
[ -n "$client" ] && tmux switch-client -c "$client" -t %q
kontroll -p %q set-layer -i 0
]], session, KEYMAPP_SOCK)
  hs.task.new("/bin/sh", nil, { "-c", script }):start()
  hs.application.launchOrFocus(TERMINAL_APP)
end

-- Heads-up display: labelled session grid while the agent layer is on.
-- The daemon toggles a marker file; a pathwatcher on the state dir
-- shows/hides and live-refreshes (much faster than URL events).

local hud = nil

local function hideHud()
  if hud then hud:delete(); hud = nil end
end

local function renderHud()
  local es = entries()
  local rowH, w, pad = 24, 320, 10
  local h = pad * 2 + rowH * math.max(#es, 1)
  local screen = hs.screen.mainScreen():frame()
  if hud then hud:delete() end
  hud = hs.canvas.new({
    x = screen.x + (screen.w - w) / 2, y = screen.y + 48, w = w, h = h,
  })
  hud[1] = {
    type = "rectangle", action = "fill",
    fillColor = { red = 0.08, green = 0.08, blue = 0.08, alpha = 0.88 },
    roundedRectRadii = { xRadius = 10, yRadius = 10 },
  }
  if #es == 0 then
    hud[2] = {
      type = "text", text = "no sessions",
      textColor = { white = 0.7 }, textSize = 13,
      frame = { x = pad, y = pad, w = w - 2 * pad, h = rowH },
    }
  end
  for i, e in ipairs(es) do
    local y = pad + (i - 1) * rowH
    local color = STATE_COLORS[e.state] or { white = 0.5 }
    hud[#hud + 1] = {
      type = "circle", action = "fill", fillColor = color,
      center = { x = pad + 8, y = y + rowH / 2 }, radius = 6,
    }
    hud[#hud + 1] = {
      type = "text",
      text = string.format("%s  %s — %s",
        KEYS[e.slot] or "?",
        e.label or e.tmux_session, e.state),
      textColor = { white = 1 }, textSize = 13,
      frame = { x = pad + 24, y = y + 2, w = w - pad - 24, h = rowH },
    }
  end
  hud:show()
end

local function syncHud()
  if hs.fs.attributes(HUD_FILE) then
    renderHud()
  else
    hideHud()
  end
end

-- Toasts: transient state-change notice, bottom-right

local toasts = {}

local function toast(label, state)
  local w, h = 260, 34
  local screen = hs.screen.mainScreen():frame()
  local c = hs.canvas.new({
    x = screen.x + screen.w - w - 16,
    y = screen.y + screen.h - h - 16 - (#toasts * (h + 8)),
    w = w, h = h,
  })
  c[1] = {
    type = "rectangle", action = "fill",
    fillColor = { red = 0.08, green = 0.08, blue = 0.08, alpha = 0.9 },
    roundedRectRadii = { xRadius = 8, yRadius = 8 },
  }
  c[2] = {
    type = "circle", action = "fill",
    fillColor = STATE_COLORS[state] or { white = 0.5 },
    center = { x = 16, y = h / 2 }, radius = 6,
  }
  c[3] = {
    type = "text", text = string.format("%s — %s", label, state),
    textColor = { white = 1 }, textSize = 13,
    frame = { x = 30, y = 8, w = w - 38, h = h - 10 },
  }
  c:show()
  table.insert(toasts, c)
  hs.timer.doAfter(2.5, function()
    c:delete(0.3)
    for i, t in ipairs(toasts) do
      if t == c then table.remove(toasts, i) break end
    end
  end)
end

local watcher = nil

function M.start()
  for i = 1, #LETTERS do
    hs.hotkey.bind(HYPER, LETTERS:sub(i, i), function() switchTo(i) end)
  end
  watcher = hs.pathwatcher.new(STATE_DIR, syncHud):start()
  syncHud() -- pick up state at load
  hs.urlevent.bind("agent-toast", function(_, params)
    toast(params.label or "session", params.state or "")
  end)
end

return M
