enum JSON_RPC_Error_Code {
  /** Invalid JSON was received by the server.
   *  An error occurred on the server while parsing the JSON text. */
  ParseError = -32700,
  /** The JSON sent is not a valid Request object. */
  InvalidRequest = -32600,
  /** The method does not exist / is not available. */
  MethodNotFound = -32601,
  /** Invalid method parameter(s). */
  InvalidParams = -32602,

  /** Internal JSON-RPC error. */
  InternalError = -32603,

  /** Reserved for implementation-defined server-errors */
  ServerError,

  /** Error code */
  Custom
}

export class JSON_RPC_Error extends Error {
  readonly code: JSON_RPC_Error_Code;

  constructor(
    readonly code_number: number,
    message: string,
    readonly data?: any
  ) {
    super(message);
    switch (code_number) {
      case -32700:
        this.code = JSON_RPC_Error_Code.ParseError;
        break;
      case -32600:
        this.code = JSON_RPC_Error_Code.InvalidRequest;
        break;
      case -32601:
        this.code = JSON_RPC_Error_Code.MethodNotFound;
        break;
      case -32602:
        this.code = JSON_RPC_Error_Code.InvalidParams;
        break;
      case -32603:
        this.code = JSON_RPC_Error_Code.InternalError;
        break;
      default:
        if (code_number <= -32000 && code_number >= -32099) {
          this.code = JSON_RPC_Error_Code.ServerError;
        } else {
          this.code = JSON_RPC_Error_Code.Custom;
        }
        break;
    }
  }
}
