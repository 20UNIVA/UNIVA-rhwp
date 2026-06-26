/**
 * 스타일 대화상자 (StyleDialog)
 * 한컴 F6 대화상자를 참고한 스타일 목록 + 속성 미리보기 + 편집 기능
 *
 *  ┌───────────────────────────────────────────────────┐
 *  │ 스타일                                        [×] │
 *  ├───────────────────────────────────────────────────┤
 *  │ 스타일 목록(M):        │ 문단 모양 정보          │
 *  │ ┌──────────────────┐  │  왼쪽 여백: 0 pt        │
 *  │ │ 바탕글    Ctrl+1 │  │  오른쪽 여백: 0 pt      │
 *  │ │ 본문      Ctrl+2 │  │  줄 간격: 160%          │
 *  │ │ 개요 1    Ctrl+3 │  │  정렬 방식: 양쪽        │
 *  │ │ ...              │  │                         │
 *  │ └──────────────────┘  │ 글자 모양 정보          │
 *  │ [+][✎][−]             │  글꼴: 함초롬바탕       │
 *  │                       │  크기: 10 pt             │
 *  │ 현재 커서 위치 스타일  │  장평: 100% 자간: 0%    │
 *  │ 스타일: 바탕글         │                         │
 *  ├───────────────────────┴─────────────────────────┤
 *  │                            [설정(D)]  [취소]     │
 *  └─────────────────────────────────────────────────┘
 */

import type { WasmBridge } from '@/core/wasm-bridge';
import type { EventBus } from '@/core/event-bus';
import type { CharProperties, ParaProperties } from '@/core/types';
import { ModalDialog } from './dialog';
import { t } from '@/i18n/t';

interface StyleEntry {
  id: number;
  name: string;
  englishName: string;
  type: number;
  nextStyleId: number;
  paraShapeId: number;
  charShapeId: number;
}

function alignLabel(key: string | undefined): string {
  // 짧은 정렬 라벨 — 사전 키 style.align_short.*
  const k = key ?? 'justify';
  switch (k) {
    case 'justify': return t('style.align_short.justify');
    case 'left': return t('style.align_short.left');
    case 'right': return t('style.align_short.right');
    case 'center': return t('style.align_short.center');
    case 'distribute': return t('style.align_short.distribute');
    case 'split': return t('style.align_short.split');
    default: return key ?? '';
  }
}

export class StyleDialog extends ModalDialog {
  private styleList!: HTMLDivElement;
  private infoPanel!: HTMLDivElement;
  private currentStyleLabel!: HTMLSpanElement;
  private selectedId = 0;
  private styles: StyleEntry[] = [];

  /** 설정(적용) 콜백 */
  onApply?: (styleId: number) => void;
  onClose?: () => void;
  /** 편집 후 다이얼로그 새로고침 콜백 */
  onEditRequest?: (styleId: number) => void;
  onAddRequest?: () => void;

  constructor(
    private wasm: WasmBridge,
    private eventBus: EventBus,
  ) {
    super(t('menu.format.style'), 560);
  }

