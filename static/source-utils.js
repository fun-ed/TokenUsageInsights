export function getOmpSessionSourceBadge(sourceKind) {
  if (sourceKind === 'omp-session') {
    return '<span class="badge source-badge" title="OMP default profile">Default</span>';
  }

  if (typeof sourceKind === 'string' && sourceKind.startsWith('omp-profile:')) {
    const profileName = sourceKind.slice('omp-profile:'.length);
    return `<span class="badge source-badge" title="OMP profile">${escapeHtml(profileName)}</span>`;
  }

  return '';
}

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}
