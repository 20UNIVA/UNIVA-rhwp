/**
 * 서버 (rhwp-server) 에러 응답 → i18n 키 매핑.
 *
 * 서버는 `{ "error": "한국어 메시지", "code": "stable_code" }` 박는다.
 * 클라이언트는 `code` 기반 매핑으로 갈음, ko 사용자에겐 서버 메시지를
 * 그대로 보여줘 *정보 손실 0*, en·ja 사용자에겐 사전 매핑된 메시지.
 *
 * vfinder playbook §5 정합.
 */

import { t, getLang, type MessageKey } from './t';

/** 서버 에러 응답 모양. */
export interface ServerErrorBody {
  error?: string;
  code?: string;
  [k: string]: unknown;
}

/** stable code → i18n 키 매핑. */
const CODE_TO_KEY: Record<string, MessageKey> = {
  // 핵심 에러
  doc_parse_failed: 'error.server.bad_request',
  doc_corrupted: 'error.server.bad_request',
  session_not_found: 'error.server.not_found',
  session_or_storage_missing: 'error.server.not_found',
  snapshot_parse_failed: 'error.server.bad_request',
  user_id_missing: 'error.server.forbidden',
  empty_doc_create_failed: 'error.server.internal',
  save_failed: 'error.server.internal',
  storage_upload_failed: 'error.server.internal',
  input_read_failed: 'error.server.bad_request',
  sqlite_error: 'error.server.internal',
  // generic
  bad_request: 'error.server.bad_request',
  unprocessable: 'error.server.bad_request',
  not_found: 'error.server.not_found',
  forbidden: 'error.server.forbidden',
  conflict: 'error.server.bad_request',
  internal: 'error.server.internal',
};

/**
 * 서버 에러를 현재 lang 에 맞춰 i18n 한다.
 *
 * 우선 순위:
 *   1. ko 사용자 → 서버 한국어 메시지 그대로 (정보 손실 0)
 *   2. code 매핑 박힌 자료 → 사전 키 박힘 (en·ja)
 *   3. fallback → 서버 메시지 (자료가 영어 매핑 박히지 않은 자체)
 */
export function localizeServerError(body: ServerErrorBody | undefined): string {
  const lang = getLang();
  const serverMsg = body?.error ?? '';
  const code = body?.code;

  // ko 자체 자체 자체 서버 메시지 그대로
  if (lang === 'ko') {
    return serverMsg || t('error.server.internal');
  }

  // en·ja 자체 자체 자체 code 매핑 우선
  if (code && CODE_TO_KEY[code]) {
    return t(CODE_TO_KEY[code]);
  }

  // fallback — 서버 메시지 또는 generic
  return serverMsg || t('error.server.internal');
}

/**
 * fetch Response 자체 자체 자체 *에러 자체 자체* → 매핑된 메시지 박는다.
 */
export async function localizeFetchError(res: Response): Promise<string> {
  try {
    const body = (await res.json()) as ServerErrorBody;
    return localizeServerError(body);
  } catch {
    return t('error.server.internal');
  }
}