  protected createBody(): HTMLElement {
    const body = document.createElement('div');
    body.className = 'sd-body';

    // ── 좌측: 스타일 목록 ──
    const leftCol = document.createElement('div');
    leftCol.className = 'sd-left';

    const listLabel = document.createElement('div');
    listLabel.className = 'sd-list-label';
    listLabel.textContent = t('style.list_label_m');
    leftCol.appendChild(listLabel);

    this.styleList = document.createElement('div');
    this.styleList.className = 'sd-style-list';
    leftCol.appendChild(this.styleList);

    // 아이콘 버튼 바
    const iconBar = document.createElement('div');
    iconBar.className = 'sd-icon-bar';

    const btnAdd = this.createIconBtn('+', t('style.btn.add'), () => {
      this.onAddRequest?.();
    });
    const btnEdit = this.createIconBtn('✎', t('style.btn.edit'), () => {
      this.onEditRequest?.(this.selectedId);
    });
    const btnDelete = this.createIconBtn('−', t('style.btn.delete'), () => {
      this.handleDelete();
    });
    iconBar.appendChild(btnAdd);
    iconBar.appendChild(btnEdit);
    iconBar.appendChild(btnDelete);
    leftCol.appendChild(iconBar);

    // 현재 커서 위치 스타일
    const curInfo = document.createElement('div');
    curInfo.className = 'sd-cur-style';
    const curLabel = document.createElement('span');
    curLabel.textContent = t('style.current_cursor_style');
    this.currentStyleLabel = document.createElement('span');
    this.currentStyleLabel.className = 'sd-cur-style-name';
    curInfo.appendChild(curLabel);
    curInfo.appendChild(this.currentStyleLabel);
    leftCol.appendChild(curInfo);

    body.appendChild(leftCol);

    // ── 우측: 속성 정보 ──
    this.infoPanel = document.createElement('div');
    this.infoPanel.className = 'sd-info-panel';
    body.appendChild(this.infoPanel);

    return body;
  }

  private createIconBtn(text: string, title: string, handler: () => void): HTMLButtonElement {
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'sd-icon-btn';
    btn.textContent = text;
    btn.title = title;
    btn.addEventListener('click', handler);
    return btn;
  }

  private loadStyles(): void {
    try {
      this.styles = this.wasm.getStyleList();
    } catch {
      this.styles = [];
    }
    this.renderList();
  }

  private renderList(): void {
    this.styleList.replaceChildren();
    for (const s of this.styles) {
      const item = document.createElement('div');
      item.className = 'sd-style-item' + (s.id === this.selectedId ? ' sd-selected' : '');

      const typeIcon = document.createElement('span');
      typeIcon.className = 'sd-type-icon';
      typeIcon.textContent = s.type === 0 ? '¶' : 'A';
      typeIcon.title = s.type === 0 ? t('style.kind.paragraph') : t('style.kind.character');

      const name = document.createElement('span');
      name.className = 'sd-style-name';
      name.textContent = s.name;

      item.appendChild(typeIcon);
      item.appendChild(name);
      item.addEventListener('click', () => {
        this.selectedId = s.id;
        this.renderList();
        this.updateInfo();
      });
      item.addEventListener('dblclick', () => {
        this.onEditRequest?.(s.id);
      });
      this.styleList.appendChild(item);
    }
  }

  private updateInfo(): void {
    this.infoPanel.replaceChildren();
    try {
      const detail = this.wasm.getStyleDetail(this.selectedId);
      const style = this.styles.find(s => s.id === this.selectedId);

      // 문단 모양 정보
      if (style?.type === 0) {
        this.addInfoSection(t('style.section.para_info'), this.buildParaInfo(detail.paraProps));
      }

      // 글자 모양 정보
      this.addInfoSection(t('style.section.char_info'), this.buildCharInfo(detail.charProps));

      // 번호/글머리표 정보
      const headType = detail.paraProps?.headType ?? 'None';
      const headLabel: Record<string, string> = {
        None: t('char_shape.misc.none'),
        Outline: t('style.head.outline'),
        Number: t('style.head.number'),
        Bullet: t('style.head.bullet'),
      };
      this.addInfoSection(t('style.section.numbering_info'), t('style.section.kind_with_value', { value: headLabel[headType] ?? headType }));
    } catch {
      this.infoPanel.textContent = t('style.props_query_failed');
    }
  }

  private addInfoSection(title: string, content: string): void {
    const sec = document.createElement('div');
    sec.className = 'sd-info-section';

    const h = document.createElement('div');
    h.className = 'sd-info-title';
    h.textContent = title;
    sec.appendChild(h);

    const p = document.createElement('div');
    p.className = 'sd-info-content';
    const lines = content.split('<br>');
    lines.forEach((line, index) => {
      if (index > 0) p.appendChild(document.createElement('br'));
      p.appendChild(document.createTextNode(line.split('&nbsp;').join('\u00A0')));
    });
    sec.appendChild(p);

    this.infoPanel.appendChild(sec);
  }

