import { execFileSync } from "node:child_process";

// Inspect the processes themselves: state.json can be stale or overwritten by
// another launch, and the debugging port does not identify a profile owner.
export function parseBrowserProcesses(output, userDataDirs) {
  return output.split("\n").flatMap((line) => {
    const match = line.trim().match(/^(\d+)\s+(.+)$/);
    if (!match) return [];
    const [, pid, command] = match;
    if (/(?:^|\s)--type=/.test(command)) return [];
    const userDataDir = userDataDirs.find((dir) => {
      const flag = `--user-data-dir=${dir}`;
      const index = command.indexOf(flag);
      return index > 0 && /\s/.test(command[index - 1]) &&
        (index + flag.length === command.length || /\s/.test(command[index + flag.length]));
    });
    if (!userDataDir) return [];
    const port = command.match(/(?:^|\s)--remote-debugging-port=(\d+)(?:\s|$)/);
    return [{ pid: Number(pid), port: port ? Number(port[1]) : null, userDataDir }];
  });
}

export function findBrowserProcesses(userDataDirs) {
  // Fail closed if process inspection fails.
  return parseBrowserProcesses(
    execFileSync("ps", ["-axo", "pid=,command="], { encoding: "utf8" }),
    userDataDirs,
  );
}
