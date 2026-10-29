// DS: avatar.html · .avatar, tamanhos --xs/--sm/--lg, cores --teal/--green/--purple/--magenta/--orange
import { avatarColor, initials } from "../lib/format";

type AvatarSize = "xs" | "sm" | "md" | "lg";

export function Avatar({ name, seed, size = "md" }: { name: string; seed?: string; size?: AvatarSize }) {
  const color = avatarColor(seed ?? name);
  const cls = ["avatar", size !== "md" && `avatar--${size}`, color && `avatar--${color}`].filter(Boolean).join(" ");
  return <span className={cls} aria-hidden="true">{initials(name)}</span>;
}