  private buildParaInfo(pp: ParaProperties): string {
    const pxToPt = (v: number) => (v * 72 / 96).toFixed(1);
    const align = alignLabel(pp.alignment);
    const lsType = pp.lineSpacingType ?? 'Percent';
    const ls = pp.lineSpacing ?? 160;
    const ml = pp.marginLeft != null ? pxToPt(pp.marginLeft) : '0.0';
    const mr = pp.marginRight != null ? pxToPt(pp.marginRight) : '0.0';
    const indent = pp.indent != null ? pxToPt(pp.indent) : '0.0';
    const indentPt = parseFloat(indent);
    const firstLine = indentPt > 0
      ? t('style.indent.indent_pt', { v: indent })
      : indentPt < 0
        ? t('style.indent.hanging_pt', { v: Math.abs(indentPt).toFixed(1) })
        : t('style.indent.normal');
    // 사전 단일 키 부재 — line-spacing 단위 인라인 (lsType 비사용 변수 정리)
    void lsType;
    const lsStr = (pp.lineSpacingType ?? 'Percent') === 'Percent'
      ? `${ls} %`
      : `${pxToPt(ls)} pt`;

    // 다음 스타일
    const style = this.styles.find(s => s.id === this.selectedId);
    const nextStyle = style ? this.styles.find(s => s.id === style.nextStyleId) : null;
    const nextStyleName = nextStyle?.name ?? style?.name ?? '';

    return [
      t('style.para_info.line1', { ml, firstLine }),
      t('style.para_info.line2', { mr, align }),
      t('style.para_info.line3_linespacing', { ls: lsStr }),
      t('style.para_info.line4_next_style', { name: nextStyleName }),
    ].join('<br>');
  }

  private buildCharInfo(cp: CharProperties): string {
    const font = cp.fontFamily ?? 'sans-serif';
    const size = cp.fontSize != null ? (cp.fontSize / 100).toFixed(0) : '10';
    const ratio = cp.ratios?.[0] ?? 100;
    const spacing = cp.spacings?.[0] ?? 0;
    return [
      t('style.char_info.font', { font }),
      t('style.char_info.size_pt', { size }),
      t('style.char_info.ratio_spacing', { ratio, spacing }),
    ].join('<br>');
  }

  private handleDelete(): void {
    if (this.selectedId === 0) {
      alert(t('error.client.basic_style_undeletable'));
      return;
    }
    const style = this.styles.find(s => s.id === this.selectedId);
    if (!style) return;
    if (!confirm(t('style.confirm.delete', { name: style.name }))) return;
    try {
      this.wasm.deleteStyle(this.selectedId);
      this.selectedId = 0;
      this.loadStyles();
      this.updateInfo();
    } catch (err) {
      console.warn('[StyleDialog] 삭제 실패:', err);
    }
  }

  /** 외부에서 스타일 목록 새로고침 (편집/추가 후) */
  refresh(): void {
    this.loadStyles();
    this.updateInfo();
  }

  protected onConfirm(): void {
    this.onApply?.(this.selectedId);
  }

  override hide(): void {
    super.hide();
    this.onClose?.();
  }

  override show(): void {
    super.show();
    this.loadStyles();
    this.updateInfo();
    // 현재 커서 위치 스타일 표시
    try {
      // eventBus로부터 현재 스타일 정보 가져오기
      this.currentStyleLabel.textContent = this.styles.find(s => s.id === this.selectedId)?.name ?? '';
    } catch {
      // 무시
    }
  }

  /** 현재 커서 위치 스타일 ID를 설정하고 뷰를 갱신 */
  setCurrentStyleId(styleId: number): void {
    this.selectedId = styleId;
    const style = this.styles.find(s => s.id === styleId);
    if (this.currentStyleLabel) {
      this.currentStyleLabel.textContent = style?.name ?? '';
    }
    // 목록 선택 + 정보 패널 갱신
    this.renderList();
    this.updateInfo();
  }
}
