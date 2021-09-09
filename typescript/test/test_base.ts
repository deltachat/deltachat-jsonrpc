import { tmpdir } from "os";
import { join } from "path";
import { mkdtemp, rm } from "fs/promises";
import { existsSync } from "fs";
import { spawn } from "child_process";
import { unwrapPromise } from "./ts_helpers";

/* port is not configurable yet */

export const CMD_API_SERVER_PORT = 8080;
export async function startCMD_API_Server(port: typeof CMD_API_SERVER_PORT) {
  const tmp_dir = await mkdtemp(join(tmpdir(), "test_prefix"));

  const path_of_server = join(__dirname, "../../target/debug/webserver");

  if (!existsSync(path_of_server)) {
    throw new Error(
      "server executable does not exist, you need to build it first" +
        "\nserver executable not found at " +
        path_of_server
    );
  }

  const server = spawn(path_of_server, {
    cwd: tmp_dir,
    env: {
      RUST_LOG: "info",
    },
  });

  return {
    close: async () => {
      if (!server.kill(9)) {
        console.log("server termination failed");
      }
      await rm(tmp_dir, { recursive: true });
    },
  };
}

export type CMD_API_Server_Handle = unwrapPromise<
  ReturnType<typeof startCMD_API_Server>
>;
