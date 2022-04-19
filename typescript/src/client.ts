import * as T from "../generated/types.js";
import * as RPC from "../generated/jsonrpc.js";
import { RawClient } from "../generated/client.js";
import { WebsocketClient, ClientHandler as yerpcClientHandler } from "yerpc";
import { eventIdToName } from "./events.js";
import { TinyEmitter } from "tiny-emitter";

type event_names = ReturnType<typeof eventIdToName> | "ALL";

type WireEvent = { id: number; contextId: number; field1: any; field2: any };
export type DeltachatEvent = WireEvent & { name: event_names };

type events = Record<event_names, (event: DeltachatEvent) => void>;

export class ModularDeltachat<
  Transport extends yerpcClientHandler
> extends TinyEmitter<events> {
  rpc: RawClient;
  account?: T.Account;
  constructor(protected transport: Transport) {
    super();
    this.rpc = new RawClient(this.transport);

    this.transport.on("request", (request) => {
      const method = request.method;
      if (method === "event") {
        const params = request.params! as WireEvent;
        const name = eventIdToName(params.id);
        const event = { name, ...params };
        this.emit(name, event);
        this.emit("ALL", event);

        if (this.contextEmitters[params.contextId]) {
          this.contextEmitters[params.contextId].emit(name, event);
          this.contextEmitters[params.contextId].emit("ALL", event);
        }
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

  private contextEmitters: TinyEmitter<events>[] = [];
  getContextEvents(account_id: number) {
    if (this.contextEmitters[account_id]) {
      return this.contextEmitters[account_id];
    } else {
      this.contextEmitters[account_id] = new TinyEmitter();
      return this.contextEmitters[account_id];
    }
  }
}

export type Opts = {
  url: string;
};

export const DEFAULT_OPTS: Opts = {
  url: "ws://localhost:20808/ws",
};
export class Deltachat extends ModularDeltachat<WebsocketClient> {
  opts: Opts;
  close() {
    this.transport._socket.close();
  }
  constructor(opts: Opts | string | undefined) {
    if (typeof opts === "string") opts = { url: opts };
    if (opts) opts = { ...DEFAULT_OPTS, ...opts };
    else opts = { ...DEFAULT_OPTS };

    super(new WebsocketClient(opts.url));
    this.opts = opts;
  }
}
