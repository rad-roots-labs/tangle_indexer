const RADROOTS_MARKET_RELAY_URL = import.meta.env.VITE_PUBLIC_RADROOTS_MARKET_RELAY_URL;
const RADROOTS_MARKET_INDEXES_URL = import.meta.env.VITE_PUBLIC_RADROOTS_MARKET_INDEXES_URL;
const IDB_NAME = import.meta.env.VITE_PUBLIC_IDB_NAME;

// Only validate in browser context, not during build/analysis
if (typeof window !== 'undefined') {
	if (!RADROOTS_MARKET_RELAY_URL || typeof RADROOTS_MARKET_RELAY_URL !== 'string') throw new Error('Missing env var: VITE_PUBLIC_RADROOTS_MARKET_RELAY_URL');
	if (!RADROOTS_MARKET_INDEXES_URL || typeof RADROOTS_MARKET_INDEXES_URL !== 'string') throw new Error('Missing env var: VITE_PUBLIC_RADROOTS_MARKET_INDEXES_URL');
	if (!IDB_NAME || typeof IDB_NAME !== 'string') throw new Error('Missing env var: VITE_PUBLIC_IDB_NAME');
}

export const _env = {
    IDB_NAME,
    RADROOTS_MARKET_INDEXES_URL,
    RADROOTS_MARKET_RELAY_URL,
} as const;
