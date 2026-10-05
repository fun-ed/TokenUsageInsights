import { parseUsageTimestamp } from './time-utils.js?v=1';

function getSessionSortValue(session, sortColumn) {
  const value = session?.[sortColumn];
  if (sortColumn === 'timestamp') {
    return parseUsageTimestamp(value)?.getTime() ?? 0;
  }
  return value ?? 0;
}

export function formatSessionModelDisplay(model, reasoningEffort, assistantType) {
  const modelLabel = model ?? '';
  if (assistantType !== 'omp' || !modelLabel || !reasoningEffort) return modelLabel;

  const suffix = `:${reasoningEffort}`;
  return modelLabel.endsWith(suffix) ? modelLabel : `${modelLabel}${suffix}`;
}

export function compareSessionRows(a, b, sortColumn, sortDirection) {
  const valueA = getSessionSortValue(a, sortColumn);
  const valueB = getSessionSortValue(b, sortColumn);
  let comparison;

  if (typeof valueA === 'string' && typeof valueB === 'string') {
    comparison = valueA.localeCompare(valueB);
  } else {
    comparison = valueA - valueB;
  }

  return sortDirection === 'asc' ? comparison : -comparison;
}

export function matchesSessionIdentity(session, identity) {
  return sessionIdentityKey(session) === sessionIdentityKey(identity);
}

export function sessionIdentityKey(session) {
  return JSON.stringify([
    session?.assistant_type || '',
    session?.source_kind || '',
    session?.source_dir_key || '',
    session?.session_id || '',
  ]);
}

export function parentSessionIdentityKey(session) {
  if (!session?.parent_session_id) return null;
  return sessionIdentityKey({
    ...session,
    session_id: session.parent_session_id,
  });
}

export function filterEntriesBySessionIdentity(entries, sessions) {
  const sessionKeys = new Set((sessions || []).map(sessionIdentityKey));
  return (entries || []).filter(entry => sessionKeys.has(sessionIdentityKey(entry)));
}

export const SESSION_PRICING_MODELS = Object.freeze([
  'glm-5.3',
  'deepseek-v4.1-flash',
  'glm-5.3-flash',
]);

export function buildSessionPricingPayload(session, pricingModel) {
  if (
    typeof session?.assistant_type !== 'string'
    || !session.assistant_type
    || typeof session.source_kind !== 'string'
    || !session.source_kind
    || typeof session.session_id !== 'string'
    || !session.session_id
    || (
      typeof session.source_dir_key !== 'string'
      && session.source_dir_key !== null
      && session.source_dir_key !== undefined
    )
  ) {
    throw new TypeError('Session pricing requires a complete session identity');
  }

  const normalizedModel = pricingModel === '' ? null : pricingModel;
  if (normalizedModel !== null && !SESSION_PRICING_MODELS.includes(normalizedModel)) {
    throw new TypeError('Unsupported session pricing model');
  }

  return {
    session_id: session.session_id,
    source_kind: session.source_kind,
    source_dir_key: session.source_dir_key ?? null,
    pricing_model: normalizedModel,
  };
}
