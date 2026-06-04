import { ref } from 'vue'

const nlPort = (window as any).NL_PORT as string | undefined

export function buildThumbUrl(filename: string): string {
  if (!filename) return ''
  const name = filename.split(/[\\/]/).pop() || filename
  if (nlPort) {
    return `http://localhost:${nlPort}/data/thumbnails/${name}`
  }
  return `/data/thumbnails/${name}`
}

export function useThumbnail() {
  const cache = ref<Record<string, string>>({})

  function getSrc(filePath: string, thumbnailPath: string): string {
    if (cache.value[filePath]) return cache.value[filePath]
    if (thumbnailPath) {
      cache.value[filePath] = buildThumbUrl(thumbnailPath)
      return cache.value[filePath]
    }
    return ''
  }

  function clearCache() {
    cache.value = {}
  }

  return {
    cache,
    getSrc,
    clearCache,
  }
}
