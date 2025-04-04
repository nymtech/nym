import { open } from '@tauri-apps/plugin-shell'

export const openInBrowser = async (url: string): Promise<void> => {
  console.log(`Attempting to open in browser: ${url}`)

  try {
    await open(url)
    console.log('Browser opened successfully')
  } catch (error) {
    console.error('Failed to open browser with shell plugin:', error)

    try {
      window.open(url, '_blank')
    } catch (e) {
      console.error('Fallback also failed:', e)
    }
  }
}
