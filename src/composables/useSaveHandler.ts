import { onMounted, onUnmounted } from 'vue'
import { useAppShellStore, type SaveHandler } from '@/stores/appShell'

/**
 * Composable for components that have save functionality.
 * Registers the save handler when the component mounts and
 * unregisters when it unmounts.
 *
 * @param handler - The save function to call on Ctrl+S
 *
 * @example
 * ```ts
 * const { register, unregister } = useSaveHandler(async () => {
 *   await saveMyData()
 * })
 * ```
 */
export function useSaveHandler(handler: SaveHandler) {
  const appShell = useAppShellStore()
  let unregister: (() => void) | null = null

  function register() {
    unregister = appShell.registerSaveHandler(handler)
  }

  function unregisterHandler() {
    if (unregister) {
      unregister()
      unregister = null
    }
  }

  onMounted(register)
  onUnmounted(unregisterHandler)

  return {
    register,
    unregister: unregisterHandler,
  }
}
