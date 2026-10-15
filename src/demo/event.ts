// Remplace @tauri-apps/api/event dans le mode « demo » (captures d'écran).
type Handler = (e: { payload: unknown }) => void;
const handlers = new Map<string, Set<Handler>>();

export function listen<T>(name: string, fn: (e: { payload: T }) => void): Promise<() => void> {
  const set = handlers.get(name) ?? new Set<Handler>();
  set.add(fn as Handler);
  handlers.set(name, set);
  return Promise.resolve(() => set.delete(fn as Handler));
}

export function emit(name: string, payload: unknown) {
  handlers.get(name)?.forEach((fn) => fn({ payload }));
}
