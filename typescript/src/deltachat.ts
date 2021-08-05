import { RawApi } from "./bindings";

import WebSocket from "isomorphic-ws";
import { JSON_RPC_Error } from "./json_rpc_error";
import { EventEmitter } from "eventemitter3";
import { get_event_name_from_id } from "./events";

export class DeltaChat extends EventEmitter<
  ReturnType<typeof get_event_name_from_id> | "socket_connection_change",
  any
> {
  raw_api: RawApi = new RawApi(this.call.bind(this));
  private backend_connection: boolean = false;

  private callbacks: {
    [invocation_id: number]: { res: Function; rej: Function };
  } = {};
  private invocation_id_counter = 1;

  private socket: WebSocket;

  constructor(public address: string) {
    super();
  }

  async connect(): Promise<void> {
    return new Promise((res, rej) => {
      console.log("connecting to", this.address);
      this.socket = new WebSocket(this.address);
      const self = this; // socket event callback overwrites this to undefined sometimes

      this.socket.addEventListener("message", this.onMessage.bind(self));
      this.socket.addEventListener("error", (event) => {
        console.error(event);
        // TODO handle error
        self.backend_connection = false;
        this.emit("socket_connection_change", false)
        rej("socket error");
      });
      this.socket.addEventListener("close", (event) => {
        console.debug("socket is closed now");
        self.backend_connection = false;
        this.emit("socket_connection_change", false)
      });
      this.socket.addEventListener("open", (event) => {
        console.debug("socket is open now");
        self.backend_connection = true;
        this.emit("socket_connection_change", true)
        res();
      });
    });
  }

  private onMessage(event: { data: any; type: string; target: WebSocket }) {
    // handle answer
    // console.debug({ event });
    let answer;
    try {
      answer = JSON.parse(event.data);
    } catch (error) {
      console.log("message recieved is not valid json:", event.data, error);
      return;
    }
    console.debug("<--", answer);
    if (answer.method === "event") {
      if (!answer.params) {
        throw new Error("invalid event, data missing");
      }
      this.emit(
        get_event_name_from_id(answer.params.id),
        answer.params.field1,
        answer.params.field2
      );
    } else {
      // handle command results
      if (!answer.id) {
        throw new Error("invocation_id missing");
      }
      const callback = this.callbacks[answer.id];
      if (!callback) {
        throw new Error(`No callback found for invocation_id ${answer.id}`);
      }

      if (answer.error) {
        callback.rej(
          new JSON_RPC_Error(
            answer.error.code,
            answer.error.message,
            answer.error.data
          )
        );
      } else {
        callback.res(answer.result || null);
      }

      this.callbacks[answer.id] = null;
    }
  }

  private call(method: string, params?: any): Promise<any> {
    if (!this.backend_connection) throw new Error("Not connected to backend");

    let callback: { res: Function; rej: Function };
    const promise = new Promise((res, rej) => {
      callback = { res, rej };
    });
    const invocation_id = this.invocation_id_counter++;
    this.callbacks[invocation_id] = callback;

    let data = {
      jsonrpc: "2.0",
      method,
      id: invocation_id,
      params,
    };

    try {
      // make sure all errors are contained in the promise result
      console.debug("-->", data);
      this.socket.send(JSON.stringify(data));
      return promise;
    } catch (error) {
      return Promise.reject(error);
    }
  }

  _currentCallCount() {
    return Object.keys(this.callbacks).length;
  }

  _currentUnresolvedCallCount() {
    return Object.keys(this.callbacks).filter(
      (key) => this.callbacks[Number(key)] !== null
    ).length;
  }
}
