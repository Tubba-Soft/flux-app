import { invoke } from '@tauri-apps/api/core';

/**
 * Opens any external URL in the user's default web browser (Chrome, Edge, Firefox, etc.)
 * Uses native Windows Shell via Tauri IPC command `open_external_url`, with a safe browser fallback.
 */
export async function openExternalLink(url: string): Promise<void> {
  if (!url || (!url.startsWith('http://') && !url.startsWith('https://'))) {
    console.error('Invalid URL specified for openExternalLink:', url);
    return;
  }

  try {
    await invoke('open_external_url', { url });
  } catch (err) {
    console.warn('Could not launch browser via Tauri IPC, falling back to window.open:', err);
    try {
      window.open(url, '_blank', 'noopener,noreferrer');
    } catch (winErr) {
      console.error('Failed to open link in browser:', winErr);
    }
  }
}
