type ToastFn = (msg: string) => void

let globalToast: ToastFn = () => {}

export function setToastHandler(fn: ToastFn) {
  globalToast = fn
}

export async function apiFetch<T>(url: string, options?: RequestInit): Promise<T> {
  const res = await fetch(url, options)
  if (!res.ok) {
    let msg = `HTTP ${res.status}`
    try {
      const body = await res.json()
      if (body.error) msg = body.error
      else if (body.message) msg = body.message
    } catch {}
    globalToast(msg)
    throw new Error(msg)
  }
  return res.json()
}

export async function apiPost<T>(url: string, body?: unknown): Promise<T> {
  return apiFetch<T>(url, {
    method: 'POST',
    headers: body ? { 'Content-Type': 'application/json' } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  })
}
