import { i18n_conf } from '@radroots/apps-lib';
import { iso639_1, locales_default, type Locales } from '@radroots/locales';

export type Locale = Locales;

const translations: Record<Locale, typeof iso639_1> = locales_default.reduce((acc, locale) => ({ ...acc, [locale as Locale]: iso639_1, }), {} as Record<Locale, typeof iso639_1>);
const locale_keys = Object.keys(translations);

const i18n = i18n_conf<Locale>({
    default_locale: `en`,
    translations,
    loaders: [
        ...locale_keys.map((locale) => [`common`, `countries`, `error`, `eula`, `icu`, `market`, `notification`, `units`].map(key => ({
            locale,
            key,
            loader: async () => (await import(`../../../../../packages/locales/src/messages/${locale}/${key}.json`)).default
        }))),
    ].flat()
});

const {
    t: ls,
    loading: translations_loading,
    locales,
    locale,
    loadTranslations: load_translations
} = i18n;

translations_loading.subscribe(async (_loading) => {
    if (_loading) await translations_loading.toPromise();
});

export { load_translations, locale, locales, ls, translations, translations_loading };
