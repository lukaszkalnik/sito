import { useTranslation } from 'react-i18next';

import { useFormats } from '../../i18n/formats';
import { cacheSizeText, type CacheSizeText } from './cacheSize';

const UNIT_KEY: Readonly<Record<CacheSizeText['unit'], string>> = {
  mb: 'cache.sizeMb',
  kb: 'cache.sizeKb',
};

/** `bytes` with its unit in the current language, e.g. „412 MB”. */
export function useSizeText(): (bytes: number) => string {
  const { t } = useTranslation();
  const formats = useFormats();
  return (bytes) => {
    const size = cacheSizeText(bytes, formats);
    return t(UNIT_KEY[size.unit], { size: size.value });
  };
}
