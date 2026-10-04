export function getOmpSessionSourceBadge(sourceKind) {
  if (sourceKind === 'omp-session') {
    return '<span class="badge source-badge" title="OMP default profile">Default</span>';
  }

  if (typeof sourceKind === 'string' && sourceKind.startsWith('omp-profile:')) {
    const profileName = sourceKind.slice('omp-profile:'.length);
    return `<span class="badge source-badge" title="OMP profile">${escapeHtml(profileName)}</span>`;
  }

  return getAdditionalSourceBadge(sourceKind, 'omp-source:', 'OMP');
}

export function getClaudeSessionSourceBadge(sourceKind) {
  return getAdditionalSourceBadge(sourceKind, 'claude-source:', 'Claude Code');
}

// `additional_sources` roots carry the hex-encoded absolute root in their
// source kind; show the last two path components and keep the full path as title.
function getAdditionalSourceBadge(sourceKind, prefix, toolLabel) {
  if (typeof sourceKind !== 'string' || !sourceKind.startsWith(prefix)) {
    return '';
  }
  const path = decodeHexPath(sourceKind.slice(prefix.length));
  if (!path) {
    return '';
  }
  const label = path.split('/').filter(Boolean).slice(-2).join('/') || path;
  return `<span class="badge source-badge" title="${escapeHtml(`${toolLabel} additional source: ${path}`)}">${escapeHtml(label)}</span>`;
}

function decodeHexPath(hex) {
  if (!hex || hex.length % 2 !== 0 || !/^[0-9a-f]+$/i.test(hex)) {
    return '';
  }
  const bytes = new Uint8Array(hex.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16);
  }
  return new TextDecoder().decode(bytes);
}

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}
