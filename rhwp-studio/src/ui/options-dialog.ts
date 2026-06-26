/**
 * 환경 설정 대화상자 (도구 > 환경 설정)
 *
 * 탭 구조: [글꼴] (향후 [편집], [보기] 등 탭 추가 가능)
 */
import { ModalDialog } from './dialog';
import { userSettings } from '@/core/user-settings';
import { FontSetDialog } from './font-set-dialog';
import { isLocalFontSupported, detectLocalFonts, getLocalFonts } from '@/core/local-fonts';
import { t } from '@/i18n/t';

export class OptionsDialog extends ModalDialog {
  private showRecentCheck!: HTMLInputElement;
  private recentCountInput!: HTMLInputElement;

  constructor() {
    super(t('prefs.dialog_title'), 480);
  }

  protected createBody(): HTMLElement {
    const body = document.createElement('div');
    body.className = 'opt-body';

    // 탭 헤더
    const tabs = document.createElement('div');
    tabs.className = 'dialog-tabs';

    const fontTab = document.createElement('button');
    fontTab.className = 'dialog-tab active';
    fontTab.textContent = t('prefs.font');
    fontTab.dataset.tab = 'font';
    tabs.appendChild(fontTab);

    body.appendChild(tabs);

    // 글꼴 탭 패널
    const fontPanel = this.createFontPanel();
    fontPanel.className = 'dialog-tab-panel opt-tab-panel active';
    fontPanel.dataset.tab = 'font';
    body.appendChild(fontPanel);

    // 탭 클릭 이벤트 (향후 탭 추가 대비)
    tabs.addEventListener('click', (e) => {
      const btn = (e.target as HTMLElement).closest('.dialog-tab') as HTMLElement | null;
      if (!btn) return;
      const tabId = btn.dataset.tab;
      tabs.querySelectorAll('.dialog-tab').forEach(t => t.classList.remove('active'));
      body.querySelectorAll('.dialog-tab-panel').forEach(p => p.classList.remove('active'));
      btn.classList.add('active');
      const panel = body.querySelector(`.dialog-tab-panel[data-tab="${tabId}"]`);
      panel?.classList.add('active');
    });

    return body;
  }

  private createFontPanel(): HTMLElement {
    const panel = document.createElement('div');
    const fs = userSettings.getFontSettings();

    // ── 글꼴 보기 섹션 ──
    const viewSection = document.createElement('div');
    viewSection.className = 'dialog-section';

    const viewTitle = document.createElement('div');
    viewTitle.className = 'dialog-section-title';
    viewTitle.textContent = t('prefs.font_preview');
    viewSection.appendChild(viewTitle);

    // 최근 사용 글꼴 보이기
    const recentRow = document.createElement('div');
    recentRow.className = 'dialog-row opt-row';

    this.showRecentCheck = document.createElement('input');
    this.showRecentCheck.type = 'checkbox';
    this.showRecentCheck.id = 'opt-show-recent';
    this.showRecentCheck.checked = fs.showRecentFonts;

    const recentLabel = document.createElement('label');
    recentLabel.htmlFor = 'opt-show-recent';
    recentLabel.textContent = t('prefs.show_recent_fonts');

    this.recentCountInput = document.createElement('input');
    this.recentCountInput.type = 'number';
    this.recentCountInput.className = 'dialog-input opt-count-input';
    this.recentCountInput.min = '1';
    this.recentCountInput.max = '5';
    this.recentCountInput.value = String(fs.recentFontCount);

    const countLabel = document.createElement('span');
    countLabel.className = 'opt-count-label';
    countLabel.textContent = t('prefs.count_unit');

    recentRow.appendChild(this.showRecentCheck);
    recentRow.appendChild(recentLabel);
    recentRow.appendChild(this.recentCountInput);
    recentRow.appendChild(countLabel);
    viewSection.appendChild(recentRow);

    panel.appendChild(viewSection);

    // ── 대표 글꼴 등록 섹션 ──
    const fontSetSection = document.createElement('div');
    fontSetSection.className = 'dialog-section';

    const fontSetTitle = document.createElement('div');
    fontSetTitle.className = 'dialog-section-title';
    fontSetTitle.textContent = t('prefs.representative_font_group');
    fontSetSection.appendChild(fontSetTitle);

    const fontSetDesc = document.createElement('p');
    fontSetDesc.className = 'opt-desc';
    fontSetDesc.textContent = t('prefs.representative_font_desc');
    fontSetSection.appendChild(fontSetDesc);

    const fontSetBtn = document.createElement('button');
    fontSetBtn.className = 'dialog-btn opt-fontset-btn';
    fontSetBtn.textContent = t('prefs.register_representative');
    fontSetBtn.addEventListener('click', () => {
      const dlg = new FontSetDialog();
      dlg.show();
    });
    fontSetSection.appendChild(fontSetBtn);

    panel.appendChild(fontSetSection);

    // ── 로컬 글꼴 섹션 ──
    const localSection = document.createElement('div');
    localSection.className = 'dialog-section';

    const localTitle = document.createElement('div');
    localTitle.className = 'dialog-section-title';
    localTitle.textContent = t('prefs.local_fonts_group');
    localSection.appendChild(localTitle);

    const localDesc = document.createElement('p');
    localDesc.className = 'opt-desc';
    localDesc.textContent = t('prefs.local_fonts_desc');
    localSection.appendChild(localDesc);

    const localRow = document.createElement('div');
    localRow.className = 'dialog-row opt-row';

    const localBtn = document.createElement('button');
    localBtn.className = 'dialog-btn opt-fontset-btn';
    localBtn.textContent = t('prefs.detect_local_fonts');

    const localStatus = document.createElement('span');
    localStatus.className = 'opt-local-status';

    // 이미 감지된 글꼴이 있으면 상태 표시
    const cached = getLocalFonts();
    if (cached.length > 0) {
      localStatus.textContent = t('prefs.local_fonts_detected', { count: cached.length });
    }

    localBtn.addEventListener('click', async () => {
      if (!isLocalFontSupported()) {
        localStatus.textContent = t('prefs.browser_unsupported');
        return;
      }
      localBtn.disabled = true;
      localStatus.textContent = t('prefs.detecting');
      try {
        const fonts = await detectLocalFonts();
        localStatus.textContent = `${fonts.length}개 로컬 글꼴 감지됨`;
      } catch {
        localStatus.textContent = t('prefs.detect_failed');
      }
      localBtn.disabled = false;
    });

    localRow.appendChild(localBtn);
    localRow.appendChild(localStatus);
    localSection.appendChild(localRow);

    panel.appendChild(localSection);

    return panel;
  }

  protected onConfirm(): void {
    const count = Math.min(5, Math.max(1, parseInt(this.recentCountInput.value) || 3));
    userSettings.updateFontSettings({
      showRecentFonts: this.showRecentCheck.checked,
      recentFontCount: count,
    });
  }
}
