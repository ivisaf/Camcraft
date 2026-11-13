// wasi_shim.js - the wasi_snapshot_preview1 imports camcraft-ui.wasm asks a
// browser for (#31). flecs' C is linked against the pinned wasi libc
// (web/build-app-wasm.sh), so the module imports nine WASI calls for its
// clock, its log output, its start-up probe for preopened directories and
// its exit path. A browser has no WASI, so this module answers exactly
// those nine: the clocks from the page's own clocks, stdout/stderr into the
// console, and what a tab cannot do (files, blocking waits) with the WASI
// answer that says so. Every answer that refuses or skips something the
// caller asked for is also said once in the console (`console.warn`), so
// nothing is refused silently. The build script fails by name if the
// module ever asks for an import this file does not export - a tenth call
// is added here on purpose, never answered by a guess.
// Nothing here grants the module anything a page script does not have.
// Provenance: our own code, written from the public WASI preview1
// specification (the witx function signatures, struct layouts and errno
// table).

const ERRNO_SUCCESS = 0;
const ERRNO_BADF = 8;
const ERRNO_INVAL = 28;
const ERRNO_NOSYS = 52;
const ERRNO_NOTSUP = 58;
const ERRNO_SPIPE = 70;
const FILETYPE_CHARACTER_DEVICE = 2;
const CLOCK_REALTIME = 0;
const CLOCK_MONOTONIC = 1;
const EVENTTYPE_CLOCK = 0;
const SUBSCRIPTION_SIZE = 48;
const EVENT_SIZE = 32;

let memory = null;
const pending = { 1: "", 2: "" };
const decoder = new TextDecoder("utf-8");
const said = new Set();

/** Hand the shim the module's linear memory. Call once, right after the
 *  module is instantiated and before any export that can reach flecs. */
export function bind_memory(wasm_memory) {
  memory = wasm_memory;
}

/** A view over the current memory (it is re-read every call because the
 *  buffer is replaced whenever the module grows its memory). */
function view() {
  return memory ? new DataView(memory.buffer) : null;
}

/** Say once per `key` that a WASI call was refused or skipped, and why. */
function report(key, what) {
  if (said.has(key)) return;
  said.add(key);
  console.warn(`camcraft wasi: ${what}`);
}

/** A call made before `bind_memory`: nothing can be read or written yet. */
function unbound(name) {
  report(`unbound:${name}`, `${name} was called before the page bound the module's memory - answered ENOSYS`);
  return ERRNO_NOSYS;
}

/** The wall clock (id 0) and the monotonic clock (id 1), in nanoseconds.
 *  The CPU-time clocks have no page equivalent: EINVAL, said in the console. */
export function clock_time_get(id, _precision, result_ptr) {
  const v = view();
  if (!v) return unbound("clock_time_get");
  let ms;
  if (id === CLOCK_REALTIME) ms = Date.now();
  else if (id === CLOCK_MONOTONIC) ms = performance.now();
  else {
    report(`clock:${id}`, `clock_time_get: clock id ${id} does not exist in a browser - answered EINVAL`);
    return ERRNO_INVAL;
  }
  v.setBigUint64(result_ptr, BigInt(Math.round(ms * 1e6)), true);
  return ERRNO_SUCCESS;
}

/** stdout (1) and stderr (2) go to the console, one line at a time
 *  (`console.log` and `console.warn`). Any other descriptor: EBADF. */
export function fd_write(fd, iovs_ptr, iovs_len, nwritten_ptr) {
  const v = view();
  if (!v) return unbound("fd_write");
  if (fd !== 1 && fd !== 2) {
    report(`fd_write:${fd}`, `fd_write: descriptor ${fd} is not open (a tab has no files) - answered EBADF`);
    return ERRNO_BADF;
  }
  let written = 0;
  for (let i = 0; i < iovs_len; i++) {
    const ptr = v.getUint32(iovs_ptr + i * 8, true);
    const len = v.getUint32(iovs_ptr + i * 8 + 4, true);
    // Copy out of the module's memory before decoding.
    pending[fd] += decoder.decode(new Uint8Array(memory.buffer, ptr, len).slice());
    written += len;
  }
  let nl = pending[fd].indexOf("\n");
  while (nl >= 0) {
    const line = pending[fd].slice(0, nl);
    if (fd === 2) console.warn(line);
    else console.log(line);
    pending[fd] = pending[fd].slice(nl + 1);
    nl = pending[fd].indexOf("\n");
  }
  v.setUint32(nwritten_ptr, written, true);
  return ERRNO_SUCCESS;
}

