import type { CommandDef } from '../types';
import { OptionsDialog } from '../../ui/options-dialog';
import { t } from '@/i18n/t';

export const toolCommands: CommandDef[] = [
  {
    id: 'tool:options',
    label: t('cmd.tool.options'),
    execute(_services) {
      const dlg = new OptionsDialog();
      dlg.show();
    },
  },
];
