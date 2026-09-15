export function createQueue() {
  let chain: Promise<unknown> = Promise.resolve();
  return function enqueue<T>(fn: () => Promise<T>): Promise<T> {
    const next = chain.then(fn, fn) as Promise<T>;
    chain = next;
    return next;
  };
}
