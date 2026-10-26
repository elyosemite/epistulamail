// DS: identidade.html · .avatar (--sm, --lg, cores da conta) com .avatar__img (foto da conta, US5);
//     arquivos.html · .avatar--archive, o arquivo local com o ícone no lugar da inicial (L71, US15)
import { useState } from "react";
import { Icon } from "./Icon";

type Size = "sm" | "md" | "lg";

interface AccountAvatarProps {
  initial: string;
  /** Classe de cor (`avatar--purple`) ou `null` para o azul padrão. */
  colorClass: string | null;
  /** `data:` URL da foto; sem ela (ou se falhar) fica a inicial. */
  photo: string | null;
  size?: Size;
  /** Arquivo de e-mail importado (US15): neutro, com o ícone. */
  archive?: boolean;
}

export function AccountAvatar({ initial, colorClass, photo, size = "md", archive }: AccountAvatarProps) {
  const [failed, setFailed] = useState<string | null>(null);
  if (archive) {
    return (
      <span className={["avatar", size !== "md" && `avatar--${size}`, "avatar--archive"].filter(Boolean).join(" ")} aria-hidden="true">
        <Icon name="archive" />
      </span>
    );
  }
  const cls = ["avatar", size !== "md" && `avatar--${size}`, colorClass].filter(Boolean).join(" ");
  return (
    <span className={cls} aria-hidden="true">
      {initial}
      {photo && failed !== photo && <img className="avatar__img" alt="" src={photo} onError={() => setFailed(photo)} />}
    </span>
  );
}
