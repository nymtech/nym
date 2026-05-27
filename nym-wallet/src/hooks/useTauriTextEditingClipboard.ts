import { useEffect } from 'react';
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager';

function isTextEditingElement(el: EventTarget | null): el is HTMLInputElement | HTMLTextAreaElement {
  if (!el || !(el instanceof HTMLElement)) return false;
  if (el.closest('[data-nym-currency-field]')) return false;
  /** Mnemonic login uses replace-whole-value paste + trim (must not insert at caret). */
  if (el.closest('[data-nym-auth-paste-field]')) return false;
  /** Send/delegate/bond fields pass clipboard to React as a full replacement, not insertion. */
  if (el.closest('[data-nym-paste-replace]')) return false;
  if (el instanceof HTMLTextAreaElement) return !el.disabled && !el.readOnly;
  if (el instanceof HTMLInputElement) {
    if (el.disabled || el.readOnly) return false;
    const blocked = ['button', 'checkbox', 'radio', 'submit', 'file', 'reset', 'image'];
    return !blocked.includes(el.type);
  }
  return false;
}

/** Input types where copying the whole value on Cmd+C (no selection) matches typical desktop behavior. */
function isSingleLineTextLikeInput(el: HTMLInputElement): boolean {
  const t = el.type;
  return t === 'text' || t === 'search' || t === 'tel' || t === 'url' || t === 'email' || t === 'password' || t === '';
}

function setNativeValue(el: HTMLInputElement | HTMLTextAreaElement, value: string) {
  const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
  const setter = Object.getOwnPropertyDescriptor(proto, 'value')?.set;
  setter?.call(el, value);
  el.dispatchEvent(new Event('input', { bubbles: true }));
  el.dispatchEvent(new Event('change', { bubbles: true }));
}

/**
 * WebKit/WebView in Tauri often breaks native copy/paste in text fields. Route clipboard through the
 * Tauri clipboard plugin when the user triggers standard shortcuts while focused in an input.
 */
export function useTauriTextEditingClipboard() {
  useEffect(() => {
    const onKeyDown = async (e: KeyboardEvent) => {
      if (!e.ctrlKey && !e.metaKey) return;
      if (e.defaultPrevented) return;
      if (!isTextEditingElement(e.target)) return;

      const el = e.target;

      if (e.key === 'v') {
        e.preventDefault();
        try {
          const text = await readText();
          if (text == null) return;
          const start = el.selectionStart ?? 0;
          const end = el.selectionEnd ?? 0;
          const next = `${el.value.slice(0, start)}${text}${el.value.slice(end)}`;
          setNativeValue(el, next);
          const caret = start + text.length;
          el.setSelectionRange(caret, caret);
        } catch {
          /* fall through - user can retry from context menu */
        }
        return;
      }

      if (e.key === 'c' || e.key === 'x') {
        const start = el.selectionStart ?? 0;
        const end = el.selectionEnd ?? 0;
        let selected = el.value.slice(start, end);
        if (!selected && e.key === 'c' && el instanceof HTMLInputElement && isSingleLineTextLikeInput(el)) {
          selected = el.value;
        }
        if (!selected) return;
        e.preventDefault();
        try {
          await writeText(selected);
        } catch {
          return;
        }
        if (e.key === 'x') {
          if (start !== end) {
            const next = `${el.value.slice(0, start)}${el.value.slice(end)}`;
            setNativeValue(el, next);
            el.setSelectionRange(start, start);
          }
        }
      }
    };

    window.addEventListener('keydown', onKeyDown, true);
    return () => window.removeEventListener('keydown', onKeyDown, true);
  }, []);
}
