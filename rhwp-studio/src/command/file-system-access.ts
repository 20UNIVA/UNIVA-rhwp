export interface FileSystemWritableFileStreamLike {
  write(data: Blob): Promise<void>;
  close(): Promise<void>;
}

export interface FileSystemFileHandleLike {
  kind?: 'file';
  name: string;
  getFile(): Promise<File>;
  createWritable(): Promise<FileSystemWritableFileStreamLike>;
}

export interface FileSystemWindowLike {
  showOpenFilePicker?: (options?: {
    excludeAcceptAllOption?: boolean;
    multiple?: boolean;
    types?: { description: string; accept: Record<string, string[]> }[];
  }) => Promise<FileSystemFileHandleLike[]>;
  showSaveFilePicker?: (options?: {
    suggestedName?: string;
    types?: { description: string; accept: Record<string, string[]> }[];
  }) => Promise<FileSystemFileHandleLike>;
}

export interface FileHandleReadResult {
  name: string;
  bytes: Uint8Array;
}

export interface SaveDocumentOptions {
  blob: Blob;
  suggestedName: string;
  currentHandle: FileSystemFileHandleLike | null;
  windowLike: FileSystemWindowLike;
  /** [Task #833] true 시 currentHandle 무시 + 항상 showSaveFilePicker 호출 (다른 이름으로 저장). */
  forceSaveAs?: boolean;
}

export interface SaveDocumentResult {
  method: 'current-handle' | 'save-picker' | 'fallback';
  handle: FileSystemFileHandleLike | null;
  fileName: string;
}

const HWP_OPEN_PICKER_TYPES = [{
  description: 'HWP/HWPX 문서',
  accept: { 'application/x-hwp': ['.hwp', '.hwpx'] },
}];

const HWP_SAVE_PICKER_TYPES = [{
  description: 'HWP 문서',
  accept: { 'application/x-hwp': ['.hwp'] },
}];

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === 'AbortError';
}

/**
 * 현재 창이 *다른 origin* 의 부모창 안 iframe 으로 떠 있는지 감지.
 *
 * 브라우저 보안 정책상 `showSaveFilePicker`·`showOpenFilePicker` 같은 File
 * System Access API 는 *cross-origin iframe* 에서는 *항상* SecurityError
 * ("Cross origin sub frames aren't allowed to show a file picker") 로 거부된다.
 * 즉 그 환경에선 picker 호출 자체가 *실패가 예정된 호출*이므로 부르지 않고
 * 곧장 fallback (anchor download) 으로 가는 게 깔끔하다.
 *
 * same-origin iframe 또는 top window 환경에선 false 를 반환해 기존 picker 흐름
 * 그대로 유지.
 *
 * 본 헬퍼는 *agent iframe + SSR 비활성* 상황 차단에도 쓰인다 — file.ts 가
 * import 해 다운로드 폴백 우회 판정에 사용. SSR 비활성인 cross-origin iframe
 * 환경은 *환경 설정 결함* (URL 에 ?fileId·?ssr 부착 누락) 이므로 다운로드로
 * 흘려보내지 않고 명확히 안내해야 한다.
 */
export function isCrossOriginEmbedded(windowLike: FileSystemWindowLike): boolean {
  try {
    // 본 창이 부모 창이라면 iframe 자체가 아니므로 cross-origin 일 수 없음.
    const win = windowLike as unknown as Window;
    if (win.self === win.top) return false;
    // parent.location.href 접근 시 cross-origin 이면 SecurityError 가 throw 된다.
    // 접근 성공 = same-origin iframe.
    void win.parent.location.href;
    return false;
  } catch {
    return true;
  }
}

async function writeBlobToHandle(handle: FileSystemFileHandleLike, blob: Blob): Promise<void> {
  const writable = await handle.createWritable();
  await writable.write(blob);
  await writable.close();
}

export async function pickOpenFileHandle(windowLike: FileSystemWindowLike): Promise<FileSystemFileHandleLike | null> {
  if (!windowLike.showOpenFilePicker) return null;
  // cross-origin iframe 안에서는 picker 자체가 보안 정책 거부.
  if (isCrossOriginEmbedded(windowLike)) return null;

  try {
    const handles = await windowLike.showOpenFilePicker({
      excludeAcceptAllOption: true,
      multiple: false,
      types: HWP_OPEN_PICKER_TYPES,
    });
    return handles[0] ?? null;
  } catch (error) {
    if (isAbortError(error)) return null;
    throw error;
  }
}

export async function readFileFromHandle(handle: FileSystemFileHandleLike): Promise<FileHandleReadResult> {
  const file = await handle.getFile();
  return {
    name: file.name,
    bytes: new Uint8Array(await file.arrayBuffer()),
  };
}

export async function saveDocumentToFileSystem(options: SaveDocumentOptions): Promise<SaveDocumentResult> {
  const { blob, suggestedName, currentHandle, windowLike, forceSaveAs } = options;

  // [Task #833] forceSaveAs 시 currentHandle 우회 → 항상 picker (다른 이름으로 저장).
  if (currentHandle && !forceSaveAs) {
    await writeBlobToHandle(currentHandle, blob);
    return {
      method: 'current-handle',
      handle: currentHandle,
      fileName: currentHandle.name,
    };
  }

  // cross-origin iframe 안에서는 *예정된 SecurityError* 우회 — picker 시도 자체
  // 를 안 하고 곧장 fallback download 로 흐른다. 시도하면 console 에 보안 거부
  // 경고가 그대로 노출되어 *진짜 결함* 인지 *환경 제약* 인지 구분이 안 된다.
  if (windowLike.showSaveFilePicker && !isCrossOriginEmbedded(windowLike)) {
    const handle = await windowLike.showSaveFilePicker({
      suggestedName,
      types: HWP_SAVE_PICKER_TYPES,
    });
    await writeBlobToHandle(handle, blob);
    return {
      method: 'save-picker',
      handle,
      fileName: handle.name,
    };
  }

  return {
    method: 'fallback',
    handle: null,
    fileName: suggestedName,
  };
}
