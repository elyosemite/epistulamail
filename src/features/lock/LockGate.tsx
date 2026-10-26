// US10 (CA2 e CA3): com o app bloqueado, só a tela de desbloqueio existe; o resto do app é desmontado, e nada do que
// estava aberto fica na tela. Desbloqueado e com a senha ligada, bloqueia depois do tempo sem uso escolhido e com
// Ctrl+Shift+L.
import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { idleLimitMs, isLockShortcut } from "../../lib/lock";
import { api, onBackendEvent, type LockMinutes, type LockStatus } from "../../lib/tauri";
import { UnlockScreen } from "./UnlockScreen";

interface AppLock {
  hasPassword: boolean;
  lockNow: () => void;
  /** As Configurações avisam quando o tempo sem uso muda. */
  setIdleMinutes: (minutes: LockMinutes) => void;
}

const LockContext = createContext<AppLock>({ hasPassword: false, lockNow: () => undefined, setIdleMinutes: () => undefined });

export const useAppLock = () => useContext(LockContext);

/** Reinicia a contagem no máximo a cada 5 s, para não refazer o timer a cada movimento do mouse. */
const ACTIVITY_THROTTLE_MS = 5_000;
const ACTIVITY_EVENTS = ["pointerdown", "pointermove", "keydown", "wheel"] as const;

export function LockGate({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<LockStatus | null>(null);
  const [idleMinutes, setIdleMinutes] = useState<LockMinutes | null>(null);

  useEffect(() => {
    api.lockStatus().then(setStatus).catch(() => setStatus({ locked: false, hasPassword: false, retryIn: 0 }));
    return onBackendEvent("lock-changed", setStatus);
  }, []);

  const unlocked = status !== null && !status.locked;
  const hasPassword = !!status?.hasPassword;

  useEffect(() => {
    if (!unlocked || !hasPassword) return setIdleMinutes(null);
    api.getPrivacySettings().then((s) => setIdleMinutes(s.lock.minutes)).catch(() => undefined);
  }, [unlocked, hasPassword]);

  const lockNow = useCallback(() => {
    void api.lockApp().catch(() => undefined);
  }, []);

  // Tempo sem uso.
  const last = useRef(0);
  useEffect(() => {
    const limit = idleMinutes ? idleLimitMs(idleMinutes) : null;
    if (!unlocked || !hasPassword || limit === null) return;
    let timer = window.setTimeout(lockNow, limit);
    const onActivity = () => {
      const now = Date.now();
      if (now - last.current < ACTIVITY_THROTTLE_MS) return;
      last.current = now;
      window.clearTimeout(timer);
      timer = window.setTimeout(lockNow, limit);
    };
    ACTIVITY_EVENTS.forEach((name) => window.addEventListener(name, onActivity, { passive: true }));
    return () => {
      window.clearTimeout(timer);
      ACTIVITY_EVENTS.forEach((name) => window.removeEventListener(name, onActivity));
    };
  }, [unlocked, hasPassword, idleMinutes, lockNow]);

  // Ctrl+Shift+L.
  useEffect(() => {
    if (!unlocked || !hasPassword) return;
    const onKey = (e: KeyboardEvent) => {
      if (!isLockShortcut(e)) return;
      e.preventDefault();
      lockNow();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [unlocked, hasPassword, lockNow]);

  if (status === null) return null;
  if (status.locked) return <UnlockScreen retryIn={status.retryIn} />;
  return <LockContext.Provider value={{ hasPassword, lockNow, setIdleMinutes }}>{children}</LockContext.Provider>;
}
