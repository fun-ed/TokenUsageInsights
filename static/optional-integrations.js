export const OPTIONAL_INTEGRATIONS_ENABLED_STORAGE_KEY = 'optional_integrations_enabled';

export function isOptionalIntegrationsEnabled(storage) {
  return storage?.getItem(OPTIONAL_INTEGRATIONS_ENABLED_STORAGE_KEY) === 'true';
}

export function shouldFetchOptionalIntegrationStatus(enabled, activeTab) {
  return enabled === true && activeTab === 'integrations';
}

export function sourceStateLabel(state) {
  return {
    found: '已驗證',
    missing: '找不到',
    invalid: '格式無效',
    unverified: '尚未驗證',
  }[state] || '未知';
}
