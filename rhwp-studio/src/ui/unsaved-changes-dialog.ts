/**
 * 저장되지 않은 변경사항 확인 대화상자.
 *
 * 브라우저 창/탭 닫기는 beforeunload 기본 확인창만 사용할 수 있으므로,
 * 이 대화상자는 앱 내부 문서 교체 동작에서만 사용한다.
 */
import { ModalDialog } from './dialog';
import { t } from '@/i18n/t';

export type UnsavedChangesChoice = 'save' | 'discard' | 'cancel';

interface UnsavedChangesDialogOptions {
  fileName: string;
  canSave: boolean;
}

class UnsavedChangesDialog extends ModalDialog {
  private resolve!: (value: UnsavedChangesChoice) => void;

  constructor(private readonly options: UnsavedChangesDialogOptions) {
    super(t('unsaved.dialog_title'), 420);
  }

  protected createBody(): HTMLElement {
    const body = document.createElement('div');
    body.style.padding = '16px 20px';
    body.style.lineHeight = '1.6';
    body.style.whiteSpace = 'pre-line';

    const fileName = this.options.fileName || t('unsaved.fallback_filename');
    body.textContent = this.options.canSave
      ? t('unsaved.message_can_save', { fileName })
      : t('unsaved.message_cannot_save', { fileName });

    return body;
  }

  protected onConfirm(): void {
    this.resolve('save');
  }

  override hide(): void {
    this.resolve('cancel');
    super.hide();
  }

  showAsync(): Promise<UnsavedChangesChoice> {
    return new Promise((resolve) => {
      let resolved = false;
      this.resolve = (value: UnsavedChangesChoice) => {
        if (!resolved) {
          resolved = true;
          resolve(value);
        }
      };

      super.show();

      const footer = this.dialog.querySelector('.dialog-footer');
      const saveBtn = this.dialog.querySelector('.dialog-btn-primary') as HTMLButtonElement | null;
      const cancelBtn = footer?.querySelector('.dialog-btn:not(.dialog-btn-primary)') as HTMLButtonElement | null;

      if (saveBtn) {
        saveBtn.textContent = t('menu.file.save');
        saveBtn.disabled = !this.options.canSave;
        saveBtn.title = this.options.canSave ? '' : t('unsaved.hwpx_save_disabled');
      }
      if (cancelBtn) {
        cancelBtn.textContent = t('button.cancel');
      }

      const discardBtn = document.createElement('button');
      discardBtn.type = 'button';
      discardBtn.className = 'dialog-btn';
      discardBtn.textContent = t('unsaved.discard');
      discardBtn.addEventListener('click', () => {
        this.resolve('discard');
        super.hide();
      });
      footer?.insertBefore(discardBtn, cancelBtn ?? null);
    });
  }
}

export function showUnsavedChangesDialog(options: UnsavedChangesDialogOptions): Promise<UnsavedChangesChoice> {
  return new UnsavedChangesDialog(options).showAsync();
}
