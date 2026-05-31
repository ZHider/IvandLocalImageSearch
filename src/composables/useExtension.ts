import { extensions, events } from '@neutralinojs/lib'

const EXTENSION_ID = 'ai.search.extension'

export function useExtension() {
  function send(event: string, data: Record<string, unknown> = {}) {
    extensions.dispatch(EXTENSION_ID, event, data)
  }

  function on(event: string, callback: (data: unknown) => void) {
    events.on(event, callback)
  }

  return {
    send,
    on,
  }
}