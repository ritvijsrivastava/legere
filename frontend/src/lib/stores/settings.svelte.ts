import * as api from '../api';
import type { Settings } from '../types';

const DEFAULT_SETTINGS: Settings = {
	default_font_size: 'medium',
	default_library_view: 'cards',
	autosync: true,
	reader_font_size: 19,
	reader_measure: 'default',
	reader_leading: 'default',
	app_theme: 'dark',
	reader_theme: 'light'
};

class SettingsStore {
	current = $state<Settings>({ ...DEFAULT_SETTINGS });
	loaded = $state(false);

	async refresh() {
		this.current = await api.getSettings();
		this.loaded = true;
	}

	async update(partial: Partial<Settings>) {
		this.current = { ...this.current, ...partial };
		await api.updateSettings(this.current);
	}
}

export const settingsStore = new SettingsStore();
