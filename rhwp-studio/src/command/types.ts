import type { EventBus } from '@/core/event-bus';
import type { WasmBridge } from '@/core/wasm-bridge';
import type { DocumentDirtyState } from '@/core/document-dirty-state';
import type { InputHandler } from '@/engine/input-handler';
import type { ViewportManager } from '@/view/viewport-manager';

/** 커맨드 실행 가능 여부 판단용 에디터 상태 스냅샷 */
export interface EditorContext {
  /** 문서가 로드되어 있는가? */
  hasDocument: boolean;
  /** 선택 영역이 있는가? */
  hasSelection: boolean;
  /** 커서가 표 셀 내부인가? */
  inTable: boolean;
  /** F5 셀 선택 모드인가? */
  inCellSelectionMode: boolean;
  /** 표 객체 선택 모드인가? */
  inTableObjectSelection: boolean;
  /** 그림 객체 선택 모드인가? */
  inPictureObjectSelection: boolean;
  /** 커서가 누름틀 필드 내부인가? */
  inField: boolean;
  /** 편집 가능 모드인가? (vs 읽기 전용) */
  isEditable: boolean;
  /** Undo 가능한가? */
  canUndo: boolean;
  /** Redo 가능한가? */
  canRedo: boolean;
  /** 현재 줌 레벨 (0.5 ~ 4.0) */
  zoom: number;
  /** 조판부호 보이기 모드인가? */
  showControlCodes: boolean;
  /** 저장되지 않은 문서 변경사항이 있는가? */
  isDirty: boolean;
  /** 원본 파일 형식 (#888 — HWPX 출처는 HWP 변환 저장) */
  sourceFormat?: 'hwp' | 'hwpx';
}

/** 개별 커맨드 정의 */
export interface CommandDef {
  /** 네임스페이스 ID: "카테고리:액션" (예: "edit:copy") */
  readonly id: string;
  /** 표시 레이블 (한국어) */
  readonly label: string;
  /** 단축키 표시 문자열 (예: "Ctrl+C"). 표시 전용 */
  readonly shortcutLabel?: string;
  /** 아이콘 CSS 클래스명 (기존 icon-* 클래스) */
  readonly icon?: string;
  /**
   * 현재 컨텍스트에서 실행 가능한지 판단.
   * 생략 시 항상 활성.
   */
  canExecute?: (ctx: EditorContext) => boolean;
  /** 커맨드 실행 */
  execute: (services: CommandServices, params?: Record<string, unknown>) => void;
}

/**
 * vfinder save-as picker 결과 — `{path, name, overwrite}` 한 묶음. server forward
 * 와 client direct upload 양쪽이 *같은 target* 으로 갈 수 있게 picker 책임을 분리한 자리.
 */
export interface SaveAsTarget {
  /** 부모 폴더 경로 (root 기준). */
  path: string;
  /** 파일명 (확장자 포함). */
  name: string;
  /** 동일 이름 자리 덮어쓰기 선택 여부. */
  overwrite: boolean;
}

/** 커맨드 execute()에 주입되는 서비스 */
export interface CommandServices {
  eventBus: EventBus;
  wasm: WasmBridge;
  /** 저장되지 않은 문서 변경 상태 */
  documentState: DocumentDirtyState;
  /** 현재 에디터 상태 스냅샷 */
  getContext: () => EditorContext;
  /** InputHandler 접근 (문서 미로드 시 null) */
  getInputHandler: () => InputHandler | null;
  /** ViewportManager 접근 (문서 미로드 시 null) */
  getViewportManager: () => ViewportManager | null;
  /**
   * SSR 세션 저장(서버 외부 저장소 덮어쓰기). 설정 시 file:save 가 로컬 저장 대신 이것을 우선 시도한다.
   * 반환 true = 서버 저장 성공. 미설정/false 면 기존 로컬 저장으로 진행.
   */
  saveToServer?: () => Promise<boolean>;
  /**
   * vfinder iframe 흐름으로 *다른 이름으로 저장* (호환 유지 — file.ts 는 이제
   * `pickVfinderSaveAsTarget` + `forwardSaveAsToServer` 두 함수를 *직접* 조합해서
   * picker 가 한 번만 뜨도록 흐름을 통합한다. 본 메서드는 외부 e2e/툴 호환 위해 유지).
   *
   * 반환 true = 성공. false = 취소·실패.
   */
  saveAsViaVfinder?: () => Promise<boolean>;
  /**
   * vfinder save-as picker iframe 만 띄워 *target* 만 반환. server forward 또는 client
   * direct vfinder upload 어느 쪽으로 갈지는 caller (file.ts) 가 결정한다 — picker 가
   * 한 번만 뜨도록 *picker 책임* 과 *저장 책임* 을 분리한 결과.
   *
   * 미설정 자리·취소·timeout 자리 null.
   */
  pickVfinderSaveAsTarget?: (suggestedName: string) => Promise<SaveAsTarget | null>;
  /**
   * vfinder save-as picker 결과 (`SaveAsTarget`) 를 server-side `/sessions/{id}/save-as`
   * 에 forward 한다. SSR 활성 자리만 의미 — *server-side 저장 흐름* 의 진입점.
   *
   * 반환 true = 성공 (URL 갱신·세션 갱신 완료). false = 세션 정보 부재 자리 (caller 가
   * vfinder 직호출 fallback 으로 흘릴 자리). throw = 서버 4xx/5xx 응답.
   */
  forwardSaveAsToServer?: (target: SaveAsTarget) => Promise<boolean>;
  /**
   * vfinder iframe 흐름으로 *파일 열기*.
   *
   * 동작: 부모창에 `rhwp:open-request` postMessage 발사 → 부모창이 vfinder picker
   * iframe (mode=picker, kind=file) 띄움 → 사용자가 파일 고르면 부모창이
   * `rhwp:open-target { fileId, name }` 으로 forward → URL `?fileId=` 갱신 후
   * 페이지 in-place 재진입 (iframe 만 reload).
   *
   * 반환 true = 사용자가 파일 골랐고 진입 트리거 발사. false = 취소·실패. 미설정이면
   * 기존 로컬 showOpenFilePicker 흐름으로 진행 (cross-origin sub frame 에서는 차단됨).
   */
  openViaVfinder?: () => Promise<boolean>;
  /**
   * agent VM 안에서 *vfinder /api/upload 직호출* 흐름의 사용자 식별자.
   *
   * iframe URL `?user=<email>` 로 부착된 값. SSR 모드 비활성에서도 cross-origin
   * iframe 환경에서 *VM 내부 vfinder upload* 인증에 그대로 박힌다 (헤더
   * `X-Vfinder-User` + 쿼리 `?user=`). 미설정 시 vfinder 서버측 기본 사용자 fallback.
   */
  vfinderUserId?: string;
  /**
   * vfinder studio base URL. 기본 `/vfinder` (같은 host path proxy).
   * agent 환경에서 vfinder 가 다른 자리에 떠 있을 때만 override.
   */
  vfinderBase?: string;
}
