/* tslint:disable */
/* eslint-disable */
/**
*/
export enum KeyEvent {
  W,
  A,
  S,
  D,
  Space,
}
/**
*/
export class ResourcesWrapper {
  free(): void;
/**
*/
  constructor();
/**
* @param {Uint8Array} data
*/
  set_debug_font_data(data: Uint8Array): void;
/**
* @param {HTMLImageElement} image
*/
  set_sprites(image: HTMLImageElement): void;
/**
* @param {Uint8Array} data
*/
  set_sprites_metadata(data: Uint8Array): void;
/**
* @param {HTMLImageElement} image
*/
  set_noise(image: HTMLImageElement): void;
/**
* @param {string} src
*/
  set_aesthetic_shader(src: string): void;
/**
* @param {string} src
*/
  set_menu_shader(src: string): void;
}
/**
*/
export class Wrapper {
  free(): void;
/**
* @param {HTMLCanvasElement} canvas
* @param {number} time_ms
* @param {ResourcesWrapper} resources
*/
  constructor(canvas: HTMLCanvasElement, time_ms: number, resources: ResourcesWrapper);
/**
* @param {number} time_ms
*/
  step(time_ms: number): void;
/**
*/
  handle_resize(): void;
/**
* @param {number} key_code
*/
  handle_key_down(key_code: number): void;
/**
* @param {number} key_code
*/
  handle_key_up(key_code: number): void;
/**
* @param {boolean} is_left_button
*/
  handle_mouse_down(is_left_button: boolean): void;
/**
* @param {boolean} is_left_button
*/
  handle_mouse_up(is_left_button: boolean): void;
/**
* @param {number} x
* @param {number} y
*/
  handle_mouse_move(x: number, y: number): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_resourceswrapper_free: (a: number) => void;
  readonly resourceswrapper_new: () => number;
  readonly resourceswrapper_set_debug_font_data: (a: number, b: number, c: number) => void;
  readonly resourceswrapper_set_sprites: (a: number, b: number) => void;
  readonly resourceswrapper_set_sprites_metadata: (a: number, b: number, c: number) => void;
  readonly resourceswrapper_set_noise: (a: number, b: number) => void;
  readonly resourceswrapper_set_aesthetic_shader: (a: number, b: number, c: number) => void;
  readonly resourceswrapper_set_menu_shader: (a: number, b: number, c: number) => void;
  readonly __wbg_wrapper_free: (a: number) => void;
  readonly wrapper_new: (a: number, b: number, c: number) => number;
  readonly wrapper_step: (a: number, b: number) => void;
  readonly wrapper_handle_resize: (a: number) => void;
  readonly wrapper_handle_key_down: (a: number, b: number) => void;
  readonly wrapper_handle_key_up: (a: number, b: number) => void;
  readonly wrapper_handle_mouse_down: (a: number, b: number) => void;
  readonly wrapper_handle_mouse_up: (a: number, b: number) => void;
  readonly wrapper_handle_mouse_move: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
}

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {InitInput | Promise<InitInput>} module_or_path
*
* @returns {Promise<InitOutput>}
*/
export default function init (module_or_path?: InitInput | Promise<InitInput>): Promise<InitOutput>;
