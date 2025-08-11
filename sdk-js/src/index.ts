export type ClientOptions = {
  baseUrl?: string;               // default http://127.0.0.1:8545
  apiKey: string;                 // X-Api-Key
  fetchImpl?: typeof fetch;       // optional custom fetch (node polyfill etc.)
};

export class DxidClient {
  private base: string;
  private key: string;
  private f: typeof fetch;

  constructor(opts: ClientOptions) {
    this.base = (opts.baseUrl ?? "http://127.0.0.1:8545").replace(/\/+$/, "");
    this.key = opts.apiKey;
    this.f = opts.fetchImpl ?? (globalThis as any).fetch;
    if (!this.f) throw new Error("No fetch available; pass fetchImpl in opts");
  }

  async status() {
    const r = await this.f(`${this.base}/status`);
    if (!r.ok) throw new Error(`status ${r.status}`);
    return r.json();
    // {height, last_block_hash, state_root, chain_id}
  }

  async balance(addrHex: string) {
    const r = await this.f(`${this.base}/balance/${addrHex}`, {
      headers: { "X-Api-Key": this.key },
    });
    if (!r.ok) throw new Error(`balance ${r.status}`);
    return r.json() as Promise<{ exists: boolean; balance: string; nonce: number }>;
  }

  async submitTx(tx: {
    from: string; // hex32
    to: string;   // hex32
    amount: string | number;
    fee: string | number;
    signature: any;
  }) {
    const r = await this.f(`${this.base}/submitTx`, {
      method: "POST",
      headers: { "Content-Type": "application/json", "X-Api-Key": this.key },
      body: JSON.stringify(tx),
    });
    if (!r.ok) throw new Error(`submitTx ${r.status}`);
    return r.json() as Promise<{ queued: boolean; file: string }>;
  }

  watch(onEvent: (e: any) => void) {
    // Server-Sent Events
    const url = `${this.base}/watch`;
    const es = new EventSource(url);
    es.onmessage = (msg) => {
      try { onEvent(JSON.parse(msg.data)); }
      catch { /* ignore */ }
    };
    return () => es.close();
  }
}
