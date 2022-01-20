import * as T from "../generated/types.js";
import * as RPC from "../generated/jsonrpc.js";
import { RawClient } from "../generated/client.js";
import { WebsocketClient } from "yerpc";
import { eventIdToName } from "./events.js";

export type Opts = {
  url: string;
};

export const DEFAULT_OPTS: Opts = {
  url: "ws://localhost:20808/ws",
};

type WireEvent = { id: number; contextId: number; field1: any; field2: any };
export type DeltachatEventData = WireEvent & { name: string };
export type DeltachatEvent = MessageEvent<DeltachatEventData>;

export class Deltachat extends EventTarget {
  rpc: RawClient;
  opts: Opts;
  transport: WebsocketClient;
  account?: T.Account;
  constructor(opts: Opts | string | undefined) {
    super();
    if (typeof opts === "string") opts = { url: opts };
    if (opts) this.opts = { ...DEFAULT_OPTS, ...opts };
    else this.opts = { ...DEFAULT_OPTS };

    this.transport = new WebsocketClient(this.opts.url);
    this.rpc = new RawClient(this.transport);

    this.transport.addEventListener("request", (event: Event) => {
      const request = (event as MessageEvent<RPC.Request>).data;
      const method = request.method;
      if (method === "event") {
        const params = request.params! as WireEvent;
        const name = eventIdToName(params.id);
        const data: DeltachatEventData = { ...params, name };
        const event = new MessageEvent<DeltachatEventData>("event", { data });
        this.dispatchEvent(event);
      }
    });
  }

  async selectAccount(id: number) {
    await this.rpc.selectAccount(id);
    this.account = await this.rpc.getAccountInfo(id);
  }

  async listAccounts(): Promise<T.Account[]> {
    return await this.rpc.getAllAccounts();
  }
}
