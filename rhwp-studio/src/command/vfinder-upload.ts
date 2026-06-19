/**
 * vfinder `/api/upload` 직호출 helper.
 *
 * agent VM 안에서 rhwp 와 vfinder 가 같은 host (path proxy `/vfinder/`) 로 같이
 * 떠 있으므로 *iframe 의 origin = rhwp origin = vfinder origin* 이라 same-origin
 * fetch 가 그대로 통과. agent host 의 SSR 파라미터 부착에 의존하지 않고 *VM 내부
 * 자족 저장* 경로를 연다.
 *
 * vfinder upload 3 모드 ([vfinder/vfinder-server/src/api/upload.rs]):
 *   - `?file_id=<id>`        : 그 id 의 파일을 *그 자리* 덮어쓰기 (저장)
 *   - `?path=&overwrite=true`: 부모 폴더 + 동일 이름 덮어쓰기
 *   - `?path=`               : 부모 폴더 + 신규 (이름 충돌 시 suffix)
 *
 * 인증: `X-Vfinder-User` 헤더 + 쿼리 `?user=` 양쪽 모두 박는다 (서버측 어느 쪽이든
 * 우선 인식하게 해 둠).
 */

export interface VfinderUploadOptions {
  blob: Blob;
  /** 저장될 파일 이름 (확장자 포함). multipart `file.filename` 으로 전달. */
  fileName: string;
  /** vfinder studio base URL. 기본 `/vfinder` (같은 host). */
  vfinderBase?: string;
  /** 사용자 식별자 (예: 이메일). 헤더·쿼리 양쪽에 박힘. */
  userId?: string;

  // ── 모드 분기 — 셋 중 정확히 하나 ─────────────────────────────
  /** *file_id* 모드 — 기존 파일 그 자리 덮어쓰기 (Ctrl+S 재저장). */
  fileId?: string;
  /** *overwrite/신규* 모드 공통 — 부모 폴더 경로 (root 기준). */
  path?: string;
  /** *overwrite* 모드 — `path` + `overwrite=true`. 동일 이름 자리 덮어쓰기. */
  overwrite?: boolean;
}

export interface VfinderUploadResult {
  /** file_object 테이블의 file_id. DB 비활성/실패 시 null. */
  fileId: string | null;
  /** 실제 저장된 leaf name (신규 모드에서 suffix 가능). */
  name: string;
  /** root 기준 실 저장 경로. */
  path: string;
  size: number;
  /** 덮어쓰기 여부. file_id·overwrite 모드는 true, 신규는 false. */
  updated: boolean;
}

/** vfinder 서버 응답의 wire schema (snake_case). */
interface VfinderUploadResponseWire {
  file_id: string | null;
  name: string;
  path: string;
  size: number;
  updated: boolean;
}

export async function uploadToVfinder(opts: VfinderUploadOptions): Promise<VfinderUploadResult> {
  const base = (opts.vfinderBase ?? '/vfinder').replace(/\/$/, '');

  const params = new URLSearchParams();
  if (opts.fileId) {
    params.set('file_id', opts.fileId);
  } else if (opts.path !== undefined) {
    params.set('path', opts.path);
    if (opts.overwrite) params.set('overwrite', 'true');
  } else {
    throw new Error('uploadToVfinder: fileId 또는 path 중 하나가 필요합니다.');
  }
  if (opts.userId) params.set('user', opts.userId);

  const formData = new FormData();
  // multipart `file` field — vfinder 서버가 인식하는 유일 field 이름.
  formData.append('file', opts.blob, opts.fileName);

  const headers: Record<string, string> = {};
  if (opts.userId) headers['X-Vfinder-User'] = opts.userId;

  const res = await fetch(`${base}/api/upload?${params.toString()}`, {
    method: 'POST',
    headers,
    body: formData,
    // 인증 cookie 경유 시나리오 호환.
    credentials: 'same-origin',
  });

  if (!res.ok) {
    const body = await res.text().catch(() => '');
    throw new Error(`vfinder upload HTTP ${res.status}: ${body || res.statusText}`);
  }

  const wire = (await res.json()) as VfinderUploadResponseWire;
  return {
    fileId: wire.file_id,
    name: wire.name,
    path: wire.path,
    size: wire.size,
    updated: wire.updated,
  };
}
