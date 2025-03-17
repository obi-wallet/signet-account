/* eslint id-denylist: 0 */

/* eslint-disable import/no-nodejs-classes */
import { EventEmitter } from 'events';

export type RpcRequest = {
  jsonrpc?: string;
  id?: number;
  method: string;
  /* eslint-disable @typescript-eslint/no-explicit-any */
  params?: any;
};

export type RpcResponse = {
  jsonrpc: string;
  id?: number;
  /* eslint-disable @typescript-eslint/no-explicit-any */
  result?: any;
  error?: RpcError;
};

export type RpcError = {
  code: number;
  message: string;
  /* eslint-disable @typescript-eslint/no-explicit-any */
  data?: any;
};

type PromiseCache = {
  resolve: (message: unknown) => void;
  reject: (reason: any) => void;
};

export class WebSocketClient extends EventEmitter {
  messageId: number;

  messageRequests: Map<number, PromiseCache>;

  /* eslint-disable no-restricted-globals */
  websocket: WebSocket;

  connected: boolean;

  queue: RpcRequest[];

  constructor() {
    super();
    this.messageId = 0;
    this.messageRequests = new Map();
    this.connected = false;
    this.queue = [];
  }

  connect(url: string): boolean {
    // In the process of connecting so the caller needs to wait and retry later
    if (this.websocket && this.websocket.readyState === 0) {
      return false;
    }

    // If the connection is open let's close it first
    if (this.websocket && this.websocket.readyState === 1) {
      this.websocket.close();
    }

    /* eslint-disable no-restricted-globals */
    this.websocket = new WebSocket(url);
    this.websocket.onopen = (/* event */) => {
      this.connected = true;

      // Some routes make requests before the connection
      // has been established
      while (this.queue.length > 0) {
        const message = this.queue.shift();
        this.notify(message);
      }

      this.emit('open');
    };

    this.websocket.onclose = (/* event */) => {
      this.connected = false;
      this.emit('close');
    };

    this.websocket.onmessage = (messageEvent) => {
      const msg = JSON.parse(messageEvent.data);

      // Got a promise to resolve
      if (msg.id > 0 && this.messageRequests.has(msg.id)) {
        const { resolve } = this.messageRequests.get(msg.id);
        resolve(msg);
        this.messageRequests.delete(msg.id);
        // Without an `id` we treat as a broadcast message
      } else if (msg.error) {
        throw new Error(msg.error.message);
      } else if (msg.result) {
        // Expects a tuple of (event, payload)
        if (Array.isArray(msg.result)) {
          const [event, payload] = msg.result;
          this.emit(event, payload);
        }
      }
    };

    return true;
  }

  notify(message: RpcRequest): void {
    message.jsonrpc = '2.0';
    if (this.connected) {
      this.websocket.send(JSON.stringify(message));
    } else {
      this.queue.push(message);
      // Try to reconnect
      this.connect(this.websocket.url);
    }
  }

  async rpc(message: RpcRequest): Promise<any> {
    this.messageId += 1;
    const id = this.messageId;
    const promise = new Promise((_resolve, reject) => {
      const resolve = (response: RpcResponse) => {
        if (response.error) {
          return reject(new Error(response.error.message));
        }
        return _resolve(response.result);
      };

      this.messageRequests.set(id, { resolve, reject });
    });
    message.id = id;
    this.notify(message);
    return promise;
  }
}
