// Minimal UTF-8 TextEncoder for embedded V8, which ships without Web APIs.
if (typeof globalThis.TextEncoder === "undefined") {
  class Utf8Encoder {
    readonly encoding = "utf-8";
    encode(input = ""): Uint8Array {
      const out: number[] = [];
      for (const ch of input) {
        const c = ch.codePointAt(0) as number;
        if (c < 0x80) out.push(c);
        else if (c < 0x800) out.push(0xc0 | (c >> 6), 0x80 | (c & 63));
        else if (c < 0x10000) out.push(0xe0 | (c >> 12), 0x80 | ((c >> 6) & 63), 0x80 | (c & 63));
        else out.push(0xf0 | (c >> 18), 0x80 | ((c >> 12) & 63), 0x80 | ((c >> 6) & 63), 0x80 | (c & 63));
      }
      return new Uint8Array(out);
    }
    encodeInto(): never {
      throw new Error("encodeInto is not supported");
    }
  }
  (globalThis as unknown as { TextEncoder: unknown }).TextEncoder = Utf8Encoder;
}
if (typeof globalThis.MessageChannel === "undefined") {
  // Referenced at load time by React's server build; renderToString never schedules work through it.
  class NoopMessageChannel {
    port1 = { postMessage() {}, onmessage: null };
    port2 = { postMessage() {}, onmessage: null };
  }
  (globalThis as unknown as { MessageChannel: unknown }).MessageChannel = NoopMessageChannel;
}
