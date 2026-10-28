import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
export default defineConfig({plugins:[react()],define:{__APP_VERSION__:JSON.stringify('0.0.0-test')},test:{testTimeout:30000,hookTimeout:30000,fileParallelism:false,maxWorkers:1,environment:'jsdom',include:['tests/**/*.test.ts','tests/**/*.test.tsx'],setupFiles:['./tests/setup.ts']}});
