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
   * vfinder iframe 흐름으로 *다른 이름으로 저장*.
   *
   * 동작: 부모창에 `rhwp:save-as-request` postMessage 발사 → 부모창이 vfinder save-as
   * iframe 띄움 → 사용자가 폴더·이름 고르면 부모창이 `rhwp:save-as-target` 으로 forward
   * → 그 인자로 `POST /sessions/:id/save-as` 호출 → 응답의 새 fileId 로 URL 갱신 +
   * 부모창에 `rhwp:saved-as` 발사.
   *
   * 반환 true = 성공 (사용자가 저장 완료). false = 취소·실패. 미설정이면 기존 로컬
   * file system access 흐름으로 진행.
   */
  saveAsViaVfinder?: () => Promise<boolean>;
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
}
