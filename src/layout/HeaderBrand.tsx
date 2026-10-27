// DS: header.html · .appbar__brand com .wordmark.wordmark--sm
import { Icon } from "../ui/Icon";

export function HeaderBrand() {
  return (
    <span className="appbar__brand">
      <span className="wordmark wordmark--sm">
        <span className="wordmark__glyph" aria-hidden="true"><Icon name="mail" /></span>
        EpistulaMail
      </span>
    </span>
  );
}
