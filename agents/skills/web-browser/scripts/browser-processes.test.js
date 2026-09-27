import assert from "node:assert/strict";
import test from "node:test";
import { parseBrowserProcesses } from "./browser-processes.js";

const dir = "/tmp/skill profile";

test("finds profile owners across ports, excluding helpers and other profiles", () => {
  const output = `
  10 /Applications/Google Chrome --user-data-dir=${dir} --remote-debugging-port=9222 --headless
  11 /Applications/Google Chrome --remote-debugging-port=9333 --user-data-dir=${dir}
  12 /Applications/Google Chrome Helper --type=renderer --user-data-dir=${dir} --remote-debugging-port=9222
  13 /Applications/Google Chrome --user-data-dir=${dir}-other --remote-debugging-port=9222
  14 /Applications/Google Chrome
  15 /Applications/Google Chrome --user-data-dir=${dir}
`;
  assert.deepEqual(parseBrowserProcesses(output, [dir]), [
    { pid: 10, port: 9222, userDataDir: dir },
    { pid: 11, port: 9333, userDataDir: dir },
    { pid: 15, port: null, userDataDir: dir },
  ]);
});

test("handles empty process list", () => {
  assert.deepEqual(parseBrowserProcesses("", [dir]), []);
});
