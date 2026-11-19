import { parentPort, workerData } from 'node:worker_threads';
import { measureRoots, measureVhd, rootsFromBaseline } from './m1-budget-supervisor.mjs';

try {
  parentPort.postMessage({
    ok: true,
    roots: measureRoots(rootsFromBaseline(workerData.baseline)),
    vhd: measureVhd(workerData.baseline.vhd),
  });
} catch (error) {
  parentPort.postMessage({ ok: false, code: error.code ?? 'measure', message: error.message });
}
