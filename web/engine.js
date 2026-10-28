(function () {
  let instance = null;
  let loading = null;

  function decodeBase64(text) {
    const binary = atob(text);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    return bytes;
  }

  async function load() {
    if (instance) return instance;
    if (!loading) {
      loading = (async () => {
        if (!window.PIO_WASM_BASE64) throw new Error('WASM do motor não encontrado.');
        const bytes = decodeBase64(window.PIO_WASM_BASE64);
        const result = await WebAssembly.instantiate(bytes, {});
        instance = result.instance;
        return instance;
      })();
    }
    return loading;
  }

  function readResult(inst, ptr) {
    const view = new DataView(inst.exports.memory.buffer);
    const length = Number(view.getBigUint64(ptr, true));
    const text = new TextDecoder('utf-8').decode(new Uint8Array(inst.exports.memory.buffer, ptr + 8, length));
    inst.exports.pio_free(ptr, 8 + length);
    return JSON.parse(text);
  }

  async function calculate(input) {
    const inst = await load();
    const bytes = new TextEncoder().encode(JSON.stringify(input));
    const inPtr = inst.exports.pio_alloc(bytes.length);
    new Uint8Array(inst.exports.memory.buffer, inPtr, bytes.length).set(bytes);
    const outPtr = inst.exports.pio_calculate(inPtr, bytes.length);
    inst.exports.pio_free(inPtr, bytes.length);
    return readResult(inst, outPtr);
  }

  async function version() {
    const inst = await load();
    return readResult(inst, inst.exports.pio_version());
  }

  window.PioEngine = {
    load: load,
    calculate: calculate,
    version: version,
    example: window.PIO_ENGINE_EXAMPLE || null
  };
})();