/** Closing stdin/stdout/stderr is accepted (the libc does it at exit);
 *  nothing else is open. */
export function fd_close(fd) {
  if (fd > 2) {
    report(`fd_close:${fd}`, `fd_close: descriptor ${fd} is not open - answered EBADF`);
    return ERRNO_BADF;
  }
  return ERRNO_SUCCESS;
}

/** The three standard streams are not seekable: ESPIPE, as a terminal
 *  answers. The libc's stdio asks this of stdout and expects that answer,
 *  so it is not reported; any other descriptor is. */
export function fd_seek(fd, _offset, _whence, _result_ptr) {
  if (fd > 2) {
    report(`fd_seek:${fd}`, `fd_seek: descriptor ${fd} is not open - answered EBADF`);
    return ERRNO_BADF;
  }
  return ERRNO_SPIPE;
}

/** The three standard streams are character devices; nothing else is open. */
export function fd_fdstat_get(fd, stat_ptr) {
  const v = view();
  if (!v) return unbound("fd_fdstat_get");
  if (fd > 2) return ERRNO_BADF;
  for (let i = 0; i < 24; i += 4) v.setUint32(stat_ptr + i, 0, true);
  v.setUint8(stat_ptr, FILETYPE_CHARACTER_DEVICE);
  return ERRNO_SUCCESS;
}

/** The libc's start-up walks descriptors from 3 upward until this answers
 *  EBADF, looking for preopened directories. A tab has none, so EBADF is
 *  the expected, complete answer ("no file system") and is not reported. */
export function fd_prestat_get(_fd, _result_ptr) {
  return ERRNO_BADF;
}

/** Never reached while `fd_prestat_get` offers no directory; if it is,
 *  that is worth a line. */
export function fd_prestat_dir_name(fd, _path_ptr, _path_len) {
  report("fd_prestat_dir_name", `fd_prestat_dir_name: descriptor ${fd} is not a preopened directory - answered EBADF`);
  return ERRNO_BADF;
}

/** The libc's sleep (nanosleep, usleep - flecs' frame pacing) and poll.
 *  A tab cannot block its only thread, so the wait is skipped: the call
 *  succeeds at once and reports every clock subscription as elapsed, which
 *  the caller reads as "the sleep is over". No time has actually passed -
 *  the first skipped wait is said in the console with its length.
 *  Descriptor subscriptions (a poll on a file) are reported as events with
 *  ENOTSUP, also said in the console. */
export function poll_oneoff(in_ptr, out_ptr, nsubscriptions, nevents_ptr) {
  const v = view();
  if (!v) return unbound("poll_oneoff");
  if (nsubscriptions === 0) return ERRNO_INVAL;
  for (let i = 0; i < nsubscriptions; i++) {
    const sub = in_ptr + i * SUBSCRIPTION_SIZE;
    const ev = out_ptr + i * EVENT_SIZE;
    const userdata = v.getBigUint64(sub, true);
    const tag = v.getUint8(sub + 8);
    for (let o = 0; o < EVENT_SIZE; o += 4) v.setUint32(ev + o, 0, true);
    v.setBigUint64(ev, userdata, true);
    v.setUint8(ev + 10, tag);
    if (tag === EVENTTYPE_CLOCK) {
      const ms = Number(v.getBigUint64(sub + 24, true)) / 1e6;
      report("poll_oneoff:clock", `poll_oneoff: a blocking wait of ${ms} ms was skipped (a tab cannot block); later ones are skipped the same way`);
      v.setUint16(ev + 8, ERRNO_SUCCESS, true);
    } else {
      report("poll_oneoff:fd", "poll_oneoff: a wait on a file descriptor was refused with ENOTSUP (a tab has no files)");
      v.setUint16(ev + 8, ERRNO_NOTSUP, true);
    }
  }
  v.setUint32(nevents_ptr, nsubscriptions, true);
  return ERRNO_SUCCESS;
}

/** The C side asked to end the process (flecs' abort path). A tab has no
 *  process to end: stop the module with an error, which the page shows in
 *  its status line and the console (index.html's error handlers). */
export function proc_exit(code) {
  throw new Error(`CamCraft's engine stopped (exit code ${code}); reload the page.`);
}
