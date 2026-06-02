import { extensions, events } from '@neutralinojs/lib'

const EXTENSION_ID = 'ai.search.extension'

// 保存 wrapper 引用以供 off 清理
const listenerMap = new Map<string, Map<Function, (e: Event) => void>>()

export function useExtension() {
  function send(event: string, data: Record<string, unknown> = {}) {
    extensions.dispatch(EXTENSION_ID, event, data)
  }

  function on(event: string, callback: (data: unknown) => void): () => void {
    const wrapper = (e: Event) => callback((e as CustomEvent).detail)
    let handlers = listenerMap.get(event)
    if (!handlers) {
      handlers = new Map()
      listenerMap.set(event, handlers)
    }
    handlers.set(callback, wrapper)
    events.on(event, wrapper)
    return () => {
      const h = listenerMap.get(event)
      if (h) {
        const w = h.get(callback)
        if (w) {
          events.off(event, w)
          h.delete(callback)
        }
      }
    }
  }

  return {
    send,
    on,
  }
}
