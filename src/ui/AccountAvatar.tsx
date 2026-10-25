// DS: identidade.html · .avatar (--sm, --lg, cores da conta) com .avatar__img (foto da conta, US5)
import { useState } from "react";

type Size = "sm" | "md" | "lg";

interface AccountAvatarProps {
  initial: string;
  /** Classe de cor (`avatar--purple`) ou `null` para o azul padrão. */
  colorClass: string | null;
  /** `data:` URL da foto; sem ela (ou se falhar) fica a inicial. */
  photo: string | null;
  size?: Size;
}

export function AccountAvatar({ initial, colorClass, photo, size = "md" }: AccountAvatarProps) {
  const [failed, setFailed] = useState<string | null>(null);
  const cls = ["avatar", size !== "md" && `avatar--${size}`, colorClass].filter(Boolean).join(" ");
  return (
    <span className={cls} aria-hidden="true">
      {initial}
      {photo && failed !== photo && <img className="avatar__img" alt="" src={photo} onError={() => setFailed(photo)} />}
    </span>
  );
}
