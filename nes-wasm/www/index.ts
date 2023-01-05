import wasmInit, { NesWasm, KeyState } from "nes-wasm";

const keymap: { [key: string]: number } = {
  ['l']: 7,
  ['k']: 6,
  [' ']: 5,
  ['Enter']: 4,
  ['w']: 3,
  ['s']: 2,
  ['a']: 1,
  ['d']: 0,
};

export class BrowserNes {
  nes: NesWasm;
  mem: WebAssembly.Memory;
  ctx: CanvasRenderingContext2D;
  frame_buffer: Uint8ClampedArray;
  frame_ready: boolean;
  keyboard: KeyState[];
  killed: boolean;
  req_frame: number;
  delay_ms: number;

  constructor(ctx: CanvasRenderingContext2D, mem: WebAssembly.Memory, bincart: Uint8Array) {
    this.nes = NesWasm.new(this, bincart);
    this.ctx = ctx;
    this.mem = mem;
    this.frame_buffer = undefined;
    this.frame_ready = false;
    this.keyboard = new Array(8).fill(KeyState.None);

    document.addEventListener('keydown', (ev) => {
      let btn = keymap[ev.key];
      if (btn != null) {
        this.keyboard[btn] = KeyState.Pressed;
      }
    })

    document.addEventListener('keyup', (ev) => {
      let btn = keymap[ev.key];
      if (btn != null) {
        this.keyboard[btn] = KeyState.Released;
        // console.log(this.keyboard);

        // LOL
        setTimeout(() => {
          this.keyboard[btn] = KeyState.None;
        }, 20);
      }
    })
  }

  poll_keyboard(ptr: number) {
    let state = new Uint8ClampedArray(this.mem.buffer, ptr, 8);
    state.set(this.keyboard)
  }

  on_frame_ready(frame_ptr: number, len: number) {
    if (!this.frame_buffer) {
      this.frame_buffer = new Uint8ClampedArray(this.mem.buffer, frame_ptr, len);
    }

    this.ctx.putImageData(new ImageData(this.frame_buffer, 240, 224), 0, 0);
    this.frame_ready = true;
  }

  delay(millis: number) {
    this.delay_ms = millis;
  }

  loop() {
    const inner = () => {
      this.frame_ready = false;
      while (!this.frame_ready) {
        this.nes.tick();
      }

      if (!this.killed) {
        if (this.delay_ms != 0) {
          setTimeout(() => {
            this.delay_ms = 0;
            this.req_frame = requestAnimationFrame(inner);
          }, this.delay_ms);
        }
        else {
          this.req_frame = requestAnimationFrame(inner);
        }
      }
    }

    this.req_frame = requestAnimationFrame(inner);
  }

  kill() {
    cancelAnimationFrame(this.req_frame);
    this.killed = true;
  }
}

const ctx = document.getElementById('c').getContext('2d');
const wasm = await wasmInit();
let instance = new BrowserNes(ctx, wasm.memory, []); // empty bin == nestest
instance.loop();

document.getElementById('file').addEventListener('change', (input: any) => {
  const file: File = input.target.files[0]
  const reader = new FileReader()
  reader.onload = async () => {
    if (instance != null) {
      instance.kill();
      instance = null;
    }

    const bincart = new Uint8Array(reader.result)
    instance = new BrowserNes(ctx, wasm.memory, bincart);
    instance.loop();
  }
  reader.onerror = () => {
    alert(reader.error)
  }
  if (file) {
    reader.readAsArrayBuffer(file)
  }
})