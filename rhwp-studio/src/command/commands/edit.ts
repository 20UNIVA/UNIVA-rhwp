import type { CommandDef } from '../types';
import { FieldEditDialog } from '@/ui/field-edit-dialog';
import { FindDialog } from '@/ui/find-dialog';
import { GotoDialog } from '@/ui/goto-dialog';
import { HistoryDialog } from '@/ui/history-dialog';
import { CompareDialog } from '@/ui/compare-dialog';
import { CompareSessionStore } from '@/compare/session';
import { t } from '@/i18n/t';

/** 검색 대화상자 싱글톤 — 열려 있으면 재사용 */
let findDialogInstance: FindDialog | null = null;
/** 싱글톤: 문서 이력 관리 대화상자 */
let historyDialogInstance: HistoryDialog | null = null;
/** 싱글톤: 두 파일 문서 비교 대화상자 */
let compareDialogInstance: CompareDialog | null = null;
/** 비교/이력 공용 세션 스토어 */
let compareSessionStore: CompareSessionStore | null = null;

export const editCommands: CommandDef[] = [
  {
    id: 'edit:undo',
    label: t('cmd.edit.undo'),
    icon: 'icon-undo',
    shortcutLabel: 'Ctrl+Z',
    canExecute: (ctx) => ctx.hasDocument && ctx.canUndo,
    execute(services) {
      services.getInputHandler()?.performUndo();
    },
  },
  {
    id: 'edit:redo',
    label: t('cmd.edit.redo'),
    icon: 'icon-redo',
    shortcutLabel: 'Ctrl+Shift+Z',
    canExecute: (ctx) => ctx.hasDocument && ctx.canRedo,
    execute(services) {
      services.getInputHandler()?.performRedo();
    },
  },
  {
    id: 'edit:cut',
    label: t('cmd.edit.cut'),
    icon: 'icon-cut',
    shortcutLabel: 'Ctrl+X',
    canExecute: (ctx) => ctx.hasDocument && (ctx.hasSelection || ctx.inPictureObjectSelection || ctx.inTableObjectSelection),
    execute(services) {
      services.getInputHandler()?.performCut();
    },
  },
  {
    id: 'edit:copy',
    label: t('cmd.edit.copy'),
    icon: 'icon-copy',
    shortcutLabel: 'Ctrl+C',
    canExecute: (ctx) => ctx.hasDocument && (ctx.hasSelection || ctx.inPictureObjectSelection || ctx.inTableObjectSelection),
    execute(services) {
      services.getInputHandler()?.performCopy();
    },
  },
  {
    id: 'edit:paste',
    label: t('cmd.edit.paste'),
    icon: 'icon-paste',
    shortcutLabel: 'Ctrl+V',
    canExecute: (ctx) => ctx.hasDocument,
    execute(services) {
      services.getInputHandler()?.performPaste();
    },
  },
  {
    id: 'edit:format-copy',
    label: t('cmd.edit.format_copy'),
    icon: 'icon-format-copy',
    shortcutLabel: 'Ctrl+Alt+C',
    canExecute: () => false, // 미구현
    execute() { /* TODO */ },
  },
  {
    id: 'edit:delete',
    label: t('cmd.edit.delete'),
    icon: 'icon-delete',
    shortcutLabel: 'Ctrl+E',
    canExecute: (ctx) => ctx.hasDocument && (ctx.hasSelection || ctx.inPictureObjectSelection || ctx.inTableObjectSelection),
    execute(services) {
      services.getInputHandler()?.performDelete();
    },
  },
  {
    id: 'edit:select-all',
    label: t('cmd.edit.select_all'),
    icon: 'icon-select-all',
    shortcutLabel: 'Ctrl+A',
    canExecute: (ctx) => ctx.hasDocument,
    execute(services) {
      services.getInputHandler()?.performSelectAll();
    },
  },
  {
    id: 'edit:find',
    label: t('cmd.edit.find'),
    icon: 'icon-find',
    shortcutLabel: 'Ctrl+F',
    canExecute: (ctx) => ctx.hasDocument,
    execute(services) {
      if (findDialogInstance && findDialogInstance.isOpen()) {
        findDialogInstance.focusInput();
        return;
      }
      findDialogInstance = new FindDialog(services, 'find');
      findDialogInstance.show();
    },
  },
  {
    id: 'edit:find-replace',
    label: t('cmd.edit.find_replace'),
    icon: 'icon-find-replace',
    shortcutLabel: 'Ctrl+F2',
    canExecute: (ctx) => ctx.hasDocument,
    execute(services) {
      if (findDialogInstance && findDialogInstance.isOpen()) {
        findDialogInstance.switchMode('replace');
        findDialogInstance.focusInput();
        return;
      }
      findDialogInstance = new FindDialog(services, 'replace');
      findDialogInstance.show();
    },
  },
  {
    id: 'edit:find-again',
    label: t('cmd.edit.find_again'),
    shortcutLabel: 'Ctrl+L',
    canExecute: (ctx) => ctx.hasDocument,
    execute(services) {
      if (findDialogInstance && findDialogInstance.isOpen()) {
        findDialogInstance.findNext();
      } else if (FindDialog.lastQuery) {
        // 대화상자 없이 WASM 직접 검색
        const ih = services.getInputHandler();
        if (!ih) return;
        const pos = ih.getCursorPosition();
        const result = services.wasm.searchText(
          FindDialog.lastQuery, pos.sectionIndex, pos.paragraphIndex,
          pos.charOffset, true, FindDialog.lastCaseSensitive,
        );
        if (result.found) {
          ih.moveCursorTo({
            sectionIndex: result.sec!,
            paragraphIndex: result.para!,
            charOffset: result.charOffset!,
          });
          const cursor = (ih as any).cursor;
          if (cursor) {
            cursor.setAnchor();
            cursor.moveTo({
              sectionIndex: result.sec!,
              paragraphIndex: result.para!,
              charOffset: result.charOffset! + result.length!,
            });
          }
          (ih as any).updateCaret?.();
        }
      }
    },
  },
  {
    id: 'edit:compare-documents',
    label: t('cmd.edit.compare_documents'),
    shortcutLabel: 'Alt+Shift+V',
    canExecute: () => true,
    execute(services) {
      if (!compareSessionStore) {
        compareSessionStore = new CompareSessionStore(services.eventBus);
      }
      if (historyDialogInstance?.isOpen()) historyDialogInstance.hide();
      if (compareDialogInstance && compareDialogInstance.isOpen()) return;
      compareDialogInstance = new CompareDialog(services, compareSessionStore);
      compareDialogInstance.show();
    },
  },
  {
    id: 'edit:document-history',
    label: t('cmd.edit.document_history'),
    shortcutLabel: 'Ctrl+Shift+H',
    canExecute: () => true,
    execute(services) {
      if (!compareSessionStore) {
        compareSessionStore = new CompareSessionStore(services.eventBus);
      }
      if (compareDialogInstance?.isOpen()) compareDialogInstance.hide();
      if (historyDialogInstance && historyDialogInstance.isOpen()) {
        return;
      }
      historyDialogInstance = new HistoryDialog(services, compareSessionStore);
      historyDialogInstance.show();
    },
  },
  {
    id: 'edit:goto',
    label: t('cmd.edit.goto'),
    shortcutLabel: 'Alt+G',
    canExecute: (ctx) => ctx.hasDocument,
    execute(services) {
      const dialog = new GotoDialog(services);
      dialog.show();
    },
  },
  {
    id: 'field:edit',
    label: t('cmd.edit.field_edit'),
    shortcutLabel: 'Ctrl+M,K',
    canExecute: (ctx) => ctx.hasDocument && ctx.inField,
    execute(services) {
      const ih = services.getInputHandler();
      if (!ih) return;
      const fi = (ih as any).getFieldInfo?.();
      console.log('[field:edit] fieldInfo:', fi);
      if (!fi || fi.fieldId == null) return;
      const props = services.wasm.getClickHereProps(fi.fieldId);
      console.log('[field:edit] props:', props);
      if (!props.ok) return;

      const dialog = new FieldEditDialog();
      dialog.onApply = (newProps) => {
        console.log('[field:edit] apply:', newProps);
        const result = services.wasm.updateClickHereProps(
          fi.fieldId, newProps.guide, newProps.memo, newProps.name, newProps.editable,
        );
        console.log('[field:edit] updateResult:', result);
        if (result.ok) {
          services.eventBus.emit('document-changed');
        }
      };
      dialog.showWith({
        guide: props.guide ?? '',
        memo: props.memo ?? '',
        name: props.name ?? '',
        editable: props.editable ?? true,
      });
    },
  },
  {
    id: 'field:remove',
    label: t('cmd.edit.field_remove'),
    canExecute: (ctx) => ctx.hasDocument && ctx.inField,
    execute(services) {
      const ih = services.getInputHandler();
      if (ih) (ih as any).removeCurrentField();
    },
  },
];
