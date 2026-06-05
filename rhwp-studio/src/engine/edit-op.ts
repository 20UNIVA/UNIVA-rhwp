/**
 * SSR 세션용 양방향 편집 연산(EditOperation) 프로토콜 — TypeScript 측 정의.
 *
 * Rust `src/document_core/commands/edit_op.rs` 의 `EditOperation` enum 과
 * **동일한 JSON 스키마**를 가져야 한다(`op` 태그 + snake_case 필드).
 * 클라이언트는 편집 후 `EditCommand.serialize()` 로 이 연산을 만들어 서버에 전송하고,
 * 서버는 native `DocumentCore::apply_edit_ops_json` 으로 동일하게 재현한다.
 *
 * 위치 인덱스는 모두 0-based, char 오프셋은 코드포인트 기준(BMP 문자 한정 정확).
 */
export type EditOperation =
  | { op: 'insert_text'; section: number; para: number; offset: number; text: string }
  | {
      op: 'delete_text';
      section: number;
      para: number;
      offset: number;
      count: number;
      deleted_text: string;
    }
  | { op: 'split_paragraph'; section: number; para: number; offset: number }
  | { op: 'merge_paragraph'; section: number; para: number; prev_len: number };
