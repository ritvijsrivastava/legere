import * as api from '../api';
import type { CaptureJob } from '../types';

/** Every in-flight or failed "Add a source" background capture (see
 *  `capture_jobs` on the backend and `AddSourceDialog`, which queues a
 *  job and closes immediately rather than blocking on it). Driven by the
 *  `capture:changed` event like every other `*Store` in this app is
 *  driven by its own `*:changed` event — a coarse refetch of this small
 *  list, not a payload-carried delta. */
class CaptureJobsStore {
	jobs = $state<CaptureJob[]>([]);
	loaded = $state(false);

	get runningCount(): number {
		return this.jobs.filter((j) => j.status.state === 'running').length;
	}

	get failedCount(): number {
		return this.jobs.filter((j) => j.status.state === 'failed').length;
	}

	async refresh() {
		try {
			this.jobs = await api.listCaptureJobs();
		} finally {
			this.loaded = true;
		}
	}

	async retry(id: string) {
		await api.retryCaptureJob(id);
		await this.refresh();
	}

	async cancel(id: string) {
		await api.cancelCaptureJob(id);
		await this.refresh();
	}

	async dismiss(id: string) {
		await api.dismissCaptureJob(id);
		await this.refresh();
	}
}

export const captureJobsStore = new CaptureJobsStore();
