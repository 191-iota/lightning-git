// Node 26 ships an experimental native `localStorage` that is unavailable
// unless the process is started with --localstorage-file, and it shadows the
// happy-dom implementation, so any test that reads or writes storage gets a
// broken global. Install a small deterministic in-memory Storage instead. This
// also makes storage behaviour identical across Node versions in CI.
class MemoryStorage {
  private store = new Map<string, string>();
  get length() {
    return this.store.size;
  }
  clear() {
    this.store.clear();
  }
  getItem(key: string) {
    return this.store.has(key) ? this.store.get(key)! : null;
  }
  key(index: number) {
    return Array.from(this.store.keys())[index] ?? null;
  }
  removeItem(key: string) {
    this.store.delete(key);
  }
  setItem(key: string, value: string) {
    this.store.set(key, String(value));
  }
}

Object.defineProperty(globalThis, "localStorage", {
  configurable: true,
  value: new MemoryStorage(),
});
