// DS: header.html (logo L76) · .appbar__brand com .wordmark.wordmark--sm

export function HeaderBrand() {
  return (
    <span className="appbar__brand">
      <span className="wordmark wordmark--sm">
        <img className="wordmark__logo" src="/epistulamail-logo.png" alt="" />
        EpistulaMail
      </span>
    </span>
  );
}
