const RADROOTS_MARKET_RELAY_URL = import.meta.env.VITE_PUBLIC_RADROOTS_MARKET_RELAY_URL;
const RADROOTS_MARKET_INDEXES_URL = import.meta.env.VITE_PUBLIC_RADROOTS_MARKET_INDEXES_URL;
const IDB_NAME = import.meta.env.VITE_PUBLIC_IDB_NAME;
const NDK_CACHE_NAME = import.meta.env.VITE_PUBLIC_NDK_CACHE_NAME;
const NDK_CLIENT_NAME = import.meta.env.VITE_PUBLIC_NDK_CLIENT_NAME;

// Only validate in browser context, not during build/analysis
if (typeof window !== 'undefined') {
	if (!RADROOTS_MARKET_RELAY_URL || typeof RADROOTS_MARKET_RELAY_URL !== 'string') throw new Error('Missing env var: VITE_PUBLIC_RADROOTS_MARKET_RELAY_URL');
	if (!RADROOTS_MARKET_INDEXES_URL || typeof RADROOTS_MARKET_INDEXES_URL !== 'string') throw new Error('Missing env var: VITE_PUBLIC_RADROOTS_MARKET_INDEXES_URL');
	if (!IDB_NAME || typeof IDB_NAME !== 'string') throw new Error('Missing env var: VITE_PUBLIC_IDB_NAME');
	if (!NDK_CACHE_NAME || typeof NDK_CACHE_NAME !== 'string') throw new Error('Missing env var: VITE_PUBLIC_NDK_CACHE_NAME');
	if (!NDK_CLIENT_NAME || typeof NDK_CLIENT_NAME !== 'string') throw new Error('Missing env var: VITE_PUBLIC_NDK_CLIENT_NAME');
}

export const _env = {
    IDB_NAME,
    NDK_CACHE_NAME,
    NDK_CLIENT_NAME,
    RADROOTS_MARKET_INDEXES_URL,
    RADROOTS_MARKET_RELAY_URL,
} as const;