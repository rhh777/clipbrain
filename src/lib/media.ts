import { getAppIcon, getFilePreview, readImageBase64, readImageThumbnail, type FilePreview } from "./ipc";
import { LruCache } from "./lru-cache";

const MiB = 1024 * 1024;
const stringBytes = (value: string | null) => (value?.length ?? 0) * 2;
const thumbnails = new LruCache<string, string>(128, 4 * MiB, stringBytes);
const originals = new LruCache<string, string>(2, 16 * MiB, stringBytes);
const icons = new LruCache<string, string | null>(64, 2 * MiB, stringBytes);
const files = new LruCache<string, FilePreview>(32, 8 * MiB, (value) => JSON.stringify(value).length * 2);
const thumbnailRequests = new Map<string, Promise<string>>();
const originalRequests = new Map<string, Promise<string>>();
const iconRequests = new Map<string, Promise<string | null>>();
const fileRequests = new Map<string, Promise<FilePreview>>();

// Avoid decoding a whole screen of large originals at once in the backend.
let activeRequests = 0;
const waiting: Array<() => void> = [];
function schedule<T>(load: () => Promise<T>): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const run = () => {
      activeRequests += 1;
      Promise.resolve().then(load).then(resolve, reject).finally(() => {
        activeRequests -= 1;
        waiting.shift()?.();
      });
    };
    if (activeRequests < 2) run();
    else waiting.push(run);
  });
}

function loadCached<T>(
  cache: LruCache<string, T>, pending: Map<string, Promise<T>>,
  key: string, load: () => Promise<T>,
): Promise<T> {
  const cached = cache.get(key);
  if (cached !== undefined) return Promise.resolve(cached);
  const existing = pending.get(key);
  if (existing) return existing;
  const request = schedule(load).then((value) => {
    cache.set(key, value);
    return value;
  }).finally(() => pending.delete(key));
  pending.set(key, request);
  return request;
}

export const loadImageThumbnail = (path: string) =>
  loadCached(thumbnails, thumbnailRequests, path, () => readImageThumbnail(path));
export const loadImagePreview = (path: string) =>
  loadCached(originals, originalRequests, path, () => readImageBase64(path));
export const loadAppIcon = (name: string) =>
  loadCached(icons, iconRequests, name, () => getAppIcon(name).catch(() => null));
export const loadFilePreview = (path: string) =>
  loadCached(files, fileRequests, path, () => getFilePreview(path));
